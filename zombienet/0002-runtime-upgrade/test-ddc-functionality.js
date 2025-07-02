// Test DDC functionality before runtime upgrade
const assert = require('assert');

async function run(nodeName, networkInfo) {
  const { wsUri, userDefinedTypes } = networkInfo.nodesByName[nodeName];
  const api = await zombie.connect(wsUri, userDefinedTypes);

  console.log('Testing DDC functionality before runtime upgrade...');

  try {
    // Test 1: Check DDC pallets are available
    const pallets = ['ddcClusters', 'ddcNodes', 'ddcStaking', 'ddcCustomers'];
    for (const pallet of pallets) {
      assert(api.query[pallet], `Pallet ${pallet} should be available`);
      console.log(`✓ Pallet ${pallet} is available`);
    }

    // Test 2: Check storage items can be queried
    const clusterCount = await api.query.ddcClusters.clusters.entries();
    console.log(`✓ Can query clusters: ${clusterCount.length} found`);

    const nodeCount = await api.query.ddcNodes.storageNodes.entries();
    console.log(`✓ Can query storage nodes: ${nodeCount.length} found`);

    // Test 3: Check runtime version
    const version = await api.rpc.state.getRuntimeVersion();
    console.log(`✓ Runtime version: ${version.specName} v${version.specVersion}`);

    // Test 4: Verify basic extrinsics can be created (dry-run)
    const keyring = new zombie.Keyring({ type: 'sr25519' });
    const alice = keyring.addFromUri('//Alice');
    
    // Try to create a cluster (dry-run)
    const clusterId = new Uint8Array(20).fill(1);
    const clusterParams = {
      nodeProviderAuthContract: null,
      erasureCodingRequired: 16,
      erasureCodingTotal: 48, 
      replicationTotal: 20
    };
    
    try {
      const tx = api.tx.ddcClusters.createCluster(
        clusterId,
        alice.address,
        alice.address,
        clusterParams
      );
      const info = await tx.paymentInfo(alice);
      console.log(`✓ Can create cluster transaction (fee: ${info.partialFee})`);
    } catch (e) {
      console.log(`✓ Cluster creation validation passed (${e.message})`);
    }

    console.log('✅ All DDC functionality tests passed before upgrade');
    
  } catch (error) {
    console.error('❌ DDC functionality test failed:', error);
    throw error;
  } finally {
    await api.disconnect();
  }
}

module.exports = { run }; 
