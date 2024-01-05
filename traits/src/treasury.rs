pub trait TreasuryVisitor<T: Config> {
    fn cluster_has_node(cluster_id: &ClusterId, node_pub_key: &NodePubKey) -> bool;
