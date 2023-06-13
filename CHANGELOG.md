# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [vNext]

### Changed

- Updated Substrate to polkadot-v0.9.26

## [4.3.0]

### Changed

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
