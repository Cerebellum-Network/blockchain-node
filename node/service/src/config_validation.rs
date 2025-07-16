//! Configuration validation module for Cere blockchain node
//! Phase 3: Infrastructure & Configuration Hardening

use jsonschema::JSONSchema;
use serde_json::Value;
use std::fmt;

/// Configuration validation errors
#[derive(Debug)]
pub enum ValidationError {
	/// Invalid configuration format
	InvalidConfiguration,
	/// JSON parsing error
	JsonParseError(serde_json::Error),
	/// Schema compilation error
	SchemaCompilationError(String),
	/// Validation failed with specific errors
	ValidationFailed(Vec<String>),
	/// IO error when reading files
	IoError(std::io::Error),
}

impl fmt::Display for ValidationError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			ValidationError::InvalidConfiguration => {
				write!(f, "Invalid configuration format")
			},
			ValidationError::JsonParseError(e) => {
				write!(f, "JSON parsing error: {}", e)
			},
			ValidationError::SchemaCompilationError(e) => {
				write!(f, "Schema compilation error: {}", e)
			},
			ValidationError::ValidationFailed(errors) => {
				write!(f, "Validation failed: {}", errors.join(", "))
			},
			ValidationError::IoError(e) => {
				write!(f, "IO error: {}", e)
			},
		}
	}
}

impl std::error::Error for ValidationError {}

impl From<serde_json::Error> for ValidationError {
	fn from(error: serde_json::Error) -> Self {
		ValidationError::JsonParseError(error)
	}
}

impl From<std::io::Error> for ValidationError {
	fn from(error: std::io::Error) -> Self {
		ValidationError::IoError(error)
	}
}

/// Validates a chain specification against the JSON schema
pub fn validate_chain_spec(spec: &Value) -> Result<(), ValidationError> {
	let schema_str = include_str!("../../../schemas/chain-spec.schema.json");
	let schema: Value = serde_json::from_str(schema_str)?;

	let compiled = JSONSchema::compile(&schema)
		.map_err(|e| ValidationError::SchemaCompilationError(e.to_string()))?;

	let validation_result = compiled.validate(spec);

	if let Err(errors) = validation_result {
		let error_messages: Vec<String> =
			errors.map(|e| format!("{} at {}", e, e.instance_path)).collect();

		if !error_messages.is_empty() {
			return Err(ValidationError::ValidationFailed(error_messages));
		}
	}

	Ok(())
}

/// Validates chain specification from a JSON string
pub fn validate_chain_spec_from_str(spec_json: &str) -> Result<(), ValidationError> {
	let spec: Value = serde_json::from_str(spec_json)?;
	validate_chain_spec(&spec)
}

/// Validates chain specification from a file path
pub fn validate_chain_spec_from_file(file_path: &str) -> Result<(), ValidationError> {
	let content = std::fs::read_to_string(file_path)?;
	validate_chain_spec_from_str(&content)
}

/// Validates runtime configuration parameters
pub fn validate_runtime_config(spec: &Value) -> Result<(), ValidationError> {
	// Validate basic structure
	validate_chain_spec(spec)?;

	// Additional runtime-specific validations
	validate_network_security(spec)?;
	validate_consensus_parameters(spec)?;
	validate_token_economics(spec)?;

	Ok(())
}

/// Validates network security configuration
fn validate_network_security(spec: &Value) -> Result<(), ValidationError> {
	let mut errors = Vec::new();

	// Check boot nodes are not using insecure protocols
	if let Some(boot_nodes) = spec.get("bootNodes").and_then(|v| v.as_array()) {
		for boot_node in boot_nodes {
			if let Some(addr) = boot_node.as_str() {
				if addr.contains("127.0.0.1") || addr.contains("localhost") {
					if let Some(chain_type) = spec.get("chainType").and_then(|v| v.as_str()) {
						if chain_type != "Development" && chain_type != "Local" {
							errors.push(format!(
								"Boot node uses localhost address in {} network: {}",
								chain_type, addr
							));
						}
					}
				}
			}
		}
	}

	// Check telemetry endpoints use secure protocols
	if let Some(telemetry) = spec.get("telemetryEndpoints").and_then(|v| v.as_array()) {
		for endpoint in telemetry {
			if let Some(endpoint_array) = endpoint.as_array() {
				if let Some(url) = endpoint_array.first().and_then(|v| v.as_str()) {
					if !url.starts_with("wss://") && !url.starts_with("ws://localhost") {
						errors.push(format!(
							"Telemetry endpoint should use secure WebSocket (wss://): {}",
							url
						));
					}
				}
			}
		}
	}

	if !errors.is_empty() {
		return Err(ValidationError::ValidationFailed(errors));
	}

	Ok(())
}

/// Validates consensus parameters
fn validate_consensus_parameters(spec: &Value) -> Result<(), ValidationError> {
	let mut errors = Vec::new();

	// Check for reasonable genesis configuration
	if let Some(genesis) = spec.get("genesis").and_then(|v| v.as_object()) {
		if let Some(raw) = genesis.get("raw").and_then(|v| v.as_object()) {
			if let Some(top) = raw.get("top").and_then(|v| v.as_object()) {
				// Validate that essential pallets are configured
				let required_prefixes = [
					"0x26aa394eea5630e07c48ae0c9558cef7",         // System
					"0x5f3e4907f716ac89b6347d15ececedca",         // Balances
					"0x3a6772616e6470615f617574686f726974696573", // Grandpa authorities
				];

				for prefix in &required_prefixes {
					let has_key = top.keys().any(|k| k.starts_with(prefix));
					if !has_key {
						errors.push(format!(
							"Missing required genesis configuration for prefix: {}",
							prefix
						));
					}
				}
			}
		}
	}

	if !errors.is_empty() {
		return Err(ValidationError::ValidationFailed(errors));
	}

	Ok(())
}

/// Validates token economics configuration
fn validate_token_economics(spec: &Value) -> Result<(), ValidationError> {
	let mut errors = Vec::new();

	if let Some(properties) = spec.get("properties").and_then(|v| v.as_object()) {
		// Validate token decimals
		if let Some(decimals) = properties.get("tokenDecimals").and_then(|v| v.as_u64()) {
			if decimals > 18 {
				errors.push(format!(
					"Token decimals too high ({}), maximum recommended is 18",
					decimals
				));
			}
		}

		// Validate token symbol
		if let Some(symbol) = properties.get("tokenSymbol").and_then(|v| v.as_str()) {
			if symbol.len() > 10 {
				errors.push(format!(
					"Token symbol too long ({}), maximum recommended is 10 characters",
					symbol.len()
				));
			}

			if !symbol.chars().all(|c| c.is_ascii_uppercase()) {
				errors.push(format!("Token symbol should be uppercase ASCII: {}", symbol));
			}
		}

		// Validate SS58 format
		if let Some(ss58_format) = properties.get("ss58Format").and_then(|v| v.as_u64()) {
			if ss58_format > 16383 {
				errors
					.push(format!("SS58 format out of range ({}), maximum is 16383", ss58_format));
			}
		}
	}

	if !errors.is_empty() {
		return Err(ValidationError::ValidationFailed(errors));
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;

	#[test]
	fn test_validate_runtime_config() {
		let valid_spec = json!({
			"name": "Test Chain",
			"id": "test_chain",
			"chainType": "Development",
			"bootNodes": [],
			"telemetryEndpoints": [],
			"protocolId": null,
			"properties": {
				"ss58Format": 42,
				"tokenDecimals": 12,
				"tokenSymbol": "TEST"
			},
			"forkBlocks": null,
			"badBlocks": null,
			"consensusEngine": null,
			"lightSyncState": null,
			"genesis": {
				"raw": {
					"top": {
						"0x26aa394eea5630e07c48ae0c9558cef7": "0x00",
						"0x5f3e4907f716ac89b6347d15ececedca": "0x00",
						"0x3a6772616e6470615f617574686f726974696573": "0x00"
					}
				}
			}
		});

		assert!(validate_runtime_config(&valid_spec).is_ok());
	}

	#[test]
	fn test_validate_chain_spec_invalid() {
		let invalid_spec = json!({
			"name": "", // Empty name should fail
			"id": "test",
			"chainType": "InvalidType" // Invalid chain type
		});

		assert!(validate_chain_spec(&invalid_spec).is_err());
	}

	#[test]
	fn test_validate_network_security() {
		let insecure_spec = json!({
			"name": "Test Chain",
			"id": "test_chain",
			"chainType": "Live",
			"bootNodes": ["/ip4/127.0.0.1/tcp/30333/p2p/test"],
			"telemetryEndpoints": [["ws://insecure.example.com", 0]],
			"properties": {
				"ss58Format": 42,
				"tokenDecimals": 12,
				"tokenSymbol": "TEST"
			},
			"genesis": {
				"raw": {
					"top": {}
				}
			}
		});

		assert!(validate_network_security(&insecure_spec).is_err());
	}

	#[test]
	fn test_validate_token_economics() {
		let invalid_token_spec = json!({
			"properties": {
				"ss58Format": 99999, // Too high
				"tokenDecimals": 25, // Too high
				"tokenSymbol": "toolongSymbol" // Too long
			}
		});

		assert!(validate_token_economics(&invalid_token_spec).is_err());
	}
}
