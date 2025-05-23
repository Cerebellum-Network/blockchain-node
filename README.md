# Cere Blockchain Node

## Build

### Rust Setup

First, complete the [basic Rust setup instructions](./docs/rust-setup.md).

### Build Environment Setup

```sh
./scripts/init.sh
```

### Build

Use the following command to build the node without launching it:

```sh
cargo +1.82.0 build --release
```

## Run

### Single-Node Development Chain

This command will start the single-node development chain with non-persistent state:

```bash
./target/release/cere --dev
```

Purge the development chain's state:

```bash
./target/release/cere purge-chain --dev
```

Start the development chain with detailed logging:

```bash
RUST_BACKTRACE=1 ./target/release/cere -ldebug --dev
```

> Development chain means that the state of our chain will be in a tmp folder while the nodes are
> running. Also, **alice** account will be authority and sudo account as declared in the
> [genesis state](https://github.com/Cerebellum-Network/blockchain-node/blob/dev/node/service/src/chain_spec.rs#L241).
> At the same time the following accounts will be pre-funded:
> - Alice
> - Bob
> - Alice//stash
> - Bob//stash

In case of being interested in maintaining the chain' state between runs a base path must be added
so the db can be stored in the provided folder instead of a temporal one. We could use this folder
to store different chain databases, as a different folder will be created per different chain that
is ran. The following commands shows how to use a newly created folder as our db base path.

```bash
// Create a folder to use as the db base path
$ mkdir my-chain-state

// Use of that folder to store the chain state
$ ./target/release/cere --dev --base-path ./my-chain-state/

// Check the folder structure created inside the base path after running the chain
$ ls ./my-chain-state
chains
$ ls ./my-chain-state/chains/
dev
$ ls ./my-chain-state/chains/dev
db keystore network
```

### Local Testnet Development Chain
Purge the Alice's node state:
```bash
./target/release/cere purge-chain --base-path /tmp/alice --chain local -y
```
Start Alice's node:
```bash
./target/release/cere \
  --base-path /tmp/alice \
  --chain local \
  --alice \
  --port 30333 \
  --ws-port 9945 \
  --rpc-port 9933 \
  --rpc-cors "http://localhost:*","http:127.0.0.1:*","https://localhost:*","https:127.0.0.1:*","https://explorer.cere.network","https://polkadot.js.org" \
  --node-key 0000000000000000000000000000000000000000000000000000000000000001 \
  --validator
```
Purge the Bob's node state:
```bash
./target/release/cere purge-chain --base-path /tmp/bob --chain local -y
```
Start Bob's node:
```bash
./target/release/cere \
  --base-path /tmp/bob \
  --chain local \
  --bob \
  --port 30334 \
  --ws-port 9946 \
  --rpc-port 9934 \
  --rpc-cors "http://localhost:*","http:127.0.0.1:*","https://localhost:*","https:127.0.0.1:*","https://explorer.cere.network","https://polkadot.js.org" \
  --validator \
  --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp
```

> Development chain means that the state of our chain will be in a tmp folder while the nodes are
> running. Also, **alice** and **bob** accounts will be authority accounts and **alice** sudo account as declared in the
> [genesis state](https://github.com/Cerebellum-Network/blockchain-node/blob/dev/node/service/src/chain_spec.rs#279).
> At the same time the following accounts will be pre-funded:
> - Alice
> - Bob
> - Alice//stash
> - Bob//stash

### Zombienet

Zombienet is a cli tool to easily spawn ephemeral networks and perform tests against them. Its installation and usage guide is available [here](https://github.com/paritytech/zombienet#usage).

The following scenarios expect the node binary available at `./target/release/cere`.

#### Test block building

Spawn 2 nodes network and test if it produces blocks and finalized transaction.

```console
zombienet -p native test zombienet/0000-block-building/block-building.zndsl
```

#### Spawn 5 DDC validation nodes

The following command spawns 5 validator nodes with DDC validation enabled as well as 1 non-validator node to check it is not affected. Set `DAC_URL` environment variable with an address to [webdis](https://webd.is/) which will proxy validator's requests for DDC activity data to DAC DataModel Redis.

```console
export DAC_URL=http://localhost:7379/
zombienet -p native test zombienet/0001-ddc-validation/ddc-validation.toml
```

### Runtimes

The node supports 2 runtimes.

#### Runtime `cere`

Runtime `cere` uses by default in Cere Mainnet/Testnet/QAnet. You can start the node with it by:
1. Running the node connected to [Cere Mainnet](#mainnet), [Cere Testnet](#testnet) or [Cere QAnet](#qanet)
2. Running the node with a custom spec. Be sure that [id](https://github.com/Cerebellum-Network/blockchain-node/blob/dev-cere/node/service/src/chain_spec.rs#L265) **does not** start with `cere_dev`
    ```bash
    ./target/release/cere --chain=./target/release/customSpecRaw.json
    ```

#### Runtime `cere-dev`

Runtime `cere-dev` uses by default in Cere Devnet. You can start the node with it by:
1. Running the node connected to [Cere Devnet](#Devnet)
1. Running the [Single-Node Development Chain](#Single-Node-Development-Chain)
1. Running the [Local Testnet Development Chain](#local-testnet-development-chain)
1. Running the node with a custom spec. Be sure that [id](https://github.com/Cerebellum-Network/blockchain-node/blob/dev-cere/node/service/src/chain_spec.rs#L265) **starts** with `cere_dev` and you pass `--force-cere-dev` parameter
    ```bash
    ./target/release/cere --chain=./target/release/customSpecRaw.json --force-cere-dev
    ```

### Connect to Cere Networks

#### Mainnet

```bash
./target/release/cere --chain=cere-mainnet
```

#### Testnet

```bash
./target/release/cere --chain=cere-testnet
```

#### QAnet

```bash
./target/release/cere --chain=cere-qanet
```

#### Devnet

```bash
./target/release/cere --chain=cere-devnet
```

## Connect with Cere Explorer Front-end

Once the node is running locally, you can connect it with **Cere Explorer** front-end
to interact with your chain. [Click
here](https://explorer.cere.network/?rpc=ws://localhost:9944) connecting the Explorer to your
local node.
