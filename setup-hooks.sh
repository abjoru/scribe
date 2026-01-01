#!/bin/bash
# Setup git hooks for Scribe development

echo "Setting up git hooks..."
git config core.hooksPath .git-hooks
chmod +x .git-hooks/pre-commit

echo "✅ Git hooks configured successfully!"
echo "Pre-commit hook will run: cargo fmt --check, cargo clippy, cargo test"
