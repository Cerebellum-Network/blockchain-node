# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Legend

- [C] Changes is `Cere` Runtime
- [D] Changes is `Cere Dev` Runtime

## [6.3.0]

- [C,D] Update Substrate from `v1.10` to `v1.12`.

## [6.2.0]

- [C,D] Update Substrate from `v1.9` to `v1.10`.

## [6.1.0]

- [C,D] Update Substrate from `v1.8` to `v1.9`.

## [6.0.0]

- [C,D] Update Substrate from `v1.7` to `v1.8`.

## [5.9.0]

- [C,D] Update Substrate from `v1.6` to `v1.7`.

## [5.8.0]

- [C,D] Update Substrate from `v1.5` to `v1.6`.

## [5.7.0]

- [C,D] Update Substrate from `v1.4` to `v1.5`.

## [5.6.0]

- [C,D] Update Substrate from `v1.2` to `v1.4`.

## [5.5.0]

- [C,D] Update Substrate from `v1.1` to `v1.2`.

## [5.4.0]

- [C,D] Introduce new events to the DDC Payouts Pallet
- [C,D] `pallet-ddc-clusters-gov`: Introduction of the Cluster Governance pallet for managing clusters protocol parameters.

## [5.3.1]

### Changed

- [C,D] `WhitelistOrigin` is set to the Technical Committee Collective Body
- [C,D] The _Support Curve_ in OpenGov Tracks is made more strict

## [5.3.0]

- [C,D] Updated Substrate to polkadot-v1.1.0
- [C,D] Introduction of the OpenGov
- [C,D] `pallet-ddc-clusters`: Added Erasure coding and Replication in cluster params

## [5.2.2]

- [C,D] Depositing extra amount in ddc-customers pallet is fixed

## [5.2.1]

### Changed

- [C,D] Fix inflation parameters for the staking reward curve

## [5.2.0]

### Added

- [C,D] Missing storage migrations to Staking pallet

### Changed

- [C,D] Remove Society pallet
- [C,D] Bump Balances storage version

## [5.1.4]

### Changed

- [C,D] Inflation parameters for the staking reward curve are back to normal values
- [C,D] Daily burning is set to 2.5%

## [5.1.3]

### Changed

- [C,D] Fixed prefixes for ChainBridge's pallet storage items
- [C,D] Fixed prefixes for ERC721 pallet storage items

## [5.1.2]

### Changed

- [C,D] Inflation parameters for the staking reward curve are doubled to temporarily increase validators payouts
- [C,D] Daily burning is set to 0.058%

## [5.1.1]

### Added

- [C,D] Missing storage migrations for `pallet_contracts`, `pallet_im_online`, `pallet_democracy`, and `pallet_fast_unstake`

## [5.1.0]

### Changed

- [C] `5.0.1` release changes are reverted
- [C,D] Off-chain workers are enabled

## [5.0.1]

### Changed

- [C,D] Set burn rate at 0.058% CERE tokens at the end of every era.

## [5.0.0]

### Changed

- [C,D] Updated Substrate to polkadot-v1.0.0
- [C,D] `pallet-ddc-customers`: implemented bucket removal

### Added
- Added ChargeError event to payout pallet

## [4.8.9]

### Changed

- [C,D] Updated Substrate to polkadot-v0.9.42
- Added ChargeError event to payout pallet
- Introduce a burn rate of 0.058% daily to bring inflation down.
- More explicit events in `pallet-ddc-payouts` about batch index

## [4.8.8]

### Changed

- [C,D] Updated Substrate to polkadot-v0.9.40
- More explicit events in `pallet-ddc-payouts` and `pallet-ddc-customers`
- Introduce a burn rate of 0.058% daily to bring inflation down.
- More explicit events in `pallet-ddc-payouts` about batch index

## [4.8.7]

### Changed

- [C,D] Updated Substrate to polkadot-v0.9.38
- [C] Added pallet-preimage to support democracy functionality.
- Changes in `pallet-ddc-payouts::begin_billing_report` crate to accept start and end of the era.
- More explicit events in `pallet-ddc-payouts` and `pallet-ddc-customers`

## [4.8.6]

### Changed

- [C,D] Updated Substrate to polkadot-v0.9.37
- More explicit events in `pallet-ddc-payouts` and `pallet-ddc-customers`
- More explicit events in `pallet-ddc-payouts` about batch index

## [4.8.5]

### Changed

- [C,D] Updated Substrate to polkadot-v0.9.36
- [C] Added pallet-preimage to support democracy functionality.

## [4.8.4]

### Changed

- [C,D] Updated Substrate to polkadot-v0.9.33

## [4.8.3]

## Changed

- [C,D] Updated Substrate to polkadot-v0.9.31

## [4.8.2]

### Added

- [C,D] New `pallet-ddc-nodes` is added which allows registering a DDC node within the network with specific settings.
- [C,D] New `pallet-ddc-clusters` is added which allows launching a DDC cluster in the network and managing it.
- [C,D] New `pallet-ddc-staking` is added which allows making bonds for DDC nodes before joining a DDC cluster.
- [C,D] New `pallet-ddc-customers` is added which allows depositing tokens and creating buckets for DDC customers.
- [C,D] New `pallet-ddc-payouts` is added which allows processing payouts to DDC nodes providers based on DAC validation results.
- New `ddc-primitives` crate with DDC common types definition.
- New `ddc-traits` crate with DDC common traits definition.

### Changed

- [C] Fixed governance parameters

## [4.8.1]

### Added

- [C,D] Contract migration

## [4.8.0]

### Added

- [D] Handlebars template to generate weights file
- [D] Genesis config for `pallet-ddc-staking` to set genesis DDC participants (empty by default) and staking settings
- [D] Unit tests in `pallet-ddc-staking` for basic staking scenario

### Changed

- [C,D] Updated Substrate to polkadot-v0.9.30

## [4.7.2]

### Changed

- Reduce by 2 orders of magnitude the constants changed in v4.7.1

## [4.7.1]

### Changed
- Updated governance related constants

## [4.7.0]

### Changed

- Updated Substrate to polkadot-v0.9.29

## [4.6.0]

### Changed

- Updated Substrate to polkadot-v0.9.28

## [4.5.0]

### Changed

- Updated Substrate to polkadot-v0.9.27

## [4.4.0]

### Changed

- Updated Substrate to polkadot-v0.9.26
- New `cluster` parameter for `serve` and `store` calls in `pallet-ddc-staking` to specify the DDC cluster ID which the caller is willing to join

## [4.3.0]

### Added

- DDC Staking `CurrentEra` follows DAC era counter

### Changed

- Preferences parameter removed from `pallet-ddc-staking` calls, `value` parameter returned back to `bond` and `unbond` calls
- Distinct bond size requirements for `Storage` and `Edge` roles with default to 100 CERE
- Updated Substrate to polkadot-v0.9.25

## [4.2.0]

### Changed

- Updated Substrate to polkadot-v0.9.24

## [4.1.0]

### Added

- `rust-toolchain.toml` as a single source of truth on toolchain requirements (except Nix builder)
- Cere Dev Local Testnet config
- New `set_staking_configs` call in `pallet-ddc-staking` to allow to set DDC Staking bond size by the authority

### Changed

- `pallet-ddc-staking` now requires one fixed size bond for both `Storage` and `Edge` roles instead of the bond limited by the lower boundary only
- Updated Substrate to polkadot-v0.9.23

## [4.0.0]

### Added

- Code from the [old repository](https://github.com/Cerebellum-Network/pos-network-node) to follow the [Substrate node template](https://github.com/substrate-developer-hub/substrate-node-template). Its CHANGELOG can be found [here](https://github.com/Cerebellum-Network/pos-network-node/blob/master-cere/CHANGELOG.md).
- Support for various runtimes: `cere` and `cere-dev`
