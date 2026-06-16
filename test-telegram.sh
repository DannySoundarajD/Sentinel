#!/bin/bash
# Test Telegram Bot Configuration

echo "🤖 Telegram Bot Test Script"
echo "============================"
echo ""

CONFIG_FILE="$HOME/.local/share/sentinx/sentinel/config.toml"

if [ ! -f "$CONFIG_FILE" ]; then
    echo "❌ Config file not found at: $CONFIG_FILE"
    echo "Please save your Telegram token in Settings first."
    exit 1
fi

echo "📄 Checking config file..."
if grep -q "\[telegram\]" "$CONFIG_FILE"; then
    echo "✓ Telegram section found in config"
    
    TOKEN=$(grep "bot_token" "$CONFIG_FILE" | cut -d'"' -f2)
    if [ -n "$TOKEN" ] && [ "$TOKEN" != "" ]; then
        echo "✓ Bot token configured (${#TOKEN} characters)"
        echo ""
        echo "Token preview: ${TOKEN:0:10}..."
        echo ""
        echo "📡 Testing Telegram API connection..."
        
        RESPONSE=$(curl -s "https://api.telegram.org/bot${TOKEN}/getMe")
        
        if echo "$RESPONSE" | grep -q '"ok":true'; then
            BOT_USERNAME=$(echo "$RESPONSE" | grep -o '"username":"[^"]*"' | cut -d'"' -f4)
            BOT_NAME=$(echo "$RESPONSE" | grep -o '"first_name":"[^"]*"' | cut -d'"' -f4)
            echo "✓ Bot is valid!"
            echo "  Bot Name: $BOT_NAME"
            echo "  Bot Username: @$BOT_USERNAME"
            echo ""
            echo "✅ Configuration is correct!"
            echo ""
            echo "Next steps:"
            echo "1. Open Telegram and search for @$BOT_USERNAME"
            echo "2. Send /start or /help to your bot"
            echo "3. Check that Sentinel daemon is running (./dev-launch.sh)"
            echo ""
            echo "Available commands:"
            echo "  /help - Show all commands"
            echo "  /status - System metrics"
            echo "  /code python print('hello') - Execute code"
            echo "  /history - View chat history"
        else
            echo "❌ Bot token is invalid or bot doesn't exist"
            echo ""
            echo "Response: $RESPONSE"
            echo ""
            echo "Please check:"
            echo "1. Token is correct (get it from @BotFather)"
            echo "2. Bot hasn't been deleted"
            echo "3. Internet connection is working"
        fi
    else
        echo "❌ Bot token is empty"
        echo "Please add your bot token in Settings"
    fi
else
    echo "❌ No Telegram configuration found"
    echo ""
    echo "To configure Telegram:"
    echo "1. Open Sentinel Settings"
    echo "2. Enable Telegram Bridge"
    echo "3. Enter your bot token from @BotFather"
    echo "4. Click Save Configuration"
fi
