pub trait EraVisitor<T: frame_system::Config> {
	fn current_era() -> Option<sp_staking::EraIndex>;
}
