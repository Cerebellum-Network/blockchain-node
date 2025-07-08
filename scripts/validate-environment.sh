#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

ENVIRONMENT=${1:-dev}
SPEC_FILE=""

# Determine spec file based on environment
case $ENVIRONMENT in
  "dev")
    SPEC_FILE="node/service/chain-specs/devnet.json"
    ;;
  "staging")
    SPEC_FILE="node/service/chain-specs/testnet.json"
    ;;
  "prod")
    SPEC_FILE="node/service/chain-specs/mainnet.json"
    ;;
  *)
    echo -e "${RED}❌ Invalid environment: $ENVIRONMENT${NC}"
    echo "Usage: $0 <dev|staging|prod>"
    exit 1
    ;;
esac

echo -e "${BLUE}🔍 Validating $ENVIRONMENT environment configuration...${NC}"

# Check if spec file exists
if [ ! -f "$SPEC_FILE" ]; then
  echo -e "${RED}❌ Spec file not found: $SPEC_FILE${NC}"
  exit 1
fi

# Check if ajv is available
if ! command -v ajv &> /dev/null; then
  echo -e "${YELLOW}⚠️ ajv not found, installing...${NC}"
  npm install -g ajv-cli
fi

# Schema validation
echo -e "${BLUE}📋 Running schema validation...${NC}"
if ajv validate -s schemas/chain-spec.schema.json -d "$SPEC_FILE"; then
  echo -e "${GREEN}✅ Schema validation passed${NC}"
else
  echo -e "${RED}❌ Schema validation failed${NC}"
  exit 1
fi

# Environment-specific checks
echo -e "${BLUE}🌍 Running environment-specific checks...${NC}"

case $ENVIRONMENT in
  "dev")
    echo -e "${BLUE}🔧 Development environment checks...${NC}"
    
    # Check chainType is Development
    if ! grep -q '"chainType": "Development"' "$SPEC_FILE"; then
      echo -e "${RED}❌ Development environment must have chainType: Development${NC}"
      exit 1
    fi
    
    # Check id ends with _dev
    if ! grep -q '"id": ".*_dev"' "$SPEC_FILE"; then
      echo -e "${RED}❌ Development environment id must end with _dev${NC}"
      exit 1
    fi
    
    # Allow development keys in dev environment
    echo -e "${GREEN}✅ Development keys are allowed in dev environment${NC}"
    
    # Check for reasonable test balances
    python3 -c "
import json
with open('$SPEC_FILE', 'r') as f:
    spec = json.load(f)

balances = spec.get('genesis', {}).get('runtime', {}).get('balances', {}).get('balances', [])
for balance in balances:
    if len(balance) >= 2:
        amount = balance[1]
        if amount > 100000000000000:  # 100K CERE for dev
            print('❌ Excessive balance for dev environment:', amount)
            exit(1)
print('✅ Balance validation passed for dev environment')
    "
    ;;
    
  "staging")
    echo -e "${BLUE}🧪 Staging environment checks...${NC}"
    
    # Check chainType is not Development
    if grep -q '"chainType": "Development"' "$SPEC_FILE"; then
      echo -e "${RED}❌ Staging environment cannot have chainType: Development${NC}"
      exit 1
    fi
    
    # Check for development keys
    DEV_KEYS=(
      "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY"  # Alice
      "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty"  # Bob
      "5GNJqTPyNqANBkUVMN1LPPrxXnFouWXoe2wNSmmEoLctxiZY"  # Charlie
    )
    
    for key in "${DEV_KEYS[@]}"; do
      if grep -q "$key" "$SPEC_FILE"; then
        echo -e "${RED}❌ Development key found in staging spec: $key${NC}"
        exit 1
      fi
    done
    
    # Check for reasonable staging balances
    python3 -c "
import json
with open('$SPEC_FILE', 'r') as f:
    spec = json.load(f)

balances = spec.get('genesis', {}).get('runtime', {}).get('balances', {}).get('balances', [])
for balance in balances:
    if len(balance) >= 2:
        amount = balance[1]
        if amount > 500000000000000:  # 500K CERE for staging
            print('❌ Excessive balance for staging environment:', amount)
            exit(1)
print('✅ Balance validation passed for staging environment')
    "
    
    # Check validator configuration
    python3 -c "
import json
with open('$SPEC_FILE', 'r') as f:
    spec = json.load(f)

staking = spec.get('genesis', {}).get('runtime', {}).get('staking', {})
validator_count = staking.get('validatorCount', 0)
min_validator_count = staking.get('minimumValidatorCount', 0)

if validator_count < 3:
    print('❌ Staging must have at least 3 validators')
    exit(1)
    
if min_validator_count < 3:
    print('❌ Staging must have minimum 3 validators')
    exit(1)
    
print('✅ Validator configuration valid for staging')
    "
    ;;
    
  "prod")
    echo -e "${BLUE}🚀 Production environment checks...${NC}"
    
    # Check chainType is Live
    if ! grep -q '"chainType": "Live"' "$SPEC_FILE"; then
      echo -e "${RED}❌ Production environment must have chainType: Live${NC}"
      exit 1
    fi
    
    # Check id doesn't end with _dev or _local
    if grep -q '"id": ".*_dev"' "$SPEC_FILE" || grep -q '"id": ".*_local"' "$SPEC_FILE"; then
      echo -e "${RED}❌ Production environment id cannot end with _dev or _local${NC}"
      exit 1
    fi
    
    # Strict check for development keys
    DEV_KEYS=(
      "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY"  # Alice
      "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty"  # Bob
      "5GNJqTPyNqANBkUVMN1LPPrxXnFouWXoe2wNSmmEoLctxiZY"  # Charlie
      "5HpG9w8EBLe5XCrbczpwq5TSXvedjrBGCwqxK1iQ7qUsSWFc"  # Dave
      "5FLSigC9HGRKVhB9FiEo4Y3koPsNmBmLJbpXg2mp1hXcS59Y"  # Eve
    )
    
    for key in "${DEV_KEYS[@]}"; do
      if grep -q "$key" "$SPEC_FILE"; then
        echo -e "${RED}❌ Development key found in production spec: $key${NC}"
        exit 1
      fi
    done
    
    # Check for production-grade validator configuration
    python3 -c "
import json
with open('$SPEC_FILE', 'r') as f:
    spec = json.load(f)

staking = spec.get('genesis', {}).get('runtime', {}).get('staking', {})
validator_count = staking.get('validatorCount', 0)
min_validator_count = staking.get('minimumValidatorCount', 0)

if validator_count < 10:
    print('❌ Production must have at least 10 validators')
    exit(1)
    
if min_validator_count < 7:
    print('❌ Production must have minimum 7 validators')
    exit(1)
    
print('✅ Validator configuration valid for production')
    "
    
    # Check for reasonable production balances
    python3 -c "
import json
with open('$SPEC_FILE', 'r') as f:
    spec = json.load(f)

balances = spec.get('genesis', {}).get('runtime', {}).get('balances', {}).get('balances', [])
total_supply = 0
for balance in balances:
    if len(balance) >= 2:
        amount = balance[1]
        total_supply += amount
        if amount > 1000000000000000:  # 1M CERE max individual
            print('❌ Excessive individual balance for production:', amount)
            exit(1)

if total_supply > 10000000000000000:  # 10M CERE max total
    print('❌ Excessive total supply for production:', total_supply)
    exit(1)
    
print('✅ Balance validation passed for production environment')
    "
    
    # Check for secure network configuration
    python3 -c "
import json
with open('$SPEC_FILE', 'r') as f:
    spec = json.load(f)

# Check for proper boot nodes
boot_nodes = spec.get('bootNodes', [])
if len(boot_nodes) < 3:
    print('❌ Production must have at least 3 boot nodes')
    exit(1)

# Check for telemetry endpoints
telemetry = spec.get('telemetryEndpoints')
if telemetry is None:
    print('⚠️ No telemetry endpoints configured for production')
else:
    print('✅ Telemetry endpoints configured')

print('✅ Network configuration valid for production')
    "
    ;;
esac

# DDC-specific validation
echo -e "${BLUE}🔗 Running DDC-specific validation...${NC}"
python3 -c "
import json
with open('$SPEC_FILE', 'r') as f:
    spec = json.load(f)

# Check DDC nodes configuration
ddc_nodes = spec.get('genesis', {}).get('runtime', {}).get('ddcNodes', {}).get('storageNodes', [])
if len(ddc_nodes) == 0:
    print('⚠️ No DDC storage nodes configured')
else:
    for i, node in enumerate(ddc_nodes):
        props = node.get('props', {})
        
        # Check port ranges
        http_port = props.get('http_port', 0)
        grpc_port = props.get('grpc_port', 0)
        p2p_port = props.get('p2p_port', 0)
        
        if http_port < 1024 or http_port > 65535:
            print(f'❌ Invalid HTTP port for node {i}: {http_port}')
            exit(1)
            
        if grpc_port < 1024 or grpc_port > 65535:
            print(f'❌ Invalid gRPC port for node {i}: {grpc_port}')
            exit(1)
            
        if p2p_port < 1024 or p2p_port > 65535:
            print(f'❌ Invalid P2P port for node {i}: {p2p_port}')
            exit(1)
        
        # Check host IP
        host = props.get('host', [])
        if len(host) != 4:
            print(f'❌ Invalid host IP for node {i}: {host}')
            exit(1)
            
        for octet in host:
            if octet < 0 or octet > 255:
                print(f'❌ Invalid IP octet for node {i}: {octet}')
                exit(1)
    
    print(f'✅ DDC nodes validation passed ({len(ddc_nodes)} nodes)')

# Check DDC clusters
ddc_clusters = spec.get('genesis', {}).get('runtime', {}).get('ddcClusters', {}).get('clusters', [])
if len(ddc_clusters) == 0:
    print('⚠️ No DDC clusters configured')
else:
    print(f'✅ DDC clusters configured ({len(ddc_clusters)} clusters)')
"

# Final validation summary
echo -e "${GREEN}✅ Configuration validation passed for $ENVIRONMENT environment${NC}"
echo -e "${BLUE}📋 Summary:${NC}"
echo "  - Environment: $ENVIRONMENT"
echo "  - Spec file: $SPEC_FILE"
echo "  - Schema validation: ✅ PASSED"
echo "  - Security validation: ✅ PASSED"
echo "  - Environment constraints: ✅ PASSED"
echo "  - DDC configuration: ✅ PASSED"

echo -e "${GREEN}🎉 All validations completed successfully!${NC}" 
