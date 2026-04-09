#!/bin/bash
set -euo pipefail

echo "🔧 Installing Claude Code CLI (native installer)..."
curl -fsSL https://claude.ai/install.sh | bash

echo "📦 Cloning Everything Claude Code..."
git clone --depth 1 https://github.com/affaan-m/everything-claude-code.git /tmp/ecc

echo "📋 Installing rules... (Claude Code plugins cannot distribute rules automatically. Install them manually)"
mkdir -p ~/.claude/rules
cp -r /tmp/ecc/rules/common ~/.claude/rules/
cp -r /tmp/ecc/rules/typescript ~/.claude/rules/
cp -r /tmp/ecc/rules/python ~/.claude/rules/
rm -rf /tmp/ecc

echo "⚙️  Writing Claude settings..."
mkdir -p ~/.claude
cat > ~/.claude/settings.json << 'EOF'
{
  "extraKnownMarketplaces": {
    "ecc": {
      "source": {
        "source": "github",
        "repo": "affaan-m/everything-claude-code"
      }
    }
  },
  "enabledPlugins": {
    "ecc@ecc": true
  },
  "env": {
    "MAX_THINKING_TOKENS": "10000",
    "CLAUDE_AUTOCOMPACT_PCT_OVERRIDE": "50"
  }
}
EOF

echo "✅ Setup complete!"
echo "   コンテナ内で 'claude' を実行して認証してください。"