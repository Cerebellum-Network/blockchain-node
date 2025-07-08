output "ecr_repository_url" {
  description = "ECR repository URL"
  value       = aws_ecr_repository.blockchain_node.repository_url
}

output "github_actions_role_arn" {
  description = "GitHub Actions IAM role ARN"
  value       = aws_iam_role.github_actions.arn
}

output "sccache_bucket_name" {
  description = "S3 bucket name for sccache"
  value       = aws_s3_bucket.sccache.bucket
}

output "ecr_repository_name" {
  description = "ECR repository name"
  value       = aws_ecr_repository.blockchain_node.name
}

output "kms_key_arn" {
  description = "KMS key ARN for ECR encryption"
  value       = aws_kms_key.ecr_key.arn
} 
