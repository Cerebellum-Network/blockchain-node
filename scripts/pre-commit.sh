#!/bin/sh

# Prevent committing badly formatted code and security issues

set -e

echo "🔍 Running pre-commit security checks..."

# 1. Format checking
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

# 2. Security linting with Clippy
echo "🔒 Running security-focused Clippy checks..."
cargo clippy --all-targets --all-features -- \
    -W clippy::suspicious \
    -W clippy::complexity \
    -W clippy::perf \
    -W clippy::nursery \
    -D warnings \
    -D clippy::unwrap_used \
    -D clippy::expect_used \
    -D clippy::panic \
    -D clippy::unimplemented \
    -D clippy::unreachable

if [ $? -ne 0 ]; then
	echo "❌ Fix security-related Clippy warnings before committing."
	exit 1
fi

# 3. Secret detection
echo "🕵️ Scanning for potential secrets..."
if command -v trufflehog >/dev/null 2>&1; then
    trufflehog git file://. --since-commit HEAD~1 --only-verified=false --fail
    if [ $? -ne 0 ]; then
        echo "❌ Potential secrets detected! Please review and remove them."
        exit 1
    fi
else
    echo "⚠️  TruffleHog not installed. Install it for secret scanning: curl -sSfL https://raw.githubusercontent.com/trufflesecurity/trufflehog/main/scripts/install.sh | sh -s -- -b /usr/local/bin"
fi

# 4. Dependency vulnerability check
echo "🛡️ Checking for vulnerable dependencies..."
if command -v cargo-audit >/dev/null 2>&1; then
    cargo audit
    if [ $? -ne 0 ]; then
        echo "❌ Vulnerable dependencies detected! Please update them."
        exit 1
    fi
else
    echo "⚠️  cargo-audit not installed. Install it: cargo install cargo-audit"
fi

# 5. License compliance check
echo "📄 Checking license compliance..."
if command -v cargo-deny >/dev/null 2>&1; then
    cargo deny check licenses
    if [ $? -ne 0 ]; then
        echo "❌ License compliance issues detected!"
        exit 1
    fi
else
    echo "⚠️  cargo-deny not installed. Install it: cargo install cargo-deny"
fi

# 6. Check for TODO/FIXME in security-critical files
echo "🔍 Checking for security TODOs..."
SECURITY_TODOS=$(grep -r -n "TODO\|FIXME\|XXX\|HACK" --include="*.rs" pallets/ runtime/ | grep -i "security\|crypto\|auth\|priv" || true)
if [ -n "$SECURITY_TODOS" ]; then
    echo "⚠️  Security-related TODOs found:"
    echo "$SECURITY_TODOS"
    echo "Please address these before committing to production branches."
fi

echo "✅ All security checks passed!"
