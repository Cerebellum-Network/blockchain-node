use aggregator_client::json;
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
				if let Some(proto::challenge_response::proof::leaf::LeafVariant::Record(record)) =
					&leaf.leaf_variant
				{
					if !record.verify() {
						return false;
					}
				}
			}
		}

		true
	}
}

impl<T: Serialize> Verify for json::SignedJsonResponse<T> {
	fn verify(&self) -> bool {
		let sig = match Signature::try_from(self.signature.as_slice()) {
			Ok(s) => s,
			Err(_) => return false,
		};

		let payload = match serde_json::to_vec(&self.payload) {
			Ok(p) => p,
			Err(_) => return false,
		};

		let pub_key = match Public::try_from(self.signer.as_slice()) {
			Ok(p) => p,
			Err(_) => return false,
		};

		ed25519_verify(&sig, payload.as_slice(), &pub_key)
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

fn verify_signature(mut signed: impl Clone + Message + Signed) -> bool {
	let signature = match signed.get_signature() {
		Some(s) => s.clone(),
		None => return false,
	};
	let sig = match Signature::try_from(signature.value.as_slice()) {
		Ok(s) => s,
		Err(_) => return false,
	};

	signed.reset_signature();
	let payload = signed.encode_to_vec();

	let pub_key = match Public::try_from(signature.signer.as_slice()) {
		Ok(p) => p,
		Err(_) => return false,
	};

	ed25519_verify(&sig, payload.as_slice(), &pub_key)
}

#[cfg(test)]
mod tests {
	use sp_core::Pair;

	use super::*;

	#[test]
	fn verify_signature_works() {
		#[derive(Clone, PartialEq, ::prost::Message)]
		pub struct SignedProtoMsg {
			#[prost(string, tag = "1")]
			pub foo: ::prost::alloc::string::String,
			#[prost(message, optional, tag = "2")]
			pub signature: ::core::option::Option<proto::Signature>,
		}
		impl_signed!(for SignedProtoMsg);

		let none_signature_msg =
			SignedProtoMsg { foo: "none_signature_msg".to_string(), signature: None };
		assert!(!verify_signature(none_signature_msg));

		let mut invalid_signature_msg =
			SignedProtoMsg { foo: "invalid_signature_msg".to_string(), signature: None };
		let invalid_signature_msg_signer = sp_core::ed25519::Pair::generate().0;
		let invalid_signature_msg_signature =
			invalid_signature_msg_signer.sign(invalid_signature_msg.encode_to_vec().as_slice());
		let mut invalid_signature_msg_signature_vec = invalid_signature_msg_signature.0.to_vec();
		invalid_signature_msg_signature_vec[0] =
			invalid_signature_msg_signature_vec[0].wrapping_add(1);
		invalid_signature_msg.signature = Some(proto::Signature {
			algorithm: proto::signature::Algorithm::Ed25519 as i32,
			value: invalid_signature_msg_signature_vec,
			signer: invalid_signature_msg_signer.public().0.to_vec(),
		});
		assert!(!verify_signature(invalid_signature_msg));

		let mut valid_signature_msg =
			SignedProtoMsg { foo: "valid_signature_msg".to_string(), signature: None };
		let valid_signature_msg_signer = sp_core::ed25519::Pair::generate().0;
		let valid_signature_msg_signature =
			valid_signature_msg_signer.sign(valid_signature_msg.encode_to_vec().as_slice());
		valid_signature_msg.signature = Some(proto::Signature {
			algorithm: proto::signature::Algorithm::Ed25519 as i32,
			value: valid_signature_msg_signature.0.to_vec(),
			signer: valid_signature_msg_signer.public().0.to_vec(),
		});
		assert!(verify_signature(valid_signature_msg));
	}

	#[test]
	fn verify_challenge_response_works() {
		let challenge_response_serialized =
			include_bytes!("./test_data/challenge_response.pb").as_slice();
		let challenge_response = proto::ChallengeResponse::decode(challenge_response_serialized)
			.expect("protobuf fixture decoding failed, fix the test data");
		assert!(challenge_response.verify());
	}
}
