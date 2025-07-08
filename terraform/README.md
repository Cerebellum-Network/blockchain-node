# 🏗️ Blockchain Node Infrastructure

This directory contains Terraform configurations for deploying secure AWS infrastructure for the Cere blockchain node.

## 📋 Overview

The infrastructure includes:
- **ECR Repository** with KMS encryption and vulnerability scanning
- **IAM Roles** with OIDC for GitHub Actions
- **S3 Buckets** for sccache and terraform state
- **Security Services** (AWS Config, GuardDuty, CloudTrail for production)
- **Multi-environment support** (dev, staging, prod)

## 🚀 Quick Start

### Prerequisites

1. **AWS CLI** configured with appropriate credentials
2. **Terraform** >= 1.0 installed
3. **Proper IAM permissions** for resource creation

### Initial Setup

1. **Initialize Terraform** (first time only):
```bash
cd terraform
terraform init
```

2. **Plan deployment** for specific environment:
```bash
# Development
terraform plan -var-file=environments/dev.tfvars

# Staging  
terraform plan -var-file=environments/staging.tfvars

# Production
terraform plan -var-file=environments/prod.tfvars
```

3. **Apply infrastructure**:
```bash
# For development
terraform apply -var-file=environments/dev.tfvars

# For production (includes security services)
terraform apply -var-file=environments/prod.tfvars
```

## 📁 File Structure

```
terraform/
├── main.tf              # Provider and backend configuration
├── variables.tf         # Input variables
├── outputs.tf           # Output values
├── ecr.tf              # ECR repository and policies
├── iam.tf              # IAM roles and policies
├── s3.tf               # S3 buckets configuration
├── security.tf         # Security services (Config, GuardDuty, CloudTrail)
├── environments/       # Environment-specific variables
│   ├── dev.tfvars     # Development environment
│   ├── staging.tfvars # Staging environment
│   └── prod.tfvars    # Production environment
└── README.md          # This file
```

## 🔧 Configuration

### Environment Variables

Each environment has its own `.tfvars` file:

- **dev.tfvars**: Development environment settings
- **staging.tfvars**: Staging environment settings  
- **prod.tfvars**: Production environment settings

### Key Resources Created

#### 🏪 ECR Repository
- **Name**: `pos-network-node`
- **Features**: 
  - KMS encryption
  - Vulnerability scanning
  - Immutable image tags
  - Lifecycle policies

#### 🔐 IAM Configuration
- **GitHub OIDC Provider**: For secure CI/CD without long-lived credentials
- **GitHub Actions Role**: With ECR and S3 permissions
- **Config Service Role**: For AWS Config (production only)

#### 🪣 S3 Buckets
- **sccache bucket**: For Rust compilation caching
- **terraform state bucket**: For remote state storage (production)
- **config bucket**: For AWS Config (production)
- **cloudtrail bucket**: For audit logging (production)

#### 🛡️ Security Services (Production Only)
- **AWS Config**: Configuration compliance monitoring
- **GuardDuty**: Threat detection
- **CloudTrail**: API call logging

## 🔒 Security Features

### 🔐 Encryption
- **ECR**: KMS encryption for container images
- **S3**: Server-side encryption for all buckets
- **State**: Encrypted terraform state storage

### 🛡️ Access Control
- **IAM**: Least privilege access policies
- **OIDC**: Secure GitHub Actions integration
- **Bucket Policies**: Restricted S3 access

### 📊 Monitoring (Production)
- **AWS Config**: Continuous compliance monitoring
- **GuardDuty**: ML-based threat detection
- **CloudTrail**: Comprehensive audit logging

## 📊 Outputs

After successful deployment, Terraform outputs:

- `ecr_repository_url`: ECR repository URL for container pushes
- `github_actions_role_arn`: IAM role ARN for GitHub Actions
- `sccache_bucket_name`: S3 bucket name for sccache
- `ecr_repository_name`: ECR repository name
- `kms_key_arn`: KMS key ARN for ECR encryption

## 🔄 CI/CD Integration

### GitHub Actions Setup

Add these secrets to your GitHub repository:

1. **AWS_ROLE_ARN**: Use the `github_actions_role_arn` output
2. **ECR_REPOSITORY**: Use the `ecr_repository_url` output
3. **SCCACHE_BUCKET**: Use the `sccache_bucket_name` output

### Example GitHub Actions Configuration

```yaml
- name: Configure AWS credentials
  uses: aws-actions/configure-aws-credentials@v4
  with:
    role-to-assume: ${{ secrets.AWS_ROLE_ARN }}
    role-session-name: GitHubActions
    aws-region: us-west-2

- name: Login to Amazon ECR
  uses: aws-actions/amazon-ecr-login@v2

- name: Build and push Docker image
  run: |
    docker build -t ${{ secrets.ECR_REPOSITORY }}:${{ github.sha }} .
    docker push ${{ secrets.ECR_REPOSITORY }}:${{ github.sha }}
```

## 🌍 Environment Differences

### Development (`dev`)
- **Security Services**: Disabled
- **Bucket Retention**: Force destroy enabled
- **Monitoring**: Basic CloudWatch only

### Staging (`staging`)
- **Security Services**: Basic Config only
- **Bucket Retention**: Protected
- **Monitoring**: Enhanced CloudWatch

### Production (`prod`)
- **Security Services**: Full suite (Config, GuardDuty, CloudTrail)
- **Bucket Retention**: Fully protected
- **Monitoring**: Complete security monitoring
- **Compliance**: SOC2 ready configuration

## 🧹 Cleanup

To destroy infrastructure:

```bash
# Warning: This will delete all resources!
terraform destroy -var-file=environments/dev.tfvars
```

**⚠️ Important**: For production, ensure all data is backed up before destroying resources.

## 🔧 Troubleshooting

### Common Issues

1. **Permission Denied**: Ensure AWS credentials have sufficient permissions
2. **State Lock**: Use DynamoDB state locking for team environments  
3. **Resource Conflicts**: Check for existing resources with same names

### Useful Commands

```bash
# Check current state
terraform show

# List all resources
terraform state list

# Import existing resource
terraform import <resource_type>.<name> <resource_id>

# Refresh state
terraform refresh -var-file=environments/<env>.tfvars
```

## 📚 Additional Resources

- [AWS ECR Documentation](https://docs.aws.amazon.com/ecr/)
- [Terraform AWS Provider](https://registry.terraform.io/providers/hashicorp/aws/latest/docs)
- [GitHub Actions OIDC](https://docs.github.com/en/actions/deployment/security-hardening-your-deployments/configuring-openid-connect-in-amazon-web-services)

## 🆘 Support

For issues or questions:
1. Check the [troubleshooting section](#-troubleshooting)
2. Review AWS CloudTrail logs for API errors
3. Consult the team's infrastructure documentation 
