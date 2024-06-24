use crate::{ClusterId, DdcEra, PayoutState};

pub trait PayoutVisitor<T: frame_system::Config> {
	fn get_billing_report_status(cluster_id: ClusterId, era: DdcEra) -> PayoutState;
}
