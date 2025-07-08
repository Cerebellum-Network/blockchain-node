resource "aws_s3_bucket" "sccache" {
  bucket        = "cere-blockchain-sccache-${var.environment}"
  force_destroy = var.environment != "prod"
  
  tags = {
    Name        = "sccache-bucket"
    Environment = var.environment
  }
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
    
    abort_incomplete_multipart_upload {
      days_after_initiation = 7
    }
  }
}

resource "aws_s3_bucket_cors_configuration" "sccache" {
  bucket = aws_s3_bucket.sccache.id
  
  cors_rule {
    allowed_headers = ["*"]
    allowed_methods = ["GET", "PUT", "POST", "DELETE", "HEAD"]
    allowed_origins = ["*"]
    expose_headers  = ["ETag"]
    max_age_seconds = 3000
  }
}

# Terraform state bucket (if it doesn't exist)
resource "aws_s3_bucket" "terraform_state" {
  count         = var.environment == "prod" ? 1 : 0
  bucket        = "cere-terraform-state"
  force_destroy = false
  
  tags = {
    Name = "terraform-state-bucket"
  }
}

resource "aws_s3_bucket_encryption" "terraform_state" {
  count  = var.environment == "prod" ? 1 : 0
  bucket = aws_s3_bucket.terraform_state[0].id
  
  server_side_encryption_configuration {
    rule {
      apply_server_side_encryption_by_default {
        sse_algorithm = "AES256"
      }
    }
  }
}

resource "aws_s3_bucket_public_access_block" "terraform_state" {
  count  = var.environment == "prod" ? 1 : 0
  bucket = aws_s3_bucket.terraform_state[0].id
  
  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

resource "aws_s3_bucket_versioning" "terraform_state" {
  count  = var.environment == "prod" ? 1 : 0
  bucket = aws_s3_bucket.terraform_state[0].id
  versioning_configuration {
    status = "Enabled"
  }
}

# DynamoDB table for terraform state locking
resource "aws_dynamodb_table" "terraform_state_lock" {
  count          = var.environment == "prod" ? 1 : 0
  name           = "terraform-state-lock"
  billing_mode   = "PAY_PER_REQUEST"
  hash_key       = "LockID"
  
  attribute {
    name = "LockID"
    type = "S"
  }
  
  tags = {
    Name = "terraform-state-lock"
  }
} 
