use jsonschema::{JSONSchema, ValidationError};
use serde_json::Value;
use std::{fs, path::Path};

/// Configuration validator for chain specifications
pub struct ConfigValidator {
    schema: JSONSchema,
}

impl ConfigValidator {
    /// Create a new config validator with the embedded schema
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let schema_content = include_str!("../../../schemas/chain-spec.schema.json");
        let schema: Value = serde_json::from_str(schema_content)?;
        let compiled = JSONSchema::compile(&schema)?;
        
        Ok(ConfigValidator { schema: compiled })
    }
    
    /// Validate a chain specification file
    pub fn validate_chain_spec(&self, spec_path: &str) -> Result<(), ValidationError> {
        let content = fs::read_to_string(spec_path)
            .map_err(|e| ValidationError::new(&format!("Failed to read file: {}", e)))?;
        
        let spec: Value = serde_json::from_str(&content)
            .map_err(|e| ValidationError::new(&format!("Invalid JSON: {}", e)))?;
        
        // Schema validation
        if let Err(errors) = self.schema.validate(&spec) {
            for error in errors {
                eprintln!("Schema validation error: {}", error);
            }
            return Err(ValidationError::new("Chain spec schema validation failed"));
        }
        
        // Additional security validations
        self.validate_security_constraints(&spec)?;
        
        // Environment-specific validations
        self.validate_environment_constraints(&spec)?;
        
        Ok(())
    }
    
    /// Validate security constraints
    fn validate_security_constraints(&self, spec: &Value) -> Result<(), ValidationError> {
        // Check balance distributions
        if let Some(balances) = spec.pointer("/genesis/runtime/balances/balances") {
            if let Some(balance_array) = balances.as_array() {
                let mut total_balance = 0u64;
                
                for balance in balance_array {
                    if let Some(amount) = balance.get(1) {
                        if let Some(amount_num) = amount.as_u64() {
                            // Check for excessive individual balance
                            if amount_num > 1_000_000_000_000_000 {
                                return Err(ValidationError::new(
                                    "Individual balance exceeds maximum allowed (1M CERE)"
                                ));
                            }
                            total_balance += amount_num;
                        }
                    }
                }
                
                // Check total supply
                if total_balance > 10_000_000_000_000_000 {
                    return Err(ValidationError::new(
                        "Total supply exceeds maximum allowed (10M CERE)"
                    ));
                }
            }
        }
        
        // Validate sudo key security
        if let Some(sudo_key) = spec.pointer("/genesis/runtime/sudo/key") {
            if let Some(key_str) = sudo_key.as_str() {
                let dev_keys = [
                    "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY", // Alice
                    "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty", // Bob
                    "5GNJqTPyNqANBkUVMN1LPPrxXnFouWXoe2wNSmmEoLctxiZY", // Charlie
                    "5HpG9w8EBLe5XCrbczpwq5TSXvedjrBGCwqxK1iQ7qUsSWFc", // Dave
                    "5FLSigC9HGRKVhB9FiEo4Y3koPsNmBmLJbpXg2mp1hXcS59Y", // Eve
                ];
                
                let chain_type = spec.pointer("/chainType").and_then(|v| v.as_str());
                if chain_type != Some("Development") && dev_keys.contains(&key_str) {
                    return Err(ValidationError::new(
                        "Development key used in non-development chain"
                    ));
                }
            }
        }
        
        // Validate session keys
        if let Some(session_keys) = spec.pointer("/genesis/runtime/session/keys") {
            if let Some(keys_array) = session_keys.as_array() {
                for key_set in keys_array {
                    if let Some(key_array) = key_set.as_array() {
                        if key_array.len() >= 3 {
                            if let Some(session_keys_obj) = key_array.get(2) {
                                self.validate_session_key_security(session_keys_obj, spec)?;
                            }
                        }
                    }
                }
            }
        }
        
        // Validate validator configuration
        self.validate_validator_configuration(spec)?;
        
        Ok(())
    }
    
    /// Validate session key security
    fn validate_session_key_security(&self, session_keys: &Value, spec: &Value) -> Result<(), ValidationError> {
        let chain_type = spec.pointer("/chainType").and_then(|v| v.as_str());
        
        if chain_type != Some("Development") {
            let required_keys = ["grandpa", "babe", "im_online", "authority_discovery"];
            
            for key_type in required_keys {
                if let Some(key_value) = session_keys.pointer(&format!("/{}", key_type)) {
                    if let Some(key_str) = key_value.as_str() {
                        // Check for development keys in production
                        if key_str.starts_with("5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY") {
                            return Err(ValidationError::new(
                                &format!("Development {} key used in non-development chain", key_type)
                            ));
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Validate validator configuration
    fn validate_validator_configuration(&self, spec: &Value) -> Result<(), ValidationError> {
        if let Some(staking) = spec.pointer("/genesis/runtime/staking") {
            let validator_count = staking.pointer("/validatorCount").and_then(|v| v.as_u64());
            let min_validator_count = staking.pointer("/minimumValidatorCount").and_then(|v| v.as_u64());
            let invulnerables = staking.pointer("/invulnerables").and_then(|v| v.as_array());
            
            if let (Some(count), Some(min_count)) = (validator_count, min_validator_count) {
                if count < min_count {
                    return Err(ValidationError::new(
                        "Validator count less than minimum validator count"
                    ));
                }
                
                // Check for reasonable validator limits
                if count > 1000 {
                    return Err(ValidationError::new(
                        "Validator count exceeds reasonable limit (1000)"
                    ));
                }
                
                if min_count == 0 {
                    return Err(ValidationError::new(
                        "Minimum validator count cannot be zero"
                    ));
                }
            }
            
            // Validate invulnerables
            if let Some(invulnerables_array) = invulnerables {
                if invulnerables_array.len() > 100 {
                    return Err(ValidationError::new(
                        "Too many invulnerable validators (max 100)"
                    ));
                }
            }
        }
        
        Ok(())
    }
    
    /// Validate environment-specific constraints
    fn validate_environment_constraints(&self, spec: &Value) -> Result<(), ValidationError> {
        let chain_type = spec.pointer("/chainType").and_then(|v| v.as_str());
        let chain_id = spec.pointer("/id").and_then(|v| v.as_str());
        
        match chain_type {
            Some("Development") => {
                // Development chains must have _dev suffix
                if let Some(id) = chain_id {
                    if !id.ends_with("_dev") {
                        return Err(ValidationError::new(
                            "Development chain ID must end with '_dev'"
                        ));
                    }
                }
            },
            Some("Local") => {
                // Local chains must have _local suffix
                if let Some(id) = chain_id {
                    if !id.ends_with("_local") {
                        return Err(ValidationError::new(
                            "Local chain ID must end with '_local'"
                        ));
                    }
                }
            },
            Some("Live") => {
                // Live chains must not have development suffixes
                if let Some(id) = chain_id {
                    if id.ends_with("_dev") || id.ends_with("_local") {
                        return Err(ValidationError::new(
                            "Live chain ID cannot end with '_dev' or '_local'"
                        ));
                    }
                }
            },
            _ => {}
        }
        
        Ok(())
    }
    
    /// Validate runtime configuration
    pub fn validate_runtime_config(&self, spec: &Value) -> Result<(), ValidationError> {
        // Check for required runtime modules
        let required_modules = [
            "system", "balances", "sudo", "session", "staking",
            "ddcStaking", "ddcCustomers", "ddcNodes", "ddcClusters"
        ];
        
        for module in required_modules {
            if spec.pointer(&format!("/genesis/runtime/{}", module)).is_none() {
                return Err(ValidationError::new(
                    &format!("Required runtime module '{}' not found", module)
                ));
            }
        }
        
        // Validate DDC-specific configurations
        self.validate_ddc_configuration(spec)?;
        
        Ok(())
    }
    
    /// Validate DDC-specific configuration
    fn validate_ddc_configuration(&self, spec: &Value) -> Result<(), ValidationError> {
        // Validate DDC nodes configuration
        if let Some(ddc_nodes) = spec.pointer("/genesis/runtime/ddcNodes/storageNodes") {
            if let Some(nodes_array) = ddc_nodes.as_array() {
                for node in nodes_array {
                    if let Some(props) = node.pointer("/props") {
                        // Validate port ranges
                        let http_port = props.pointer("/http_port").and_then(|v| v.as_u64());
                        let grpc_port = props.pointer("/grpc_port").and_then(|v| v.as_u64());
                        let p2p_port = props.pointer("/p2p_port").and_then(|v| v.as_u64());
                        
                        if let Some(port) = http_port {
                            if port < 1024 || port > 65535 {
                                return Err(ValidationError::new(
                                    "HTTP port must be between 1024 and 65535"
                                ));
                            }
                        }
                        
                        if let Some(port) = grpc_port {
                            if port < 1024 || port > 65535 {
                                return Err(ValidationError::new(
                                    "gRPC port must be between 1024 and 65535"
                                ));
                            }
                        }
                        
                        if let Some(port) = p2p_port {
                            if port < 1024 || port > 65535 {
                                return Err(ValidationError::new(
                                    "P2P port must be between 1024 and 65535"
                                ));
                            }
                        }
                        
                        // Validate host IP format
                        if let Some(host) = props.pointer("/host") {
                            if let Some(host_array) = host.as_array() {
                                if host_array.len() != 4 {
                                    return Err(ValidationError::new(
                                        "Host IP must be 4 octets"
                                    ));
                                }
                                
                                for octet in host_array {
                                    if let Some(octet_num) = octet.as_u64() {
                                        if octet_num > 255 {
                                            return Err(ValidationError::new(
                                                "IP octet must be between 0 and 255"
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
}

/// Custom validation error
#[derive(Debug)]
pub struct ValidationError {
    message: String,
}

impl ValidationError {
    pub fn new(message: &str) -> Self {
        ValidationError {
            message: message.to_string(),
        }
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Validation error: {}", self.message)
    }
}

impl std::error::Error for ValidationError {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[test]
    fn test_validator_creation() {
        let validator = ConfigValidator::new();
        assert!(validator.is_ok());
    }
    
    #[test]
    fn test_development_key_validation() {
        let validator = ConfigValidator::new().unwrap();
        
        let spec = json!({
            "chainType": "Live",
            "genesis": {
                "runtime": {
                    "sudo": {
                        "key": "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY"
                    }
                }
            }
        });
        
        let result = validator.validate_security_constraints(&spec);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_balance_validation() {
        let validator = ConfigValidator::new().unwrap();
        
        let spec = json!({
            "genesis": {
                "runtime": {
                    "balances": {
                        "balances": [
                            ["5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY", 2000000000000000]
                        ]
                    }
                }
            }
        });
        
        let result = validator.validate_security_constraints(&spec);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_environment_constraints() {
        let validator = ConfigValidator::new().unwrap();
        
        let spec = json!({
            "chainType": "Development",
            "id": "test_chain"
        });
        
        let result = validator.validate_environment_constraints(&spec);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_port_validation() {
        let validator = ConfigValidator::new().unwrap();
        
        let spec = json!({
            "genesis": {
                "runtime": {
                    "ddcNodes": {
                        "storageNodes": [{
                            "props": {
                                "http_port": 80,
                                "grpc_port": 8081,
                                "p2p_port": 8082
                            }
                        }]
                    }
                }
            }
        });
        
        let result = validator.validate_ddc_configuration(&spec);
        assert!(result.is_err());
    }
} 
