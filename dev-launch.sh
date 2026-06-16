#!/bin/bash
# Sentinel launcher with automatic window toggling support

# Get absolute path of the script directory
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$DIR"

# Check if Ollama is running, and start it if it isn't
OLLAMA_PID=""
OLLAMA_TIMEOUT=30  # 30 second timeout for Ollama to start

echo "🔍 Checking for Ollama..."
if curl -s http://localhost:11434/api/tags > /dev/null 2>&1; then
    echo "✓ Ollama is already running."
else
    echo "❌ Ollama is not running. Attempting to start Ollama..."
    if command -v ollama &>/dev/null; then
        echo "🚀 Starting ollama serve..."
        ollama serve > /tmp/ollama.log 2>&1 &
        OLLAMA_PID=$!
        echo "Ollama started (PID: $OLLAMA_PID)."
        echo "⏳ Waiting for Ollama to initialize (max ${OLLAMA_TIMEOUT}s)..."
        
        for i in $(seq 1 $OLLAMA_TIMEOUT); do
            if curl -s http://localhost:11434/api/tags > /dev/null 2>&1; then
                echo "✓ Ollama is ready!"
                break
            fi
            if [ $i -eq $OLLAMA_TIMEOUT ]; then
                echo "❌ Ollama failed to start after ${OLLAMA_TIMEOUT}s"
                echo "Check /tmp/ollama.log for details"
                kill $OLLAMA_PID 2>/dev/null
                exit 1
            fi
            sleep 1
            printf "."
        done
    else
        echo "❌ 'ollama' command not found. Please install Ollama:"
        echo "   curl -fsSL https://ollama.ai/install.sh | sh"
        exit 1
    fi
fi

# Check if Sentinel UI (Electron app) is already running
ELECTRON_PID=$(pgrep -f "sentinel-ui.*electron" | head -n 1)
if [ -n "$ELECTRON_PID" ]; then
    echo "✓ Sentinel is already running (PID: $ELECTRON_PID)"
    echo "💡 Use Super+Space to toggle the window, or kill the process to restart."
    exit 0
fi

echo "Cleaning up any leaked Sentinel processes..."
# Terminate any dangling/leaked daemons or UI servers to avoid port conflicts
pkill -f "target/debug/sentinel daemon" 2>/dev/null
pkill -f "target/release/sentinel" 2>/dev/null

# Set up cleanup trap for background processes on script exit or interruption
cleanup() {
    echo "Shutting down Sentinel..."
    if [ -n "$DAEMON_PID" ]; then
        kill "$DAEMON_PID" 2>/dev/null
    fi
    if [ -n "$OLLAMA_PID" ]; then
        echo "Stopping Ollama..."
        kill "$OLLAMA_PID" 2>/dev/null
    fi
}
trap cleanup EXIT INT TERM

echo "Starting Sentinel Daemon..."
cargo run -- daemon &
DAEMON_PID=$!

echo "Starting Sentinel UI..."
cd sentinel-ui
echo "Building frontend assets..."
npm run build
echo "Launching Electron..."
NODE_ENV=production npx electron .
