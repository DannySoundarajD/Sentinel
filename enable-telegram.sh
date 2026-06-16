#!/bin/bash
# Enable Telegram Bot Integration for Sentinel

CONFIG_DIR="$HOME/.local/share/sentinx/sentinel"
CONFIG_FILE="$CONFIG_DIR/config.toml"

echo "🤖 Sentinel Telegram Bot Setup"
echo "================================"
echo ""

# Create config directory if it doesn't exist
mkdir -p "$CONFIG_DIR"

# Check if config exists
if [ ! -f "$CONFIG_FILE" ]; then
    echo "⚠️  No config file found. Creating default config..."
    cat > "$CONFIG_FILE" << 'EOF'
[agent]
name = "Sentinel"
model = "gemma:2b"
temperature = 0.7

[vault]
memory_mode = "lite"
db_path = "$HOME/.local/share/sentinx/sentinel/vault.db"

[runtime]
ollama_host = "http://localhost:11434"
default_model = "gemma:2b"
fallback_model = "qwen:0.5b"
resource_profile = "balanced"

[guardian]
enable = true
interval_secs = 10
EOF
fi

# Prompt for Telegram bot token
echo ""
echo "To enable Telegram integration, you need a bot token from @BotFather"
echo "Steps:"
echo "  1. Open Telegram and search for @BotFather"
echo "  2. Send /newbot and follow instructions"
echo "  3. Copy the bot token (looks like: 123456789:ABCdefGHIjklMNOpqrsTUVwxyz)"
echo ""
read -p "Enter your Telegram bot token (or press Enter to skip): " BOT_TOKEN

if [ -z "$BOT_TOKEN" ]; then
    echo "❌ No token provided. Telegram integration not enabled."
    exit 0
fi

# Add or update telegram section in config
if grep -q "\[telegram\]" "$CONFIG_FILE"; then
    echo "⚠️  Telegram section already exists. Updating..."
    sed -i "/\[telegram\]/,/^$/d" "$CONFIG_FILE"
fi

echo "" >> "$CONFIG_FILE"
echo "[telegram]" >> "$CONFIG_FILE"
echo "bot_token = \"$BOT_TOKEN\"" >> "$CONFIG_FILE"
echo "allowed_users = []" >> "$CONFIG_FILE"

echo ""
echo "✓ Telegram bot enabled!"
echo ""
echo "Available commands in Telegram:"
echo "  /help - Show all commands"
echo "  /status - System metrics"
echo "  /code <lang> <code> - Execute code (py/js/rust/bash)"
echo "  /analyze <code> - Analyze code complexity"
echo "  /format <code> - Format code"
echo "  /history - View chat history"
echo "  /save <fact> - Save to memory vault"
echo "  /new - Start new session"
echo ""
echo "Restart Sentinel daemon to apply changes:"
echo "  pkill sentinel && ./target/release/sentinel daemon"
echo ""
