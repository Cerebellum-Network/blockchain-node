# State preset

Sometimes we use the blockchain as a part of a test environment.
Those tests typically want a certain state of the blockchain.
One option is to make a script which sends a number of transactions to put the blockchain into the desired state.
But those scripts may become large and execution may take significant time.
Another option is to make a chain specification, a JSON file with the desired genesis state of the blockchain.
And launch the chain with the desired state from the first block.

## Build a chain spec

Chain specification is a JSON file which contains network parameters with the the initial runtime and a genesis state of the database for each runtime module.
There are two forms of chain specification - a human-readable _plain_ form where each runtime module is represented by it's name.
And a _raw_ form with key-value pairs which will be written to the database.

1. Create a plain form chain spec.

    ```console
    ./target/release/cere build-spec --chain=dev --disable-default-bootnode > plain.json
    ```

1. Set genesis state for each module.
    There is an example in `node/service/example.json`, you can copy everything except the `code` field value from it to the `plain.json`.

1. Create a raw form chain spec.

    ```console
    ./target/release/cere build-spec --chain=plain.json --disable-default-bootnode --raw > raw.json
    ```

## Launch the chain

```console
./target/release/cere --chain=raw.json --tmp --alice --unsafe-rpc-external --unsafe-ws-external --rpc-cors=all
```

## See also

[docs.substrate.io: Create a custom chain specification](https://docs.substrate.io/tutorials/build-a-blockchain/add-trusted-nodes/#create-a-custom-chain-specification)
