#!/bin/sh

# Prevent committing badly formatted code and security issues

echo "🔍 Running pre-commit security checks..."

# 1. Code formatting checks
echo "📝 Checking code formatting..."
cargo +nightly-2024-03-12 fmt -- --check
if [ $? -ne 0 ]; then
	echo "❌ Run \`cargo fmt\` to fix formatting issues before committing."
	exit 1
fi

dprint check
if [ $? -ne 0 ]; then
	echo "❌ Run \`dprint fmt\` to fix formatting issues before committing."
	exit 1
fi

# 2. Secret detection
echo "🔐 Scanning for potential secrets..."
if command -v detect-secrets >/dev/null 2>&1; then
    # Check if baseline exists, create if not
    if [ ! -f .secrets.baseline ]; then
        echo "📋 Creating secrets baseline..."
        detect-secrets scan --baseline .secrets.baseline
    fi
    
    # Run secret detection
    detect-secrets-hook --baseline .secrets.baseline --force-use-all-plugins
    if [ $? -ne 0 ]; then
        echo "⚠️  Potential secrets detected! Review and update .secrets.baseline if needed."
        echo "Run: detect-secrets audit .secrets.baseline"
        exit 1
    fi
else
    echo "⚠️  detect-secrets not installed. Install with: pip install detect-secrets"
    echo "For now, checking for common patterns manually..."
    
    # Basic pattern checks for common secrets
    if git diff --cached --name-only | xargs grep -l "aws_access_key_id\|aws_secret_access_key\|private_key\|password.*=" 2>/dev/null; then
        echo "❌ Potential credentials found in staged files!"
        exit 1
    fi
fi

# 3. Check for large files
echo "📦 Checking for large files..."
large_files=$(git diff --cached --name-only | xargs ls -la 2>/dev/null | awk '$5 > 10485760 {print $9, $5}')
if [ -n "$large_files" ]; then
    echo "⚠️  Large files detected (>10MB):"
    echo "$large_files"
    echo "Consider using Git LFS for large files."
fi

# 4. Security-specific file checks
echo "🛡️  Running security-specific checks..."
if git diff --cached --name-only | grep -E "\.(key|pem|p12|pfx|crt|cer|der)$" >/dev/null; then
    echo "❌ Certificate or key files detected in commit!"
    echo "These files should not be committed to the repository."
    exit 1
fi

echo "✅ All pre-commit checks passed!"
