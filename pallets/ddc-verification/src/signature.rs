use prost::Message;
use sp_core::ed25519::{Public, Signature};
use sp_io::crypto::ed25519_verify;

use super::*;

pub trait Verify {
	fn verify(&self) -> bool;
}

impl Verify for proto::ActivityAcknowledgment {
	fn verify(&self) -> bool {
		verify_signature(self.clone())
	}
}

impl Verify for proto::ActivityRecord {
	fn verify(&self) -> bool {
		if !verify_signature(self.clone()) {
			return false;
		}

		for downstream in &self.downstream {
			if !downstream.verify() {
				return false;
			}
		}

		if let Some(upstream) = &self.upstream {
			if !upstream.verify() {
				return false;
			}
		}

		true
	}
}

impl Verify for proto::ActivityRequest {
	fn verify(&self) -> bool {
		if !verify_signature(self.clone()) {
			return false;
		}

		// TODO(khssnv): parent requests are expected to have an invalid signature.
		// if let Some(ref parent_request) = self.parent_request {
		// 	if !parent_request.verify() {
		// 		return false;
		// 	}
		// }

		true
	}
}

impl Verify for proto::ActivityFulfillment {
	fn verify(&self) -> bool {
		if let Some(request) = &self.request {
			if !request.verify() {
				return false;
			}
		}

		if let Some(ack) = &self.ack {
			if !ack.verify() {
				return false;
			}
		}

		true
	}
}

impl Verify for proto::challenge_response::proof::Record {
	fn verify(&self) -> bool {
		if let Some(record) = &self.record {
			return record.verify();
		}

		true
	}
}

impl Verify for proto::ChallengeResponse {
	fn verify(&self) -> bool {
		for proof in self.proofs.iter() {
			for leaf in proof.leaves.iter() {
				if let Some(leaf) = &leaf.leaf_variant {
					match leaf {
						proto::challenge_response::proof::leaf::LeafVariant::Record(record) =>
							if !record.verify() {
								return false;
							},
						_ => {},
					}
				}
			}
		}

		true
	}
}


trait Signed {
	fn get_signature(&self) -> Option<&proto::Signature>;
	fn reset_signature(&mut self);
}

/// Implements Signed trait for given types.
macro_rules! impl_signed {
	(for $($t:ty),+) => {
		$(impl Signed for $t {
			fn get_signature(&self) -> Option<&proto::Signature> {
				return self.signature.as_ref()
			}

			fn reset_signature(&mut self) {
				self.signature = None;
			}
		})*
	}
}

impl_signed!(for proto::ActivityAcknowledgment, proto::ActivityRecord, proto::ActivityRequest);

fn verify_signature(signed: impl Clone + Message + Signed) -> bool {
	let signature = match signed.get_signature() {
		Some(s) => s.clone(),
		None => return false,
	};
	let sig = match Signature::try_from(signature.clone().value.as_slice()) {
		Ok(s) => s,
		Err(_) => return false,
	};

	let mut msg = signed.clone();
	msg.reset_signature();
	let payload = msg.encode_to_vec();

	let pub_key = match Public::try_from(signature.clone().signer.as_slice()) {
		Ok(p) => p,
		Err(_) => return false,
	};

	ed25519_verify(&sig, payload.as_slice(), &pub_key)
}
