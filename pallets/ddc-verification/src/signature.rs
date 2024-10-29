use prost::Message;
use sp_core::ed25519::{Public, Signature};
use sp_io::crypto::ed25519_verify;

use super::*;

pub trait Verify {
	fn verify(&self) -> bool;
}

impl Verify for proto::ActivityAcknowledgment {
	fn verify(&self) -> bool {
		let signature = match &self.signature {
			Some(s) => s.clone(),
			None => return false,
		};
		let sig = match Signature::try_from(signature.clone().value.as_slice()) {
			Ok(s) => s,
			Err(_) => return false,
		};

		let mut msg = self.clone();
		msg.signature = None;
		let payload = msg.encode_to_vec();

		let pub_key = match Public::try_from(signature.clone().signer.as_slice()) {
			Ok(p) => p,
			Err(_) => return false,
		};

		if !ed25519_verify(&sig, payload.as_slice(), &pub_key) {
			return false;
		}

		true
	}
}

impl Verify for proto::ActivityRecord {
	fn verify(&self) -> bool {
		let signature = match &self.signature {
			Some(s) => s.clone(),
			None => return false,
		};
		let sig = match Signature::try_from(signature.clone().value.as_slice()) {
			Ok(s) => s,
			Err(_) => return false,
		};

		let mut msg = self.clone();
		msg.signature = None;
		let payload = msg.encode_to_vec();

		let pub_key = match Public::try_from(signature.clone().signer.as_slice()) {
			Ok(p) => p,
			Err(_) => return false,
		};

		if !ed25519_verify(&sig, payload.as_slice(), &pub_key) {
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
		let signature = match &self.signature {
			Some(s) => s.clone(),
			None => return false,
		};
		let sig = match Signature::try_from(signature.clone().value.as_slice()) {
			Ok(s) => s,
			Err(_) => return false,
		};

		let mut msg = self.clone();
		msg.signature = None;
		let payload = msg.encode_to_vec();

		let pub_key = match Public::try_from(signature.clone().signer.as_slice()) {
			Ok(p) => p,
			Err(_) => return false,
		};

		if !ed25519_verify(&sig, payload.as_slice(), &pub_key) {
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
