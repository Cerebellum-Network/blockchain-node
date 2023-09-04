// based on: https://github.com/paritytech/polkadot-sdk/blob/91deee7a1dba52e5e73d1a97d9fd5b8ad1e916a4/substrate/zombienet/0000-block-building/transaction-gets-finalized.js

const assert = require('assert');

async function run(nodeName, networkInfo) {
  const { wsUri, userDefinedTypes } = networkInfo.nodesByName[nodeName];
  const api = await zombie.connect(wsUri, userDefinedTypes);

  // Construct the keyring after the API (crypto has an async init)
  const keyring = new zombie.Keyring({ type: 'sr25519' });

  // Add Alice to our keyring with a hard-derivation path (empty phrase, so uses dev)
  const alice = keyring.addFromUri('//Alice');
  const bob = '5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty';

  // Create a extrinsic, transferring 10^10 units to Bob
  const transfer = api.tx.balances.transfer(bob, 10n ** 10n);

  let transaction_success_event = false;
  try {
    await new Promise(async (resolve, reject) => {
      const unsubscribe = await transfer
        .signAndSend(alice, { nonce: -1 }, ({ events = [], status }) => {
          console.log('Transaction status:', status.type);

          if (status.isInBlock) {
            console.log('Included at block hash', status.asInBlock.toHex());
            console.log('Events:');

            events.forEach(({ event: { data, method, section }, phase }) => {
              console.log('\t', phase.toString(), `: ${section}.${method}`, data.toString());

              if (section == 'system' && method == 'ExtrinsicSuccess') {
                transaction_success_event = true;
              }
            });
          } else if (status.isFinalized) {
            console.log('Finalized block hash', status.asFinalized.toHex());
            unsubscribe();
            if (transaction_success_event) {
              resolve();
            } else {
              reject('ExtrinsicSuccess has not been seen');
            }
          } else if (status.isError) {
            unsubscribe();
            reject('Transaction status.isError');
          }

        });
    });
  } catch (error) {
    assert.fail('Transfer promise failed, error: ' + error);
  }

  assert.ok('test passed');
}

module.exports = { run }
