use crate as pallet_security_audit;
use frame_support::{
    derive_impl,
    parameter_types,
    traits::ConstU32,
    PalletId,
};
use frame_system as system;
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup, Convert},
    BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
    pub enum Test
    {
        System: frame_system,
        Timestamp: pallet_timestamp,
        SecurityAudit: pallet_security_audit,
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type RuntimeTask = RuntimeTask;
    type Nonce = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Block = Block;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type DbWeight = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
}

parameter_types! {
    pub const MinimumPeriod: u64 = 1;
}

impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

parameter_types! {
    pub const SecurityAuditPalletId: PalletId = PalletId(*b"sec_audt");
    pub const MaxSecurityEvents: u32 = 1000;
    pub const MaxEventDetailsSize: u32 = 1024;
    pub const MaxActionSize: u32 = 256;
}

// Converter to convert timestamp (u64) to u64 (identity conversion for tests)
pub struct TimestampToU64Converter;
impl Convert<u64, u64> for TimestampToU64Converter {
    fn convert(moment: u64) -> u64 {
        moment
    }
}

impl pallet_security_audit::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type PalletId = SecurityAuditPalletId;
    type MaxSecurityEvents = MaxSecurityEvents;
    type MaxEventDetailsSize = MaxEventDetailsSize;
    type MaxActionSize = MaxActionSize;
    type SecurityOrigin = frame_system::EnsureRoot<Self::AccountId>;
    type TimestampToU64 = TimestampToU64Converter;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let t = system::GenesisConfig::<Test>::default().build_storage().unwrap();
    t.into()
} 
