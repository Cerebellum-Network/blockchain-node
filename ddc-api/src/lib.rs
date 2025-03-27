#![cfg_attr(not(feature = "std"), no_std)]

pub mod api;
pub mod client;
pub mod signature;

pub mod json;
pub mod proto {
	include!(concat!(env!("OUT_DIR"), "/activity.rs"));
	include!(concat!(env!("OUT_DIR"), "/inspection.rs"));
}
