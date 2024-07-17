pub trait SessionVisitor<T: frame_system::Config> {
	fn current_index() -> sp_staking::SessionIndex;
}
