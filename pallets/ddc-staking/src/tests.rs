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
