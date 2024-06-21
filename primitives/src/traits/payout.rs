use crate::{ClusterId, DdcEra, PayoutState};

pub trait PayoutVisitor<T: frame_system::Config> {
	type Call: From<frame_system::Call<T>>;

	fn get_begin_billing_report_call(
		cluster_id: ClusterId,
		era: DdcEra,
		start_era: i64,
		end_era: i64,
	) -> Self::Call;

	fn get_billing_report_status(cluster_id: ClusterId, era: DdcEra) -> PayoutState;
}
