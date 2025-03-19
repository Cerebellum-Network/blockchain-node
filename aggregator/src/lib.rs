#![cfg_attr(not(feature = "std"), no_std)]

pub mod aggregator;
pub mod signature;

pub mod insp_ddc_api;

pub mod proto {
	include!(concat!(env!("OUT_DIR"), "/activity.rs"));
}
