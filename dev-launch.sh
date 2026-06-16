#!/bin/bash
# Sentinel launcher with automatic window toggling support

# Get absolute path of the script directory
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$DIR"

# Check if Ollama is running, and start it if it isn't
OLLAMA_PID=""
if curl -s http://localhost:11434/api/tags > /dev/null 2>&1; then
    echo "Ollama is already running."
else
    echo "Ollama is not running. Attempting to start Ollama..."
    if command -v ollama &>/dev/null; then
        ollama serve > /dev/null 2>&1 &
        OLLAMA_PID=$!
        echo "Ollama started (PID: $OLLAMA_PID)."
        echo "Waiting for Ollama to initialize..."
        for i in {1..10}; do
            if curl -s http://localhost:11434/api/tags > /dev/null 2>&1; then
                echo "Ollama is ready."
                break
            fi
            sleep 1
        done
    else
        echo "Warning: 'ollama' command not found. Please install Ollama or ensure it is running."
    fi
fi

# Check if Sentinel UI is already running in development mode
# We look for the actual running Electron binary process (not wrapper scripts)
if pgrep -f "electron/dist/electron" > /dev/null; then
    echo "Sentinel is already running. Toggling window visibility..."
    cd sentinel-ui
    npx electron .
    exit 0
fi

echo "Cleaning up any leaked Sentinel processes..."
# Terminate any dangling/leaked daemons or UI servers to avoid port conflicts
pkill -f "target/debug/sentinel daemon" 2>/dev/null
pkill -f "target/release/sentinel" 2>/dev/null
pkill -f "sentinel-ui/node_modules/.bin/concurrently" 2>/dev/null
pkill -f "wait-and-launch.cjs" 2>/dev/null

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
