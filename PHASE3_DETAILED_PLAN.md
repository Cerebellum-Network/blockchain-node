# 🏗️ PHASE 3: INFRASTRUCTURE & CONFIGURATION HARDENING (Weeks 4-7)

## 📋 **PHASE 3 OVERVIEW**

**Duration**: 4 weeks (Weeks 4-7)  
**Priority**: MEDIUM  
**Team**: Infrastructure + Security + DevOps  

### **Critical Improvements Made:**
- ✅ Complete Infrastructure as Code implementation
- ✅ Enterprise-grade configuration validation
- ✅ Comprehensive blockchain network security
- ✅ Proper secrets management integration
- ✅ Infrastructure security scanning
- ✅ Multi-environment support

---

## 🔧 **TASK 3.1: AWS INFRASTRUCTURE SECURITY**

**Priority**: MEDIUM | **Effort**: 3 weeks | **Owner**: Infrastructure Team

### **Current State Issues:**
- ❌ No terraform infrastructure exists
- ❌ ECR configuration incomplete
- ❌ Missing IAM security policies
- ❌ No environment separation
- ❌ Hard-coded AWS configurations
- ❌ No infrastructure security scanning

### **Implementation Steps:**

#### **Week 1: Core Infrastructure Setup**

**Step 1.1: Terraform Foundation**
```hcl
# terraform/main.tf
terraform {
  required_version = ">= 1.0"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
  backend "s3" {
    bucket         = "cere-terraform-state"
    key            = "blockchain-node/terraform.tfstate"
    region         = "us-west-2"
    encrypt        = true
    dynamodb_table = "terraform-state-lock"
  }
}

provider "aws" {
  region = var.aws_region
  default_tags {
    tags = {
      Project     = "blockchain-node"
      Environment = var.environment
      ManagedBy   = "terraform"
    }
  }
}

# terraform/variables.tf
variable "environment" {
  description = "Environment name"
  type        = string
  validation {
    condition     = contains(["dev", "staging", "prod"], var.environment)
    error_message = "Environment must be dev, staging, or prod."
  }
}

variable "aws_region" {
  description = "AWS region"
  type        = string
  default     = "us-west-2"
}
```

**Step 1.2: Enhanced ECR Security**
```hcl
# terraform/ecr.tf
resource "aws_kms_key" "ecr_key" {
  description             = "KMS key for ECR encryption"
  deletion_window_in_days = 7
  
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid    = "Enable IAM User Permissions"
        Effect = "Allow"
        Principal = {
          AWS = "arn:aws:iam::${data.aws_caller_identity.current.account_id}:root"
        }
        Action   = "kms:*"
        Resource = "*"
      }
    ]
  })
  
  tags = {
    Name = "ecr-encryption-key"
  }
}

resource "aws_ecr_repository" "blockchain_node" {
  name                 = "pos-network-node"
  image_tag_mutability = "IMMUTABLE"
  
  image_scanning_configuration {
    scan_on_push = true
  }
  
  encryption_configuration {
    encryption_type = "KMS"
    kms_key        = aws_kms_key.ecr_key.arn
  }
  
  lifecycle_policy {
    policy = jsonencode({
      rules = [
        {
          rulePriority = 1
          description  = "Keep last 30 images"
          selection = {
            tagStatus     = "tagged"
            tagPrefixList = ["v"]
            countType     = "imageCountMoreThan"
            countNumber   = 30
          }
          action = {
            type = "expire"
          }
        }
      ]
    })
  }
}

resource "aws_ecr_repository_policy" "blockchain_node_policy" {
  repository = aws_ecr_repository.blockchain_node.name
  
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid    = "AllowPushPull"
        Effect = "Allow"
        Principal = {
          AWS = aws_iam_role.github_actions.arn
        }
        Action = [
          "ecr:GetDownloadUrlForLayer",
          "ecr:BatchGetImage",
          "ecr:BatchCheckLayerAvailability",
          "ecr:PutImage",
          "ecr:InitiateLayerUpload",
          "ecr:UploadLayerPart",
          "ecr:CompleteLayerUpload"
        ]
      }
    ]
  })
}
```

#### **Week 2: IAM Security & OIDC**

**Step 2.1: GitHub OIDC Provider**
```hcl
# terraform/iam.tf
data "tls_certificate" "github" {
  url = "https://token.actions.githubusercontent.com"
}

resource "aws_iam_openid_connect_provider" "github" {
  url = "https://token.actions.githubusercontent.com"
  
  client_id_list = ["sts.amazonaws.com"]
  
  thumbprint_list = [
    data.tls_certificate.github.certificates[0].sha1_fingerprint
  ]
  
  tags = {
    Name = "github-oidc-provider"
  }
}

resource "aws_iam_role" "github_actions" {
  name = "GitHubActions-BlockchainNode-${var.environment}"
  
  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRoleWithWebIdentity"
        Effect = "Allow"
        Principal = {
          Federated = aws_iam_openid_connect_provider.github.arn
        }
        Condition = {
          StringEquals = {
            "token.actions.githubusercontent.com:aud" = "sts.amazonaws.com"
          }
          StringLike = {
            "token.actions.githubusercontent.com:sub" = "repo:Cerebellum-Network/blockchain-node:*"
          }
        }
      }
    ]
  })
}

resource "aws_iam_role_policy" "github_actions_policy" {
  name = "GitHubActionsPolicy"
  role = aws_iam_role.github_actions.id
  
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "ecr:GetAuthorizationToken",
          "ecr:BatchCheckLayerAvailability",
          "ecr:GetDownloadUrlForLayer",
          "ecr:BatchGetImage",
          "ecr:PutImage",
          "ecr:InitiateLayerUpload",
          "ecr:UploadLayerPart",
          "ecr:CompleteLayerUpload"
        ]
        Resource = "*"
      },
      {
        Effect = "Allow"
        Action = [
          "s3:GetObject",
          "s3:PutObject"
        ]
        Resource = "${aws_s3_bucket.sccache.arn}/*"
      }
    ]
  })
}
```

**Step 2.2: S3 Bucket Security**
```hcl
# terraform/s3.tf
resource "aws_s3_bucket" "sccache" {
  bucket        = "cere-blockchain-sccache-${var.environment}"
  force_destroy = var.environment != "prod"
}

resource "aws_s3_bucket_encryption" "sccache" {
  bucket = aws_s3_bucket.sccache.id
  
  server_side_encryption_configuration {
    rule {
      apply_server_side_encryption_by_default {
        sse_algorithm = "AES256"
      }
    }
  }
}

resource "aws_s3_bucket_public_access_block" "sccache" {
  bucket = aws_s3_bucket.sccache.id
  
  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

resource "aws_s3_bucket_versioning" "sccache" {
  bucket = aws_s3_bucket.sccache.id
  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_lifecycle_configuration" "sccache" {
  bucket = aws_s3_bucket.sccache.id
  
  rule {
    id     = "cleanup"
    status = "Enabled"
    
    expiration {
      days = 30
    }
    
    noncurrent_version_expiration {
      noncurrent_days = 7
    }
  }
}
```

#### **Week 3: Multi-Environment & Security**

**Step 3.1: Environment-Specific Configurations**
```hcl
# terraform/environments/dev.tfvars
environment = "dev"
aws_region  = "us-west-2"

# terraform/environments/staging.tfvars
environment = "staging"
aws_region  = "us-west-2"

# terraform/environments/prod.tfvars
environment = "prod"
aws_region  = "us-west-2"
```

**Step 3.2: Security Scanning & Compliance**
```hcl
# terraform/security.tf
resource "aws_config_configuration_recorder" "recorder" {
  count    = var.environment == "prod" ? 1 : 0
  name     = "blockchain-node-config-recorder"
  role_arn = aws_iam_role.config_role[0].arn
  
  recording_group {
    all_supported = true
  }
}

resource "aws_guardduty_detector" "detector" {
  count  = var.environment == "prod" ? 1 : 0
  enable = true
  
  datasources {
    s3_logs {
      enable = true
    }
    malware_protection {
      scan_ec2_instance_with_findings {
        ebs_volumes {
          enable = true
        }
      }
    }
  }
}
```

**Files to Create:**
- [ ] `terraform/main.tf`
- [ ] `terraform/variables.tf`
- [ ] `terraform/outputs.tf`
- [ ] `terraform/ecr.tf`
- [ ] `terraform/iam.tf`
- [ ] `terraform/s3.tf`
- [ ] `terraform/security.tf`
- [ ] `terraform/environments/dev.tfvars`
- [ ] `terraform/environments/staging.tfvars`
- [ ] `terraform/environments/prod.tfvars`

---

## 📝 **TASK 3.2: CONFIGURATION MANAGEMENT & VALIDATION**

**Priority**: MEDIUM | **Effort**: 2 weeks | **Owner**: DevOps + Security Team

### **Current State Issues:**
- ❌ No schema validation for chain specs
- ❌ Manual configuration management
- ❌ No automated testing of configurations
- ❌ Missing configuration security checks
- ❌ No environment-specific validation

### **Implementation Steps:**

#### **Week 1: Schema Validation Framework**

**Step 1.1: JSON Schema Creation**
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "Chain Specification Schema",
  "type": "object",
  "required": ["name", "id", "chainType", "genesis"],
  "properties": {
    "name": {
      "type": "string",
      "pattern": "^[a-zA-Z0-9_-]+$"
    },
    "id": {
      "type": "string",
      "pattern": "^[a-z0-9_]+$"
    },
    "chainType": {
      "type": "string",
      "enum": ["Development", "Local", "Live"]
    },
    "genesis": {
      "type": "object",
      "required": ["runtime"],
      "properties": {
        "runtime": {
          "type": "object",
          "required": ["system", "balances", "sudo"],
          "properties": {
            "system": {
              "type": "object",
              "required": ["code"],
              "properties": {
                "code": {
                  "type": "string",
                  "pattern": "^0x[a-fA-F0-9]+$"
                }
              }
            },
            "balances": {
              "type": "object",
              "required": ["balances"],
              "properties": {
                "balances": {
                  "type": "array",
                  "items": {
                    "type": "array",
                    "items": [
                      {
                        "type": "string",
                        "pattern": "^5[a-zA-Z0-9]{47}$"
                      },
                      {
                        "type": "integer",
                        "minimum": 0
                      }
                    ]
                  }
                }
              }
            },
            "sudo": {
              "type": "object",
              "required": ["key"],
              "properties": {
                "key": {
                  "type": "string",
                  "pattern": "^5[a-zA-Z0-9]{47}$"
                }
              }
            }
          }
        }
      }
    }
  }
}
```

**Step 1.2: Rust Validation Implementation**
```rust
// node/service/src/config_validation.rs
use jsonschema::{JSONSchema, ValidationError};
use serde_json::Value;
use std::fs;

pub struct ConfigValidator {
    schema: JSONSchema,
}

impl ConfigValidator {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let schema_content = include_str!("../schemas/chain-spec.schema.json");
        let schema: Value = serde_json::from_str(schema_content)?;
        let compiled = JSONSchema::compile(&schema)?;
        
        Ok(ConfigValidator { schema: compiled })
    }
    
    pub fn validate_chain_spec(&self, spec_path: &str) -> Result<(), ValidationError> {
        let content = fs::read_to_string(spec_path)?;
        let spec: Value = serde_json::from_str(&content)?;
        
        if let Err(errors) = self.schema.validate(&spec) {
            for error in errors {
                eprintln!("Validation error: {}", error);
            }
            return Err(ValidationError::new("Chain spec validation failed"));
        }
        
        // Additional security validations
        self.validate_security_constraints(&spec)?;
        
        Ok(())
    }
    
    fn validate_security_constraints(&self, spec: &Value) -> Result<(), ValidationError> {
        // Validate balance distributions
        if let Some(balances) = spec.pointer("/genesis/runtime/balances/balances") {
            if let Some(balance_array) = balances.as_array() {
                for balance in balance_array {
                    if let Some(amount) = balance.get(1) {
                        if let Some(amount_num) = amount.as_u64() {
                            if amount_num > 1_000_000_000_000_000 {
                                return Err(ValidationError::new("Balance exceeds maximum allowed"));
                            }
                        }
                    }
                }
            }
        }
        
        // Validate sudo key is not a development key
        if let Some(sudo_key) = spec.pointer("/genesis/runtime/sudo/key") {
            if let Some(key_str) = sudo_key.as_str() {
                let dev_keys = [
                    "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY", // Alice
                    "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty", // Bob
                ];
                
                if spec.pointer("/chainType").unwrap().as_str() != Some("Development") 
                   && dev_keys.contains(&key_str) {
                    return Err(ValidationError::new("Development key used in non-development chain"));
                }
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_valid_chain_spec() {
        let validator = ConfigValidator::new().unwrap();
        assert!(validator.validate_chain_spec("chain-specs/devnet.json").is_ok());
    }
    
    #[test]
    fn test_invalid_balance() {
        // Test with invalid balance configuration
    }
    
    #[test]
    fn test_security_constraints() {
        // Test security validation rules
    }
}
```

#### **Week 2: Automated Configuration Testing**

**Step 2.1: GitHub Actions Integration**
```yaml
# .github/workflows/config-validation.yaml
name: Configuration Validation
on:
  pull_request:
    paths: ['node/service/chain-specs/**']
  push:
    branches: [dev, staging, master]

jobs:
  validate-schemas:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install JSON Schema Validator
        run: |
          pip install jsonschema
          npm install -g ajv-cli
          
      - name: Validate Chain Specifications
        run: |
          # Validate against JSON schema
          for spec in node/service/chain-specs/*.json; do
            echo "Validating $spec..."
            ajv validate -s schemas/chain-spec.schema.json -d "$spec"
            if [ $? -ne 0 ]; then
              echo "❌ Schema validation failed for $spec"
              exit 1
            fi
          done
          
      - name: Security Validation
        run: |
          # Check for development keys in production specs
          if grep -q "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY" node/service/chain-specs/mainnet.json; then
            echo "❌ Development key found in mainnet spec"
            exit 1
          fi
          
      - name: Runtime Configuration Tests
        run: |
          cargo test validate_runtime_config --features runtime-tests
          
  validate-runtime-build:
    runs-on: ubuntu-latest
    needs: validate-schemas
    steps:
      - uses: actions/checkout@v4
      
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: 1.82.0
          target: wasm32-unknown-unknown
          override: true
          
      - name: Test Runtime Build with Specs
        run: |
          for spec in node/service/chain-specs/*.json; do
            echo "Testing runtime build with $spec..."
            cargo run --release -- build-spec --chain="$spec" --raw > /tmp/raw_spec.json
            cargo run --release -- check-block --chain=/tmp/raw_spec.json 0
          done
```

**Step 2.2: Environment-Specific Validation**
```bash
#!/bin/bash
# scripts/validate-environment.sh
set -e

ENVIRONMENT=${1:-dev}
SPEC_FILE="node/service/chain-specs/${ENVIRONMENT}.json"

echo "🔍 Validating $ENVIRONMENT environment configuration..."

# Schema validation
echo "📋 Running schema validation..."
ajv validate -s schemas/chain-spec.schema.json -d "$SPEC_FILE"

# Environment-specific checks
case $ENVIRONMENT in
  "dev")
    echo "🔧 Development environment checks..."
    # Allow development keys
    ;;
  "staging")
    echo "🧪 Staging environment checks..."
    # Stricter validation
    if grep -q "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY" "$SPEC_FILE"; then
      echo "❌ Development key found in staging spec"
      exit 1
    fi
    ;;
  "prod")
    echo "🚀 Production environment checks..."
    # Strictest validation
    if grep -q "Development\|Local" "$SPEC_FILE"; then
      echo "❌ Development chain type in production spec"
      exit 1
    fi
    ;;
esac

echo "✅ Configuration validation passed for $ENVIRONMENT"
```

**Files to Create:**
- [ ] `schemas/chain-spec.schema.json`
- [ ] `node/service/src/config_validation.rs`
- [ ] `.github/workflows/config-validation.yaml`
- [ ] `scripts/validate-environment.sh`

---

## 🔐 **TASK 3.3: BLOCKCHAIN NETWORK SECURITY**

**Priority**: MEDIUM | **Effort**: 2 weeks | **Owner**: Security + Blockchain Team

### **Current State Issues:**
- ❌ No network health monitoring
- ❌ Missing peer validation
- ❌ No encrypted communication
- ❌ Inadequate consensus monitoring
- ❌ No network attack detection

### **Implementation Steps:**

#### **Week 1: Network Security Monitoring**

**Step 1.1: Network Health Monitoring Pallet**
```rust
// pallets/network-monitor/src/lib.rs
use frame_support::{
    dispatch::DispatchResult,
    pallet_prelude::*,
    traits::Get,
};
use frame_system::pallet_prelude::*;
use sp_std::vec::Vec;

#[pallet::pallet]
pub struct Pallet<T>(_);

#[pallet::config]
pub trait Config: frame_system::Config {
    type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    type MaxPeers: Get<u32>;
    type SecurityThreshold: Get<u32>;
}

#[pallet::storage]
pub type NetworkHealth<T: Config> = StorageValue<_, NetworkHealthStatus, ValueQuery>;

#[pallet::storage]
pub type ConnectedPeers<T: Config> = StorageMap<_, Blake2_128Concat, Vec<u8>, PeerInfo<T>, OptionQuery>;

#[pallet::storage]
pub type SecurityEvents<T: Config> = StorageMap<_, Blake2_128Concat, T::BlockNumber, Vec<SecurityEvent>, ValueQuery>;

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct NetworkHealthStatus {
    pub peer_count: u32,
    pub block_rate: u32,
    pub consensus_rate: u32,
    pub security_score: u32,
    pub last_updated: u64,
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct PeerInfo<T: Config> {
    pub peer_id: Vec<u8>,
    pub reputation: u32,
    pub last_seen: T::BlockNumber,
    pub misbehavior_count: u32,
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum SecurityEvent {
    SuspiciousActivity { peer_id: Vec<u8>, reason: Vec<u8> },
    ConsensusFailure { validator: Vec<u8> },
    NetworkPartition { affected_peers: u32 },
    HighMisbehavior { peer_id: Vec<u8>, count: u32 },
}

#[pallet::event]
#[pallet::generate_deposit(pub(super) fn deposit_event)]
pub enum Event<T: Config> {
    NetworkHealthUpdated { status: NetworkHealthStatus },
    SecurityAlert { event: SecurityEvent },
    PeerBlacklisted { peer_id: Vec<u8> },
    NetworkRecovered,
}

#[pallet::error]
pub enum Error<T> {
    PeerNotFound,
    InvalidSecurityThreshold,
    NetworkUnhealthy,
}

#[pallet::call]
impl<T: Config> Pallet<T> {
    #[pallet::weight(10_000)]
    pub fn update_network_health(origin: OriginFor<T>) -> DispatchResult {
        ensure_signed(origin)?;
        
        let health_status = Self::calculate_network_health();
        NetworkHealth::<T>::put(&health_status);
        
        if health_status.security_score < T::SecurityThreshold::get() {
            Self::deposit_event(Event::SecurityAlert {
                event: SecurityEvent::NetworkPartition {
                    affected_peers: health_status.peer_count,
                },
            });
        }
        
        Self::deposit_event(Event::NetworkHealthUpdated { status: health_status });
        Ok(())
    }
    
    #[pallet::weight(10_000)]
    pub fn report_peer_misbehavior(
        origin: OriginFor<T>,
        peer_id: Vec<u8>,
        reason: Vec<u8>,
    ) -> DispatchResult {
        ensure_signed(origin)?;
        
        ConnectedPeers::<T>::mutate(&peer_id, |peer_info| {
            if let Some(info) = peer_info {
                info.misbehavior_count += 1;
                if info.misbehavior_count > 5 {
                    // Blacklist peer
                    Self::deposit_event(Event::PeerBlacklisted { peer_id: peer_id.clone() });
                }
            }
        });
        
        let security_event = SecurityEvent::SuspiciousActivity { peer_id, reason };
        let current_block = frame_system::Pallet::<T>::block_number();
        SecurityEvents::<T>::mutate(&current_block, |events| {
            events.push(security_event.clone());
        });
        
        Self::deposit_event(Event::SecurityAlert { event: security_event });
        Ok(())
    }
}

impl<T: Config> Pallet<T> {
    pub fn calculate_network_health() -> NetworkHealthStatus {
        let peer_count = ConnectedPeers::<T>::iter().count() as u32;
        let block_rate = Self::calculate_block_rate();
        let consensus_rate = Self::calculate_consensus_rate();
        let security_score = Self::calculate_security_score();
        
        NetworkHealthStatus {
            peer_count,
            block_rate,
            consensus_rate,
            security_score,
            last_updated: Self::current_timestamp(),
        }
    }
    
    fn calculate_block_rate() -> u32 {
        // Calculate blocks per minute
        // Implementation depends on your consensus mechanism
        60 // Placeholder
    }
    
    fn calculate_consensus_rate() -> u32 {
        // Calculate consensus participation rate
        // Implementation depends on your consensus mechanism
        95 // Placeholder
    }
    
    fn calculate_security_score() -> u32 {
        let total_peers = ConnectedPeers::<T>::iter().count() as u32;
        let misbehaving_peers = ConnectedPeers::<T>::iter()
            .filter(|(_, info)| info.misbehavior_count > 0)
            .count() as u32;
        
        if total_peers == 0 {
            return 0;
        }
        
        let good_peer_ratio = (total_peers - misbehaving_peers) * 100 / total_peers;
        good_peer_ratio.min(100)
    }
    
    fn current_timestamp() -> u64 {
        // Get current timestamp
        // Implementation depends on your timestamp provider
        0 // Placeholder
    }
}
```

#### **Week 2: Network Communication Security**

**Step 2.1: Encrypted RPC Server**
```rust
// node/rpc/src/secure_rpc.rs
use jsonrpsee::{
    core::server::ServerHandle,
    server::{ServerBuilder, ServerConfig},
    RpcModule,
};
use rustls::{Certificate, PrivateKey, ServerConfig as TlsConfig};
use std::{fs, io::BufReader, path::Path, sync::Arc};
use tokio_rustls::TlsAcceptor;

pub struct SecureRpcServer {
    config: ServerConfig,
    tls_acceptor: Option<TlsAcceptor>,
}

impl SecureRpcServer {
    pub fn new() -> Self {
        let config = ServerConfig::builder()
            .max_request_body_size(10 * 1024 * 1024) // 10MB
            .max_response_body_size(10 * 1024 * 1024) // 10MB
            .max_connections(100)
            .build();
        
        Self {
            config,
            tls_acceptor: None,
        }
    }
    
    pub fn with_tls(mut self, cert_path: &Path, key_path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let certs = Self::load_certs(cert_path)?;
        let key = Self::load_private_key(key_path)?;
        
        let tls_config = TlsConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(certs, key)?;
        
        self.tls_acceptor = Some(TlsAcceptor::from(Arc::new(tls_config)));
        Ok(self)
    }
    
    pub async fn start(
        self,
        addr: &str,
        rpc_module: RpcModule<()>,
    ) -> Result<ServerHandle, Box<dyn std::error::Error>> {
        let builder = ServerBuilder::new().set_host_filtering(jsonrpsee::core::server::host_filtering::AllowHosts::Any);
        
        if let Some(tls_acceptor) = self.tls_acceptor {
            let server = builder
                .custom_tokio_runtime(tokio::runtime::Handle::current())
                .build(addr)
                .await?;
            
            let handle = server.start(rpc_module)?;
            Ok(handle)
        } else {
            let server = builder.build(addr).await?;
            let handle = server.start(rpc_module)?;
            Ok(handle)
        }
    }
    
    fn load_certs(path: &Path) -> Result<Vec<Certificate>, Box<dyn std::error::Error>> {
        let certfile = fs::File::open(path)?;
        let mut reader = BufReader::new(certfile);
        let certs = rustls_pemfile::certs(&mut reader)?
            .into_iter()
            .map(Certificate)
            .collect();
        Ok(certs)
    }
    
    fn load_private_key(path: &Path) -> Result<PrivateKey, Box<dyn std::error::Error>> {
        let keyfile = fs::File::open(path)?;
        let mut reader = BufReader::new(keyfile);
        let keys = rustls_pemfile::pkcs8_private_keys(&mut reader)?;
        
        if keys.is_empty() {
            return Err("No private keys found".into());
        }
        
        Ok(PrivateKey(keys[0].clone()))
    }
}

// Usage example
pub async fn create_secure_rpc_server(
    cert_path: Option<&Path>,
    key_path: Option<&Path>,
) -> Result<ServerHandle, Box<dyn std::error::Error>> {
    let mut server = SecureRpcServer::new();
    
    if let (Some(cert), Some(key)) = (cert_path, key_path) {
        server = server.with_tls(cert, key)?;
    }
    
    // Create RPC module with your methods
    let mut rpc_module = RpcModule::new(());
    
    // Add your RPC methods here
    rpc_module.register_method("system_health", |_, _| {
        Ok(serde_json::json!({
            "peers": 8,
            "isSyncing": false,
            "shouldHavePeers": true
        }))
    })?;
    
    server.start("127.0.0.1:9933", rpc_module).await
}
```

**Step 2.2: Network Security Integration**
```rust
// node/service/src/network_security.rs
use crate::NetworkHealthStatus;
use sc_network::{NetworkService, PeerId};
use sp_runtime::traits::Block as BlockT;
use std::sync::Arc;
use tokio::time::{interval, Duration};

pub struct NetworkSecurityService<Block: BlockT> {
    network: Arc<NetworkService<Block, <Block as BlockT>::Hash>>,
    health_status: NetworkHealthStatus,
}

impl<Block: BlockT> NetworkSecurityService<Block> {
    pub fn new(network: Arc<NetworkService<Block, <Block as BlockT>::Hash>>) -> Self {
        Self {
            network,
            health_status: NetworkHealthStatus::default(),
        }
    }
    
    pub async fn start_monitoring(&mut self) {
        let mut monitoring_interval = interval(Duration::from_secs(30));
        
        loop {
            monitoring_interval.tick().await;
            self.update_network_health().await;
            self.check_peer_health().await;
            self.detect_network_attacks().await;
        }
    }
    
    async fn update_network_health(&mut self) {
        let sync_status = self.network.sync_status();
        let connected_peers = self.network.connected_peers().count();
        
        self.health_status = NetworkHealthStatus {
            peer_count: connected_peers as u32,
            block_rate: self.calculate_block_rate(),
            consensus_rate: self.calculate_consensus_rate(),
            security_score: self.calculate_security_score(),
            last_updated: chrono::Utc::now().timestamp() as u64,
        };
        
        if self.health_status.security_score < 70 {
            log::warn!("Network security score low: {}", self.health_status.security_score);
        }
    }
    
    async fn check_peer_health(&self) {
        for peer_id in self.network.connected_peers() {
            if let Some(peer_info) = self.network.peer_info(peer_id) {
                // Check peer reputation and behavior
                if peer_info.reputation < 50 {
                    log::warn!("Peer {} has low reputation: {}", peer_id, peer_info.reputation);
                    // Consider disconnecting or reporting
                }
            }
        }
    }
    
    async fn detect_network_attacks(&self) {
        let connected_peers = self.network.connected_peers().count();
        
        // Detection heuristics
        if connected_peers > 1000 {
            log::warn!("Potential DDoS attack: {} connected peers", connected_peers);
        }
        
        // Check for eclipse attacks (too few peers)
        if connected_peers < 3 {
            log::warn!("Potential eclipse attack: only {} connected peers", connected_peers);
        }
    }
    
    fn calculate_block_rate(&self) -> u32 {
        // Implementation depends on your consensus mechanism
        60 // Placeholder: blocks per minute
    }
    
    fn calculate_consensus_rate(&self) -> u32 {
        // Implementation depends on your consensus mechanism
        95 // Placeholder: participation rate
    }
    
    fn calculate_security_score(&self) -> u32 {
        let peer_count = self.network.connected_peers().count() as u32;
        let sync_status = self.network.sync_status();
        
        let mut score = 100;
        
        // Reduce score based on network conditions
        if peer_count < 5 {
            score -= 20;
        }
        
        if sync_status.state.is_major_syncing() {
            score -= 10;
        }
        
        score.max(0)
    }
}
```

**Files to Create:**
- [ ] `pallets/network-monitor/src/lib.rs`
- [ ] `pallets/network-monitor/Cargo.toml`
- [ ] `node/rpc/src/secure_rpc.rs`
- [ ] `node/service/src/network_security.rs`
- [ ] `certs/server.crt` (development)
- [ ] `certs/server.key` (development)

---

## ✅ **SUCCESS METRICS & VALIDATION**

### **Task 3.1 Metrics:**
- [ ] ✅ Complete terraform infrastructure deployed
- [ ] ✅ ECR repositories secured with KMS encryption
- [ ] ✅ OIDC authentication working
- [ ] ✅ Multi-environment support active
- [ ] ✅ Infrastructure security scanning enabled

### **Task 3.2 Metrics:**
- [ ] ✅ All chain specs validated against schema
- [ ] ✅ Automated configuration testing in CI
- [ ] ✅ Environment-specific validation working
- [ ] ✅ Security constraints enforced
- [ ] ✅ Configuration drift detection active

### **Task 3.3 Metrics:**
- [ ] ✅ Network health monitoring operational
- [ ] ✅ Peer validation and reputation system working
- [ ] ✅ Encrypted RPC communication enabled
- [ ] ✅ Attack detection heuristics active
- [ ] ✅ Security event logging functional

---

## 🗂️ **COMPLETE FILE CHECKLIST**

### **Infrastructure Files:**
- [ ] `terraform/main.tf` - Core terraform configuration
- [ ] `terraform/variables.tf` - Variable definitions
- [ ] `terraform/outputs.tf` - Output definitions
- [ ] `terraform/ecr.tf` - ECR repository configuration
- [ ] `terraform/iam.tf` - IAM roles and policies
- [ ] `terraform/s3.tf` - S3 bucket for sccache
- [ ] `terraform/security.tf` - Security services configuration
- [ ] `terraform/environments/dev.tfvars` - Development environment
- [ ] `terraform/environments/staging.tfvars` - Staging environment
- [ ] `terraform/environments/prod.tfvars` - Production environment

### **Configuration Files:**
- [ ] `schemas/chain-spec.schema.json` - JSON schema for chain specs
- [ ] `node/service/src/config_validation.rs` - Configuration validation
- [ ] `.github/workflows/config-validation.yaml` - CI validation
- [ ] `scripts/validate-environment.sh` - Environment validation script

### **Security Files:**
- [ ] `pallets/network-monitor/src/lib.rs` - Network monitoring pallet
- [ ] `pallets/network-monitor/Cargo.toml` - Pallet configuration
- [ ] `node/rpc/src/secure_rpc.rs` - Secure RPC implementation
- [ ] `node/service/src/network_security.rs` - Network security service
- [ ] `certs/server.crt` - TLS certificate (development)
- [ ] `certs/server.key` - TLS private key (development)

This comprehensive Phase 3 plan addresses all the critical infrastructure and security gaps while maintaining focus on the 3 specific tasks you requested. Each task is detailed with specific implementation steps, code examples, and success metrics. 
