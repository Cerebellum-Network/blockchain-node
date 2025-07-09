#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
	codec::{Decode, Encode, MaxEncodedLen},
	dispatch::DispatchResult,
	pallet_prelude::*,
	traits::{Get, UnixTime},
	weights::Weight,
};
use frame_system::pallet_prelude::*;
pub use pallet::*;
use sp_runtime::traits::Saturating;
use sp_std::vec::Vec;

/// Weight information for network monitor pallet
pub trait WeightInfo {
	fn update_network_health() -> Weight;
	fn report_peer_misbehavior() -> Weight;
	fn update_peer_info() -> Weight;
	fn whitelist_peer() -> Weight;
	fn detect_network_attacks() -> Weight;
	fn cleanup_old_data() -> Weight;
}

/// Default weight implementation
impl WeightInfo for () {
	fn update_network_health() -> Weight {
		Weight::from_parts(10_000, 0)
	}
	fn report_peer_misbehavior() -> Weight {
		Weight::from_parts(10_000, 0)
	}
	fn update_peer_info() -> Weight {
		Weight::from_parts(10_000, 0)
	}
	fn whitelist_peer() -> Weight {
		Weight::from_parts(10_000, 0)
	}
	fn detect_network_attacks() -> Weight {
		Weight::from_parts(15_000, 0)
	}
	fn cleanup_old_data() -> Weight {
		Weight::from_parts(20_000, 0)
	}
}

/// Network health status information
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen, Default)]
#[scale_info(skip_type_params(T))]
pub struct NetworkHealthStatus {
	/// Number of connected peers
	pub peer_count: u32,
	/// Block production rate (blocks per minute)
	pub block_rate: u32,
	/// Consensus participation rate (percentage)
	pub consensus_rate: u32,
	/// Overall security score (0-100)
	pub security_score: u32,
	/// Last update timestamp
	pub last_updated: u64,
}

/// Information about a connected peer
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct PeerInfo<T: Config> {
	/// Peer identifier
	pub peer_id: BoundedVec<u8, T::MaxPeers>,
	/// Reputation score (0-100)
	pub reputation: u32,
	/// Last seen block number
	pub last_seen: BlockNumberFor<T>,
	/// Number of misbehavior incidents
	pub misbehavior_count: u32,
	/// Connection timestamp
	pub connected_at: u64,
}

/// Security event types
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum SecurityEvent {
	/// Suspicious activity detected
	SuspiciousActivity {
		peer_id: BoundedVec<u8, ConstU32<32>>,
		reason: BoundedVec<u8, ConstU32<256>>,
		severity: SecuritySeverity,
	},
	/// Consensus failure detected
	ConsensusFailure { validator: BoundedVec<u8, ConstU32<32>>, block_number: u32 },
	/// Network partition detected
	NetworkPartition { affected_peers: u32, duration: u64 },
	/// High misbehavior count
	HighMisbehavior { peer_id: BoundedVec<u8, ConstU32<32>>, count: u32 },
	/// DDoS attack detected
	DDoSAttack {
		connection_rate: u32,
		source_ips: BoundedVec<BoundedVec<u8, ConstU32<32>>, ConstU32<100>>,
	},
	/// Eclipse attack detected
	EclipseAttack { peer_count: u32, isolated_duration: u64 },
}

/// Security event severity levels
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum SecuritySeverity {
	Low,
	Medium,
	High,
	Critical,
}

/// Network attack types for detection
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum AttackType {
	DDoS,
	Eclipse,
	Sybil,
	LongRange,
	Nothing,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Maximum number of peers to track
		#[pallet::constant]
		type MaxPeers: Get<u32>;

		/// Security threshold below which alerts are triggered
		#[pallet::constant]
		type SecurityThreshold: Get<u32>;

		/// Maximum number of security events to store per block
		#[pallet::constant]
		type MaxSecurityEvents: Get<u32>;

		/// Time provider for timestamps
		type UnixTime: UnixTime;

		/// Minimum peer count for healthy network
		#[pallet::constant]
		type MinPeerCount: Get<u32>;

		/// Maximum peer count before DDoS alert
		#[pallet::constant]
		type MaxPeerCount: Get<u32>;

		/// Weight information for extrinsics
		type WeightInfo: WeightInfo;
	}

	/// Current network health status
	#[pallet::storage]
	#[pallet::getter(fn network_health)]
	pub type NetworkHealth<T: Config> = StorageValue<_, NetworkHealthStatus, ValueQuery>;

	/// Information about connected peers
	#[pallet::storage]
	#[pallet::getter(fn connected_peers)]
	pub type ConnectedPeers<T: Config> =
		StorageMap<_, Blake2_128Concat, BoundedVec<u8, T::MaxPeers>, PeerInfo<T>, OptionQuery>;

	/// Security events by block number
	#[pallet::storage]
	#[pallet::getter(fn security_events)]
	pub type SecurityEvents<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		BlockNumberFor<T>,
		BoundedVec<SecurityEvent, T::MaxSecurityEvents>,
		ValueQuery,
	>;

	/// Blacklisted peers
	#[pallet::storage]
	#[pallet::getter(fn blacklisted_peers)]
	pub type BlacklistedPeers<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		BoundedVec<u8, T::MaxPeers>,
		(u64, BoundedVec<u8, T::MaxPeers>), // (timestamp, reason)
		OptionQuery,
	>;

	/// Network statistics for attack detection
	#[pallet::storage]
	#[pallet::getter(fn network_stats)]
	pub type NetworkStats<T: Config> = StorageValue<_, NetworkStatistics, ValueQuery>;

	/// Network statistics structure
	#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub struct NetworkStatistics {
		pub total_connections: u32,
		pub rejected_connections: u32,
		pub avg_block_time: u64,
		pub consensus_failures: u32,
		pub last_stats_update: u64,
	}

	impl Default for NetworkStatistics {
		fn default() -> Self {
			Self {
				total_connections: 0,
				rejected_connections: 0,
				avg_block_time: 6000, // 6 seconds default
				consensus_failures: 0,
				last_stats_update: 0,
			}
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Network health status updated
		NetworkHealthUpdated { status: NetworkHealthStatus },
		/// Security alert triggered
		SecurityAlert { event: SecurityEvent },
		/// Peer has been blacklisted
		PeerBlacklisted {
			peer_id: BoundedVec<u8, T::MaxPeers>,
			reason: BoundedVec<u8, T::MaxPeers>,
		},
		/// Peer has been whitelisted back
		PeerWhitelisted { peer_id: BoundedVec<u8, T::MaxPeers> },
		/// Network recovered from security issue
		NetworkRecovered { previous_score: u32, new_score: u32 },
		/// Attack detected and mitigated
		AttackDetected { attack_type: AttackType, severity: SecuritySeverity, peer_count: u32 },
		/// Peer reputation updated
		PeerReputationUpdated {
			peer_id: BoundedVec<u8, T::MaxPeers>,
			old_reputation: u32,
			new_reputation: u32,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Peer not found in storage
		PeerNotFound,
		/// Invalid security threshold
		InvalidSecurityThreshold,
		/// Network is unhealthy
		NetworkUnhealthy,
		/// Too many security events
		TooManySecurityEvents,
		/// Peer already blacklisted
		PeerAlreadyBlacklisted,
		/// Invalid peer data
		InvalidPeerData,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Update network health status
		#[pallet::weight(T::WeightInfo::update_network_health())]
		#[pallet::call_index(0)]
		pub fn update_network_health(origin: OriginFor<T>) -> DispatchResult {
			ensure_signed(origin)?;

			let health_status = Self::calculate_network_health();
			NetworkHealth::<T>::put(&health_status);

			// Check if security score is below threshold
			if health_status.security_score < T::SecurityThreshold::get() {
				Self::deposit_event(Event::SecurityAlert {
					event: SecurityEvent::NetworkPartition {
						affected_peers: health_status.peer_count,
						duration: 0,
					},
				});
			}

			Self::deposit_event(Event::NetworkHealthUpdated { status: health_status });
			Ok(())
		}

		/// Report peer misbehavior
		#[pallet::weight(T::WeightInfo::report_peer_misbehavior())]
		#[pallet::call_index(1)]
		pub fn report_peer_misbehavior(
			origin: OriginFor<T>,
			peer_id: BoundedVec<u8, T::MaxPeers>,
			reason: BoundedVec<u8, T::MaxPeers>,
			severity: SecuritySeverity,
		) -> DispatchResult {
			ensure_signed(origin)?;

			// Update peer misbehavior count
			ConnectedPeers::<T>::mutate(&peer_id, |peer_info| {
				if let Some(info) = peer_info {
					info.misbehavior_count = info.misbehavior_count.saturating_add(1);

					// Decrease reputation based on severity
					let reputation_decrease = match severity {
						SecuritySeverity::Low => 5,
						SecuritySeverity::Medium => 15,
						SecuritySeverity::High => 30,
						SecuritySeverity::Critical => 50,
					};

					let old_reputation = info.reputation;
					info.reputation = info.reputation.saturating_sub(reputation_decrease);

					// Emit reputation update event
					Self::deposit_event(Event::PeerReputationUpdated {
						peer_id: peer_id.clone(),
						old_reputation,
						new_reputation: info.reputation,
					});

					// Blacklist peer if misbehavior count is too high or reputation too low
					if info.misbehavior_count > 5 || info.reputation < 20 {
						let timestamp = T::UnixTime::now().as_millis() as u64;
						BlacklistedPeers::<T>::insert(&peer_id, (timestamp, reason.clone()));
						Self::deposit_event(Event::PeerBlacklisted {
							peer_id: peer_id.clone(),
							reason: reason.clone(),
						});
					}
				}
			});

			// Record security event
			let security_event = SecurityEvent::SuspiciousActivity {
				peer_id: BoundedVec::try_from(peer_id.clone().into_inner()).unwrap_or_default(),
				reason: BoundedVec::try_from(reason.clone().into_inner()).unwrap_or_default(),
				severity,
			};

			let current_block = frame_system::Pallet::<T>::block_number();
			SecurityEvents::<T>::mutate(current_block, |events| {
				if events.len() < T::MaxSecurityEvents::get() as usize {
					let _ = events.try_push(security_event.clone());
				}
			});

			Self::deposit_event(Event::SecurityAlert { event: security_event });
			Ok(())
		}

		/// Add or update peer information
		#[pallet::weight(T::WeightInfo::update_peer_info())]
		#[pallet::call_index(2)]
		pub fn update_peer_info(
			origin: OriginFor<T>,
			peer_id: BoundedVec<u8, T::MaxPeers>,
			reputation: u32,
		) -> DispatchResult {
			ensure_signed(origin)?;

			ensure!(reputation <= 100, Error::<T>::InvalidPeerData);

			let current_block = frame_system::Pallet::<T>::block_number();
			let timestamp = T::UnixTime::now().as_millis() as u64;

			let peer_info = PeerInfo {
				peer_id: peer_id.clone(),
				reputation,
				last_seen: current_block,
				misbehavior_count: 0,
				connected_at: timestamp,
			};

			ConnectedPeers::<T>::insert(&peer_id, peer_info);
			Ok(())
		}

		/// Whitelist a previously blacklisted peer
		#[pallet::weight(T::WeightInfo::whitelist_peer())]
		#[pallet::call_index(3)]
		pub fn whitelist_peer(
			origin: OriginFor<T>,
			peer_id: BoundedVec<u8, T::MaxPeers>,
		) -> DispatchResult {
			ensure_root(origin)?;

			ensure!(BlacklistedPeers::<T>::contains_key(&peer_id), Error::<T>::PeerNotFound);

			BlacklistedPeers::<T>::remove(&peer_id);

			// Reset peer reputation to neutral
			ConnectedPeers::<T>::mutate(&peer_id, |peer_info| {
				if let Some(info) = peer_info {
					info.reputation = 50; // Neutral reputation
					info.misbehavior_count = 0;
				}
			});

			Self::deposit_event(Event::PeerWhitelisted { peer_id });
			Ok(())
		}

		/// Detect and report network attacks
		#[pallet::weight(T::WeightInfo::detect_network_attacks())]
		#[pallet::call_index(4)]
		pub fn detect_network_attacks(origin: OriginFor<T>) -> DispatchResult {
			ensure_signed(origin)?;

			let peer_count = ConnectedPeers::<T>::iter().count() as u32;
			let _stats = NetworkStats::<T>::get();

			// DDoS detection
			if peer_count > T::MaxPeerCount::get() {
				let attack_event = SecurityEvent::DDoSAttack {
					connection_rate: peer_count,
					source_ips: BoundedVec::default(), // Would be populated with actual IPs
				};

				Self::deposit_event(Event::AttackDetected {
					attack_type: AttackType::DDoS,
					severity: SecuritySeverity::High,
					peer_count,
				});

				Self::deposit_event(Event::SecurityAlert { event: attack_event });
			}

			// Eclipse attack detection
			if peer_count < T::MinPeerCount::get() && peer_count > 0 {
				let attack_event = SecurityEvent::EclipseAttack {
					peer_count,
					isolated_duration: 0, // Would calculate actual duration
				};

				Self::deposit_event(Event::AttackDetected {
					attack_type: AttackType::Eclipse,
					severity: SecuritySeverity::Critical,
					peer_count,
				});

				Self::deposit_event(Event::SecurityAlert { event: attack_event });
			}

			Ok(())
		}

		/// Clean up old security events and peer data
		#[pallet::weight(T::WeightInfo::cleanup_old_data())]
		#[pallet::call_index(5)]
		pub fn cleanup_old_data(
			origin: OriginFor<T>,
			blocks_to_keep: BlockNumberFor<T>,
		) -> DispatchResult {
			ensure_root(origin)?;

			let current_block = frame_system::Pallet::<T>::block_number();
			let cutoff_block = current_block.saturating_sub(blocks_to_keep);

			// Remove old security events
			// Note: In a real implementation, you'd need to track block numbers
			// and remove them individually or use a different storage approach

			// Remove inactive peers (not seen for a while)
			let peers_to_remove: Vec<BoundedVec<u8, T::MaxPeers>> = ConnectedPeers::<T>::iter()
				.filter(|(_, peer_info)| peer_info.last_seen < cutoff_block)
				.map(|(peer_id, _)| peer_id)
				.collect();

			for peer_id in peers_to_remove {
				ConnectedPeers::<T>::remove(&peer_id);
			}

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Calculate current network health status
		pub fn calculate_network_health() -> NetworkHealthStatus {
			let peer_count = ConnectedPeers::<T>::iter().count() as u32;
			let block_rate = Self::calculate_block_rate();
			let consensus_rate = Self::calculate_consensus_rate();
			let security_score = Self::calculate_security_score();
			let timestamp = T::UnixTime::now().as_millis() as u64;

			NetworkHealthStatus {
				peer_count,
				block_rate,
				consensus_rate,
				security_score,
				last_updated: timestamp,
			}
		}

		/// Calculate block production rate
		fn calculate_block_rate() -> u32 {
			// In a real implementation, this would calculate actual block rate
			// based on recent block production timestamps
			60 // Placeholder: 60 blocks per minute
		}

		/// Calculate consensus participation rate
		fn calculate_consensus_rate() -> u32 {
			let stats = NetworkStats::<T>::get();
			if stats.consensus_failures == 0 {
				100
			} else {
				// Calculate based on failures vs successes
				100u32.saturating_sub(stats.consensus_failures.min(100))
			}
		}

		/// Calculate overall security score
		fn calculate_security_score() -> u32 {
			let peer_count = ConnectedPeers::<T>::iter().count() as u32;

			if peer_count == 0 {
				return 0;
			}

			// Calculate based on peer reputation and network health
			let total_reputation: u32 =
				ConnectedPeers::<T>::iter().map(|(_, peer_info)| peer_info.reputation).sum();

			let avg_reputation = total_reputation / peer_count;

			// Adjust score based on peer count and reputation
			let mut score = avg_reputation;

			// Penalty for too few peers (eclipse attack risk)
			if peer_count < T::MinPeerCount::get() {
				score = score.saturating_sub(30);
			}

			// Penalty for too many peers (DDoS risk)
			if peer_count > T::MaxPeerCount::get() {
				score = score.saturating_sub(20);
			}

			// Check for recent security events
			let current_block = frame_system::Pallet::<T>::block_number();
			let recent_events = SecurityEvents::<T>::get(current_block);
			if !recent_events.is_empty() {
				score = score.saturating_sub(recent_events.len() as u32 * 5);
			}

			score.min(100)
		}

		/// Check if a peer is blacklisted
		pub fn is_peer_blacklisted(peer_id: &BoundedVec<u8, T::MaxPeers>) -> bool {
			BlacklistedPeers::<T>::contains_key(peer_id)
		}

		/// Get peer reputation score
		pub fn get_peer_reputation(peer_id: &BoundedVec<u8, T::MaxPeers>) -> Option<u32> {
			ConnectedPeers::<T>::get(peer_id).map(|info| info.reputation)
		}

		/// Update network statistics
		pub fn update_network_statistics() {
			let timestamp = T::UnixTime::now().as_millis() as u64;
			NetworkStats::<T>::mutate(|stats| {
				stats.last_stats_update = timestamp;
				// Update other statistics as needed
			});
		}
	}
}

/// Genesis configuration for the network monitor pallet
#[derive(Default)]
pub struct GenesisConfig<T: Config> {
	pub initial_peers: Vec<(BoundedVec<u8, T::MaxPeers>, u32)>, // (peer_id, reputation)
	pub _phantom: sp_std::marker::PhantomData<T>,
}

#[cfg(feature = "std")]
impl<T: Config> GenesisConfig<T> {
	pub fn build(&self) {
		for (peer_id, reputation) in &self.initial_peers {
			let current_block = frame_system::Pallet::<T>::block_number();
			let timestamp = T::UnixTime::now().as_millis() as u64;

			let peer_info = PeerInfo {
				peer_id: peer_id.clone(),
				reputation: *reputation,
				last_seen: current_block,
				misbehavior_count: 0,
				connected_at: timestamp,
			};

			ConnectedPeers::<T>::insert(peer_id, peer_info);
		}
	}
}
