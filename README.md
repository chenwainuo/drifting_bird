## Disclaimer

**Important:** 

This software is provided for testing and educational purposes only. Utilizing this software as provided may result in financial loss. The creator(s) of this software bear no responsibility for any financial or other damages incurred.

This repo also only included Rust smart contract code and doesn't come with client, operators may need to chew some glass.

This also needed to be replace for the program's onchain address: Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS

## Introduction

In light of the growing interest in Ethena, I've decided to open-source a trading bot I developed in the third quarter of 2023. This bot is designed to utilize price differences and funding differences in different markets, specifically through a strategy involving SOL-PERP and SOL-SPOT transactions across two platforms: Drift and Phoenix.

## Strategy Overview

The bot operates on two main principles:

1. **Long Position Arbitrage**: It sells PERP contracts on Drift and buys SPOT on Phoenix when the opposite price discrepancy occurs, ensuring profitability after fees. This transaction is only executed under the condition that the trade remains profitable after accounting for all applicable fees. Furthermore, this strategy is activated only during positive funding periods, which means you are compensated for maintaining a SHORT PERP position on Drift.

2. **Short Position Arbitrage**: It buys PERP contracts on Drift and sells SPOT on Phoenix whenever the price of PERP on Drift is lower than the SPOT price on Phoenix. This transaction is only executed under the condition that the trade remains profitable after accounting for all applicable fees. Furthermore, this strategy is activated only during negative funding periods, which means you are compensated for maintaining a LONG PERP position on Drift.

## Requirements for Effective Operation

- **Fee Tier**: Operator should qualify for the best fee tier on Drift, which involves depositing into the Drift insurance fund.
- **Price Differences and Funding**: To enhance the likelihood of order fulfillment, operator should be willing to accept minor price differences to enter positions that attract funding payments. (Initial negative position PNL).

## Version Compatibility

This bot was designed based on specific versions of the Drift and Phoenix platforms available in Q3 2023. Given the rapid evolution of these platforms, the bot may require updates to remain functional. Operator are advised to upgrade the software versions of Drift and Phoenix to their latest to ensure compatibility.

This bot should be able to be build with Anchor 0.26.0, Solana 1.17.0.

Lastly, if you are slabbing on a stable coin to make an Ethena-like project, my only ask is include me in the angel round.

