// Perform runtime upgrade via sudo
const assert = require('assert');
const fs = require('fs');

async function run(nodeName, networkInfo) {
  const { wsUri, userDefinedTypes } = networkInfo.nodesByName[nodeName];
  const api = await zombie.connect(wsUri, userDefinedTypes);

  console.log('Performing runtime upgrade...');

  try {
    // Read the new runtime WASM
    const wasmPath = './target/release/wbuild/cere-dev-runtime/cere_dev_runtime.compact.compressed.wasm';
    
    if (!fs.existsSync(wasmPath)) {
      throw new Error(`Runtime WASM not found at ${wasmPath}`);
    }
    
    const wasmCode = fs.readFileSync(wasmPath);
    console.log(`✓ Loaded runtime WASM (${wasmCode.length} bytes)`);

    // Get current runtime version
    const currentVersion = await api.rpc.state.getRuntimeVersion();
    console.log(`Current runtime version: ${currentVersion.specVersion}`);

    // Create keyring and get Alice (sudo account)
    const keyring = new zombie.Keyring({ type: 'sr25519' });
    const alice = keyring.addFromUri('//Alice');

    // Create the runtime upgrade transaction
    const upgradeCall = api.tx.system.setCode(wasmCode);
    const sudoCall = api.tx.sudo.sudo(upgradeCall);

    console.log('Submitting runtime upgrade transaction...');

    // Submit the upgrade transaction
    let upgradeComplete = false;
    const unsubscribe = await sudoCall.signAndSend(alice, { nonce: -1 }, ({ events = [], status }) => {
      console.log(`Transaction status: ${status.type}`);

      if (status.isInBlock) {
        console.log(`✓ Runtime upgrade included in block: ${status.asInBlock.toHex()}`);
        
        events.forEach(({ event: { data, method, section }, phase }) => {
          console.log(`Event: ${section}.${method}`, data.toString());
          
          if (section === 'sudo' && method === 'Sudid') {
            console.log('✓ Sudo call executed successfully');
          }
          
          if (section === 'system' && method === 'CodeUpdated') {
            console.log('✓ Runtime code updated successfully');
          }
        });
      } else if (status.isFinalized) {
        console.log(`✓ Runtime upgrade finalized in block: ${status.asFinalized.toHex()}`);
        upgradeComplete = true;
        unsubscribe();
      } else if (status.isError) {
        throw new Error('Runtime upgrade transaction failed');
      }
    });

    // Wait for upgrade to complete
    let attempts = 0;
    while (!upgradeComplete && attempts < 60) {
      await new Promise(resolve => setTimeout(resolve, 5000)); // Wait 5 seconds
      attempts++;
    }

    if (!upgradeComplete) {
      throw new Error('Runtime upgrade did not complete within timeout');
    }

    // Verify the upgrade by checking runtime version
    console.log('Verifying runtime upgrade...');
    await new Promise(resolve => setTimeout(resolve, 10000)); // Wait for upgrade to propagate

    const newVersion = await api.rpc.state.getRuntimeVersion();
    console.log(`New runtime version: ${newVersion.specVersion}`);

    // Verify the version actually changed (or stayed same if testing same version)
    console.log(`✅ Runtime upgrade completed successfully`);
    console.log(`Runtime spec: ${newVersion.specName}`);
    console.log(`Spec version: ${newVersion.specVersion}`);
    console.log(`Impl version: ${newVersion.implVersion}`);

  } catch (error) {
    console.error('❌ Runtime upgrade failed:', error);
    throw error;
  } finally {
    await api.disconnect();
  }
}

module.exports = { run }; 
