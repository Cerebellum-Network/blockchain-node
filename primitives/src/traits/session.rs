pub trait SessionVisitor {
	fn current_index() -> sp_staking::SessionIndex;
}
