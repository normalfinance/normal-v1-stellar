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

- Invest in Top 100 cryptos without leaving Stellar
- Invest in diversified portfolios of crypto with just one click
- Create your own crypto portfolios and earn income when others invest in them
- Earn competitive yield on XLM by providing collateral to mint synthetic assets
- Earn yield by providing liquidity to various Normal AMMs

## Contracts

The core smart contracts powering the Normal Protocol:

- [`Insurance`](https://github.com/normalfinance/normal-v1-stellar/tree/master/contracts/insurance) - A backstop fund to cover protocol debt (deficits incurred from liquidations). Pays yield to depositors in exchange for the right to make a claim in the event of protocol debt.
- [`Market`](https://github.com/normalfinance/normal-v1-stellar/tree/master/contracts/market) - Manages the creation, maintenance, margin, collateral, liquidation, and price peg of a synthetic asset.
- [`MarketFactory`](https://github.com/normalfinance/normal-v1-stellar/tree/master/contracts/market_factory) - Deploys new `Market` contracts and provides limited query access.

## Packages

dfdsf

- [`Normal`](https://github.com/normalfinance/normal-v1-stellar/tree/master/packages/normal) - Coming soon.
- [`Oracle`](https://github.com/normalfinance/normal-v1-stellar/tree/master/packages/oracle) - A proxy between oracle prices and synth and index price pegs to apply various validations and allow emergency oracle replacement or freezing.

## Deployment

Testnet and Mainnet addresses of the contract detailed above:

| Contract Name  | Testnet Address    | Mainnet Address    |
| -------------- | ------------------ | ------------------ |
| Insurance      | `<insert address>` | `<insert address>` |
| Market Factory | `<insert address>` | `<insert address>` |

## Markets / Pools

Deployed synthetic markets and their respective Synth Pool:

| Market Name | Market Address     |
| ----------- | ------------------ |
| nBTC/XLM    | `<insert address>` |
| nETH/XLM    | `<insert address>` |
| nSOL/XLM    | `<insert address>` |

## Authors

- [@jmoneynormal](https://www.github.com/jmoneynormal)

## Contributing

Contributions are always welcome!

See `contributing.md` for ways to get started.

Please adhere to this project's `code of conduct`.

## Roadmap

- Additional market support
- Synthetic yield tokens
- Increased index customization
- Limit orders
- Additional collateral types

## Used By

This project is used by the following companies:

- Coming soon

## License

[Apache-2.0](https://choosealicense.com/licenses/apache-2.0/)
