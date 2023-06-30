//! Tests for the module.

use super::{*, mock::*};
use frame_support::assert_ok;

#[test]
fn set_settings_works() {
	ExtBuilder::default().build_and_execute(|| {
        // setting works
        assert_ok!(
            DdcStaking::set_settings(
                Origin::root(),
                1,
                Some(ClusterSettings {
                    edge_bond_size: 10,
                    edge_chill_delay: 2,
                    storage_bond_size: 10,
                    storage_chill_delay: 2,
                }),
            )
        );
        let settings = DdcStaking::settings(1);
        assert_eq!(settings.edge_bond_size, 10);
        assert_eq!(settings.edge_chill_delay, 2);
        assert_eq!(settings.storage_bond_size, 10);
        assert_eq!(settings.storage_chill_delay, 2);

        // removing works
        assert_ok!(DdcStaking::set_settings(Origin::root(), 1, None));
        let settings = DdcStaking::settings(1);
        let default_settings: ClusterSettings::<Test> = Default::default();
        assert_eq!(settings.edge_bond_size, default_settings.edge_bond_size);
        assert_eq!(settings.edge_chill_delay, default_settings.edge_chill_delay);
        assert_eq!(settings.storage_bond_size, default_settings.storage_bond_size);
        assert_eq!(settings.storage_chill_delay, default_settings.storage_chill_delay);
    });
}

#[test]
fn basic_setup_works() {
    // Verifies initial conditions of mock
	ExtBuilder::default().build_and_execute(|| {
		// Account 11 is stashed and locked, and account 10 is the controller
        assert_eq!(DdcStaking::bonded(&11), Some(10));
		// Account 21 is stashed and locked, and account 20 is the controller
        assert_eq!(DdcStaking::bonded(&21), Some(20));
        // Account 1 is not a stashed
        assert_eq!(DdcStaking::bonded(&1), None);

		// Account 10 controls the stash from account 11, which is 100 units
        assert_eq!(
			DdcStaking::ledger(&10),
			Some(StakingLedger {
				stash: 11,
				total: 100,
				active: 100,
                chilling: Default::default(),
				unlocking: Default::default(),
			})
		);
		// Account 20 controls the stash from account 21, which is 100 units
		assert_eq!(
			DdcStaking::ledger(&20),
			Some(StakingLedger {
				stash: 21,
				total: 100,
				active: 100,
                chilling: Default::default(),
				unlocking: Default::default(),
			})
		);
        // Account 1 does not control any stash
		assert_eq!(DdcStaking::ledger(&1), None);

        // Cluster 1 settings are default
        assert_eq!(DdcStaking::settings(1), Default::default());
    });
}
