resource "aws_config_configuration_recorder" "recorder" {
  count    = var.environment == "prod" ? 1 : 0
  name     = "blockchain-node-config-recorder"
  role_arn = aws_iam_role.config_role[0].arn
  
  recording_group {
    all_supported = true
    
    # Enable global resource recording
    include_global_resource_types = true
    
    # Record all resource types
    recording_mode {
      recording_frequency = "CONTINUOUS"
    }
  }
  
  depends_on = [aws_s3_bucket.config_bucket]
}

resource "aws_config_delivery_channel" "delivery_channel" {
  count           = var.environment == "prod" ? 1 : 0
  name            = "blockchain-node-delivery-channel"
  s3_bucket_name  = aws_s3_bucket.config_bucket[0].bucket
  depends_on      = [aws_config_configuration_recorder.recorder]
}

resource "aws_s3_bucket" "config_bucket" {
  count         = var.environment == "prod" ? 1 : 0
  bucket        = "cere-blockchain-config-${var.environment}-${random_string.bucket_suffix[0].result}"
  force_destroy = false
  
  tags = {
    Name = "config-bucket"
  }
}

resource "random_string" "bucket_suffix" {
  count   = var.environment == "prod" ? 1 : 0
  length  = 8
  special = false
  upper   = false
}

resource "aws_s3_bucket_policy" "config_bucket_policy" {
  count  = var.environment == "prod" ? 1 : 0
  bucket = aws_s3_bucket.config_bucket[0].id
  
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid    = "AWSConfigBucketPermissionsCheck"
        Effect = "Allow"
        Principal = {
          Service = "config.amazonaws.com"
        }
        Action   = "s3:GetBucketAcl"
        Resource = aws_s3_bucket.config_bucket[0].arn
        Condition = {
          StringEquals = {
            "AWS:SourceAccount" = data.aws_caller_identity.current.account_id
          }
        }
      },
      {
        Sid    = "AWSConfigBucketExistenceCheck"
        Effect = "Allow"
        Principal = {
          Service = "config.amazonaws.com"
        }
        Action   = "s3:ListBucket"
        Resource = aws_s3_bucket.config_bucket[0].arn
        Condition = {
          StringEquals = {
            "AWS:SourceAccount" = data.aws_caller_identity.current.account_id
          }
        }
      },
      {
        Sid    = "AWSConfigBucketDelivery"
        Effect = "Allow"
        Principal = {
          Service = "config.amazonaws.com"
        }
        Action   = "s3:PutObject"
        Resource = "${aws_s3_bucket.config_bucket[0].arn}/*"
        Condition = {
          StringEquals = {
            "s3:x-amz-acl" = "bucket-owner-full-control"
            "AWS:SourceAccount" = data.aws_caller_identity.current.account_id
          }
        }
      }
    ]
  })
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
  
  tags = {
    Name = "blockchain-node-guardduty"
  }
}

# CloudTrail for audit logging
resource "aws_cloudtrail" "blockchain_trail" {
  count                         = var.environment == "prod" ? 1 : 0
  name                          = "blockchain-node-trail"
  s3_bucket_name                = aws_s3_bucket.cloudtrail_bucket[0].bucket
  include_global_service_events = true
  is_multi_region_trail         = true
  enable_logging                = true
  
  event_selector {
    read_write_type                 = "All"
    include_management_events       = true
    
    data_resource {
      type   = "AWS::S3::Object"
      values = ["${aws_s3_bucket.sccache.arn}/*"]
    }
  }
  
  tags = {
    Name = "blockchain-node-cloudtrail"
  }
}

resource "aws_s3_bucket" "cloudtrail_bucket" {
  count         = var.environment == "prod" ? 1 : 0
  bucket        = "cere-blockchain-cloudtrail-${var.environment}-${random_string.bucket_suffix[0].result}"
  force_destroy = false
  
  tags = {
    Name = "cloudtrail-bucket"
  }
}

resource "aws_s3_bucket_policy" "cloudtrail_bucket_policy" {
  count  = var.environment == "prod" ? 1 : 0
  bucket = aws_s3_bucket.cloudtrail_bucket[0].id
  
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid    = "AWSCloudTrailAclCheck"
        Effect = "Allow"
        Principal = {
          Service = "cloudtrail.amazonaws.com"
        }
        Action   = "s3:GetBucketAcl"
        Resource = aws_s3_bucket.cloudtrail_bucket[0].arn
        Condition = {
          StringEquals = {
            "AWS:SourceArn" = "arn:aws:cloudtrail:${data.aws_region.current.name}:${data.aws_caller_identity.current.account_id}:trail/blockchain-node-trail"
          }
        }
      },
      {
        Sid    = "AWSCloudTrailWrite"
        Effect = "Allow"
        Principal = {
          Service = "cloudtrail.amazonaws.com"
        }
        Action   = "s3:PutObject"
        Resource = "${aws_s3_bucket.cloudtrail_bucket[0].arn}/*"
        Condition = {
          StringEquals = {
            "s3:x-amz-acl" = "bucket-owner-full-control"
            "AWS:SourceArn" = "arn:aws:cloudtrail:${data.aws_region.current.name}:${data.aws_caller_identity.current.account_id}:trail/blockchain-node-trail"
          }
        }
      }
    ]
  })
} 
