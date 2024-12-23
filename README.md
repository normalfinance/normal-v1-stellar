<div align="center">
  <a href="https://www.normalfinance.io/protocol">
    <img src="https://cdn.prod.website-files.com/6595b2282ea917577755d3a5/6595bb9290625dfff5df3f7e_Logo%20-%20Color.svg" alt="Normal logo" width="340"/>
  </a>
</div>

<div>
  <a href="https://discord.gg/hayb9pafjZ"><img src="https://img.shields.io/discord/928701482319101952"/></a>
  <a  href="https://github.com/normalfinance/normal-v1-stellar/releases"><img src="https://img.shields.io/github/release-pre/normalfinance/normal-v1-stellar.svg"/></a>
  <a  href="https://github.com/normalfinance/normal-v1-stellar/pulse"><img src="https://img.shields.io/github/contributors/normalfinance/normal-v1-stellar.svg"/></a>
  <a href="https://opensource.org/license/apache-2-0"><img src="https://img.shields.io/github/license/normalfinance/normal-v1-stellar"/></a>
  <a href="https://github.com/normalfinance/normal-v1-stellar/pulse"><img src="https://img.shields.io/github/last-commit/normalfinance/normal-v1-stellar.svg"/></a>
  <a href="https://github.com/normalfinance/normal-v1-stellar/pulls"><img src="https://img.shields.io/github/issues-pr/normalfinance/normal-v1-stellar.svg"/></a>
 
  <a href="https://github.com/normalfinance/normal-v1-stellar/issues"><img src="https://img.shields.io/github/issues/normalfinance/normal-v1-stellar.svg"/></a>
  <a href="https://github.com/normalfinance/normal-v1-stellar/issues"><img src="https://img.shields.io/github/issues-closed/normalfinance/normal-v1-stellar.svg"/></a>
</div>

# Normal v1 on Stellar ✨

Normal is an over-collateralized synthetic asset protocol enabling low fee and deep liquidity trading of Top 100 crypto assets natively on Stellar.

## Features

-   Invest in Top 100 cryptos without leaving Stellar
-   Invest in diversified portfolios of crypto with just one click
-   Create your own crypto portfolios and earn income when others invest in them
-   Earn competitive yield on XLM by providing collateral to mint synthetic assets
-   Earn yield by providing liquidity to various Normal AMMs

## Contracts

The core smart contracts powering the Normal Protocol:

-   [`AMM`](https://github.com/normalfinance/normal-v1-stellar/tree/develop/soroban/contracts/amm) - A constant product automated market maket (AMM) for swapping in and out of synthetic assets.
-   [`Governor`](https://github.com/normalfinance/normal-v1-stellar/tree/develop/soroban/contracts/governor) - Fork of [soroban-governor](https://github.com/script3/soroban-governor), the core contract of the Normal DAO. Its core responsibility is managing the proposal workflow. Proposals allow the Governor contract to interact with the greater Soroban ecosystem, enabling things like the Governor sending funds to a grant recipient or depositing into an AMM.
-   [`Index`](https://github.com/normalfinance/normal-v1-stellar/tree/develop/soroban/contracts/index) - Manages the creation, maintenance, minting, redeeming, and price peg of an on-chain crypto index fund.
-   [`IndexFactory`](https://github.com/normalfinance/normal-v1-stellar/tree/develop/soroban/contracts/index_factory) - Deploys `Index` contracts and provides structured access using an `index_id`.
-   [`IndexToken`](https://github.com/normalfinance/normal-v1-stellar/tree/develop/soroban/contracts/index_token) - A Soroban token representing ownership in an `Index`. Extension of the Token Interface to collect a `protocol_fee` and `manager_fee` on qualifying `transfer` and `transfer_from` transactions.
-   [`InsuranceFund`](https://github.com/normalfinance/normal-v1-stellar/tree/develop/soroban/contracts/insurance_fund) - A backstop fund to cover protocol debt (deficits incurred from liquidations). Pays yield to depositors in exchange for the right to make a claim in the event of protocol debt.
-   [`OracleSecurityModule`](https://github.com/normalfinance/normal-v1-stellar/tree/develop/soroban/contracts/osm) - A proxy between oracle prices and synth and index price pegs to apply various validations and allow emergency oracle replacement or freezing.
-   [`Scheduler`](https://github.com/normalfinance/normal-v1-stellar/tree/develop/soroban/contracts/scheduler) - An on-chain dollar cost average order schedulor for recurring buys or sells of synthetic assets or crypto indexes. Does not currently support 3rd party DEXes.
-   [`State`](https://github.com/normalfinance/normal-v1-stellar/tree/develop/soroban/contracts/state) - Global configuration values for the Normal Protocol and its constituent contracts.
-   [`SynthMarket`](https://github.com/normalfinance/normal-v1-stellar/tree/develop/soroban/contracts/synth_market) - Manages the creation, maintenance, margin, collateral, liquidation, and price peg of a synthetic asset.
-   [`SynthMarketFactory`](https://github.com/normalfinance/normal-v1-stellar/tree/develop/soroban/contracts/synth_market_factory) - Deploys `SynthMarket` contracts and provides structed access using a `market_index`.
-   [`Votes`](https://github.com/normalfinance/normal-v1-stellar/tree/develop/soroban/contracts/votes) - Fork of [soroban-governor](https://github.com/script3/soroban-governor), enforcing bonding/vote-escrowed access to participation in Normal DAO proposals.

## Deployment

Testnet and Mainnet addresses of the contract detailed above:

| Contract Name          | Testnet Address    | Mainnet Address    |
| ---------------------- | ------------------ | ------------------ |
| Governor               | `<insert address>` | `<insert address>` |
| Index Factory          | `<insert address>` | `<insert address>` |
| Index Token            | `<insert address>` | `<insert address>` |
| Insurance Fund         | `<insert address>` | `<insert address>` |
| Oracle Security Module | `<insert address>` | `<insert address>` |
| Scheduler              | `<insert address>` | `<insert address>` |
| State                  | `<insert address>` | `<insert address>` |
| Synth Market Factory   | `<insert address>` | `<insert address>` |
| Votes                  | `<insert address>` | `<insert address>` |

## Markets / AMMs

Deployed synthetic markets and their respective AMM:

| Market Name | Market Address     | AMM Address        |
| ----------- | ------------------ | ------------------ |
| nBTC/XLM    | `<insert address>` | `<insert address>` |
| nETH/XLM    | `<insert address>` | `<insert address>` |
| nSOL/XLM    | `<insert address>` | `<insert address>` |

## Crypto Indexes

Deployed Normal Crypto Indexes:

| Index Name | Mainnet Address    |
| ---------- | ------------------ |
| NT5CI      | `<insert address>` |

## Authors

-   [@jmoneynormal](https://www.github.com/jmoneynormal)

## Contributing

Contributions are always welcome!

See `contributing.md` for ways to get started.

Please adhere to this project's `code of conduct`.

## Roadmap

-   Additional market support
-   Synthetic yield tokens
-   Increased index customization
-   Limit orders
-   Additional collateral types

## Used By

This project is used by the following companies:

-   Coming soon

## License

[Apache-2.0](https://choosealicense.com/licenses/apache-2.0/)
