use sp_runtime::ArithmeticError;
use sp_std::{collections::btree_map::BTreeMap, vec::Vec};

pub fn get_node_id(level: u32, node_idx: u64) -> u64 {
	2_u64.pow(level) + node_idx
}

pub fn get_leaves_ids(leaves_count: u64) -> Vec<u64> {
	let levels = get_levels(leaves_count).expect("Tree levels to be defined");
	let leaf_level = levels.last().expect("Leaf level to be defined");
	let mut leaf_ids = Vec::new();
	for leaf_idx in 0..leaves_count {
		leaf_ids.push(get_node_id(*leaf_level, leaf_idx));
	}
	leaf_ids
}

pub fn get_levels(leaves_count: u64) -> Result<Vec<u32>, &'static str> {
	if leaves_count == 0 {
		return Err("Tree must have at least one leaf.");
	}

	// The last level is based on the log2 calculation
	let last_level = leaves_count.ilog2();

	// If the tree is not a perfect binary tree, we need one more level
	if (1 << last_level) < leaves_count {
		return Ok((0..=last_level + 1).collect());
	}

	// If the tree is a perfect binary tree
	Ok((0..=last_level).collect())
}

pub const D_099: u64 = 9900; // 0.99 scaled by 10^4
pub const P_001: u64 = 100; // 0.01 scaled by 10^4

pub fn get_n0_values() -> BTreeMap<(u64, u64), u64> {
	let mut n0_map = BTreeMap::new();

	// Formula: n0 = ln(1 - d) / ln(1 - p)

	// todo: add more pairs
	n0_map.insert((D_099, P_001), 4582106); // ~458.2106 scaled by 10^4

	n0_map
}

pub fn calculate_sample_size_inf(d: u64, p: u64) -> u64 {
	let n0_values = get_n0_values();

	let n0 = n0_values.get(&(d, p)).expect("Sample size to be defined");
	*n0
}

pub fn calculate_sample_size_fin(
	n0_scaled: u64,
	population_size: u64,
) -> Result<u64, ArithmeticError> {
	const SCALE: u64 = 10000; // Reduced scale factor for fixed-point precision (10^4)

	// Ensure that population_size > 1 to avoid division by zero or invalid calculations
	if population_size <= 1 {
		return Err(ArithmeticError::Underflow);
	}

	// Formula: n = n_0 / (1 + (n_0 - 1) / N)

	// First calculate (n0 - 1)
	let numerator = n0_scaled
		.checked_sub(SCALE) // subtract 1 * SCALE to keep consistent scaling
		.ok_or(ArithmeticError::Underflow)?;

	// Scale the numerator before dividing to increase precision
	let numerator_scaled = numerator.checked_mul(SCALE).ok_or(ArithmeticError::Overflow)?;

	// Calculate (n0 - 1) / N using integer division, but scale numerator first
	// Population size is unscaled, so we scale it before the division to maintain precision
	let population_scaled = population_size.checked_mul(SCALE).ok_or(ArithmeticError::Overflow)?;

	let correction_term = numerator_scaled
		.checked_div(population_scaled)
		.ok_or(ArithmeticError::Underflow)?;

	// Now calculate 1 + correction_term, but scale 1 by SCALE to ensure consistency
	let denominator = SCALE.checked_add(correction_term).ok_or(ArithmeticError::Overflow)?; // SCALE is used to scale the 1

	// Finally calculate n = n0 / denominator, and return the result (floored automatically by
	// integer division)
	let n = n0_scaled.checked_div(denominator).ok_or(ArithmeticError::Underflow)?;

	Ok(n)
}
