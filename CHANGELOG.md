# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Legend
- [C] Changes is `Cere` Runtime
- [D] Changes is `Cere Dev` Runtime

## [vNext + 1]

### Changed

- [C,D] Updated Substrate to polkadot-v0.9.31

## [vNext]

### Added

- [D] New `pallet-ddc-validator` which implements DDC CDN nodes validation and rewarding. You can enable DDC validation providing `--enable-ddc-validation` argument and `--dac-url` argument to specify DAC endpoint. It will only work on the nodes with validation and offchain workers enabled as well.
- [D] Several calls for `pallet-ddc-staking` to distribute rewards.
- [D] Third kind of account in DDC Staking for DDC nodes (along with stash and controller).
- [D] DDC cluster managers access control list in `pallet-ddc-staking` managed by governance.
- [Zombienet](https://github.com/paritytech/zombienet) configurations to test block building and spawn a network for DDC validation debugging.
- New `ddc-primitives` crate with DDC common types definition

### Changed

- ...

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
