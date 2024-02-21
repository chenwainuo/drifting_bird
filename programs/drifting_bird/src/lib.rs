mod oracle_state;

use std::ops::Deref;
use std::str::FromStr;
use anchor_lang::prelude::*;
use drift::state::user::User;
use borsh::{BorshSerialize, BorshDeserialize};
use drift::state::oracle::OraclePriceData;
use phoenix::program::{
    new_order::{CondensedOrder, MultipleOrderPacket},
    CancelMultipleOrdersByIdParams, CancelOrderParams, MarketHeader,
};
use phoenix::{
    quantities::WrapperU64,
    state::{
        markets::{FIFOOrderId, FIFORestingOrder, Market},
        OrderPacket, Side,
    },
};
use pyth_sdk_solana::state::load_price_account;
use crate::oracle_state::PriceFeed;
use anchor_spl::token::{self, Burn, Mint, MintTo, Token, TokenAccount, Transfer};

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

pub const PHOENIX_MARKET_DISCRIMINANT: u64 = 8167313896524341111;


fn load_header(info: &AccountInfo) -> Result<MarketHeader> {
    require!(
        info.owner == &phoenix::id(),
        StrategyError::InvalidPhoenixProgram
    );
    let data = info.data.borrow();
    let header =
        bytemuck::try_from_bytes::<MarketHeader>(&data[..std::mem::size_of::<MarketHeader>()])
            .map_err(|_| {
                msg!("Failed to parse Phoenix market header");
                StrategyError::FailedToDeserializePhoenixMarket
            })?;
    require!(
        header.discriminant == PHOENIX_MARKET_DISCRIMINANT,
        StrategyError::InvalidPhoenixProgram,
    );
    Ok(*header)
}
fn get_best_bid_and_ask(
    market: &dyn Market<Pubkey, FIFOOrderId, FIFORestingOrder, OrderPacket>,
    trade_size: u64
) -> (u64,u64,u64, u64) {
    let mut total_size_bid = 0u64;
    let mut total_value_bid = 0u64;
    let mut total_size_ask = 0u64;
    let mut total_value_ask = 0u64;
    let t = trade_size;

    for (order, size) in market.get_book(Side::Bid).iter() {
        let price = order.price_in_ticks.as_u64();
        let size = size.num_base_lots.as_u64();
        if total_size_bid.clone() + size > t {
            let remaining_size = t.checked_sub(total_size_bid.clone()).unwrap();
            total_value_bid += price * remaining_size;
            total_size_bid = t.clone();
            break;
        } else {
            total_size_bid += size.clone();
            total_value_bid += price * size.clone();
        }
    }
    let average_price_bid = if total_size_bid >= t { (total_value_bid.checked_div(total_size_bid).unwrap()) } else { 0 };


    for (order, size) in market.get_book(Side::Ask).iter() {
        let price = order.price_in_ticks.as_u64();
        let size = size.num_base_lots.as_u64();
        if total_size_bid.clone() + size > t {
            let remaining_size = t.checked_sub(total_size_ask.clone()).unwrap();
            total_value_ask += price * remaining_size;
            total_size_ask = t.clone();
            break;
        } else {
            total_size_ask += size.clone();
            total_value_ask += price * size.clone();
        }
    }
    let average_price_ask = if total_size_ask >= t { (total_value_ask.checked_div(total_size_ask).unwrap()) } else { 0 };

    (average_price_bid, total_size_bid.clone(), average_price_ask, total_size_ask.clone())

}

#[program]
pub mod drifting_bird {
    use drift::controller::position::PositionDirection;
    use drift::load;
    use drift::state::user::OrderType;
    use drift::state::user::OrderType::{Limit, Oracle};
    use num_traits::{CheckedAdd, CheckedDiv, CheckedSub};
    use phoenix::program::MarketHeader;
    use phoenix::quantities::WrapperU64;
    use phoenix::state::markets::{FIFOOrderId, FIFORestingOrder, Market};
    use phoenix::state::{OrderPacket, Side};
    use super::*;


    pub fn initialize(ctx: Context<ReadOrder>, t: u64, is_buy_bird: bool, trade_size: u64) -> Result<()> {
        let mut best_bid_drift = 0 as u64;
        let mut best_bid_size_drift = 0 as u64;
        let mut best_ask_drift = 0 as u64;
        let mut best_ask_size_drift = 0 as u64;

        for order in ctx.accounts.user.load().unwrap().orders.iter() {
            if order.market_index != 0 || !order.post_only.clone() {
                continue
            }
            let price = order.price.checked_div(1000).unwrap();
            let oracle_price = ctx.accounts.oracle.get_price_unchecked().price.checked_div((100 as i64)).unwrap();
            let size = order.base_asset_amount.checked_div(1000000).unwrap();

            if order.order_type == Limit {
                if order.direction == PositionDirection::Long {
                    if order.oracle_price_offset == 0 {
                        if best_bid_size_drift == 0 {
                            best_bid_size_drift = size;
                            best_bid_drift = price;
                        } else if price > best_bid_drift {
                            best_bid_size_drift = size;
                            best_bid_drift = price;
                        }
                    } else {
                        let order_price = (oracle_price.checked_add((order.oracle_price_offset as i64)).unwrap() as u64).checked_div(1000).unwrap();
                        if best_bid_size_drift == 0 {
                            best_bid_size_drift = size;
                            best_bid_drift = order_price;
                        } else if order_price > best_bid_drift {
                            best_bid_size_drift = size;
                            best_bid_drift = order_price;
                        }
                    }
                } else if order.direction == PositionDirection::Short {
                    if order.oracle_price_offset == 0 {
                        if best_ask_size_drift == 0 {
                            best_ask_size_drift = size;
                            best_ask_drift = price.clone();
                        } else if price < best_ask_drift {
                            best_ask_size_drift = size;
                            best_ask_drift = price.clone();
                        }
                    } else {
                        let order_price = (oracle_price.checked_add((order.oracle_price_offset as i64)).unwrap() as u64).checked_div(1000).unwrap();
                        if best_ask_size_drift == 0 {
                            best_ask_size_drift = size;
                            best_ask_drift = order_price as u64;
                        } else if order_price < best_ask_drift {
                            best_ask_size_drift = size;
                            best_ask_drift = order_price as u64;
                        }
                    }
                }
            }
        }
        msg!("{} {} {} {}", best_bid_drift, best_bid_size_drift, best_ask_drift, best_ask_size_drift);

        let market_account = &ctx.accounts.bird_market;
        let header = load_header(market_account)?;
        let market_data = market_account.data.borrow();
        let (_, market_bytes) = market_data.split_at(std::mem::size_of::<MarketHeader>());
        let market = phoenix::program::load_with_dispatch(&header.market_size_params, market_bytes)
            .map_err(|_| {
                StrategyError::FailedToDeserializePhoenixMarket
            })?
            .inner;

        let (best_bid, best_bid_size, best_ask, best_ask_size) = get_best_bid_and_ask(market, trade_size);

        msg!("{} {} {} {}", best_bid,best_bid_size, best_ask, best_ask_size);

        if best_bid > best_ask_drift && !is_buy_bird.clone() {
            let spread = (best_bid.checked_mul(100000).unwrap().checked_sub(best_ask_drift.checked_mul(100000).unwrap())).unwrap().checked_div(best_ask_drift).unwrap();
            if spread < t {
                panic!()
            }
            if best_ask_size_drift < trade_size {
                panic!()
            }
            return Ok(());
        } else if best_bid_drift > best_ask && is_buy_bird.clone() {
            let spread = (best_bid_drift.checked_mul(100000).unwrap().checked_sub(best_ask.checked_mul(100000).unwrap())).unwrap().checked_div(best_ask).unwrap();
            if spread < t {
                panic!()
            }
            if best_bid_size_drift < trade_size {
                panic!()
            }
            return Ok(());
        }
        panic!()
    }
}

#[derive(Accounts)]
pub struct ReadOrder<'info> {
    #[account(mut)]
    pub user: AccountLoader<'info, User>,
    /// CHECK: Checked in instruction and CPI
    pub bird_market: UncheckedAccount<'info>,
    pub oracle: Account<'info, PriceFeed>,
}

// An enum for custom error codes
#[error_code]
pub enum StrategyError {
    NoReturnData,
    InvalidStrategyParams,
    EdgeMustBeNonZero,
    InvalidPhoenixProgram,
    FailedToDeserializePhoenixMarket,
}
