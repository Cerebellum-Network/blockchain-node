# Outputs for Cere Blockchain Node Infrastructure

output "ecr_repository_url" {
  description = "URL of the ECR repository"
  value       = aws_ecr_repository.blockchain_node.repository_url
}

output "ecr_repository_arn" {
  description = "ARN of the ECR repository"
  value       = aws_ecr_repository.blockchain_node.arn
}

output "github_actions_role_arn" {
  description = "ARN of the GitHub Actions IAM role"
  value       = aws_iam_role.github_actions.arn
}

output "github_oidc_provider_arn" {
  description = "ARN of the GitHub OIDC provider"
  value       = aws_iam_openid_connect_provider.github.arn
}

output "kms_key_arn" {
  description = "ARN of the KMS key for ECR encryption"
  value       = aws_kms_key.ecr.arn
}

output "kms_key_alias" {
  description = "Alias of the KMS key for ECR encryption"
  value       = aws_kms_alias.ecr.name
}

output "terraform_state_bucket" {
  description = "Name of the S3 bucket for Terraform state"
  value       = aws_s3_bucket.terraform_state.bucket
}

output "terraform_state_lock_table" {
  description = "Name of the DynamoDB table for Terraform state locking"
  value       = aws_dynamodb_table.terraform_state_lock.name
}

output "aws_region" {
  description = "AWS region where resources are deployed"
  value       = data.aws_region.current.name
}

output "aws_account_id" {
  description = "AWS account ID"
  value       = data.aws_caller_identity.current.account_id
}