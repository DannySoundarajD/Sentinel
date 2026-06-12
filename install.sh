#!/bin/bash
set -e

echo "Installing Sentinel..."
echo ""

# Detect distro and install dependencies
if command -v pacman &>/dev/null; then
    echo "Detected Arch Linux"
    sudo pacman -S --needed --noconfirm xdotool libnotify nodejs npm
elif command -v apt &>/dev/null; then
    echo "Detected Ubuntu/Debian"
    sudo apt update -qq
    sudo apt install -y xdotool libnotify-bin nodejs npm
elif command -v dnf &>/dev/null; then
    echo "Detected Fedora"
    sudo dnf install -y xdotool libnotify nodejs npm
else
    echo "Unsupported distro."
    echo "Please manually install: xdotool, libnotify, nodejs, npm"
    exit 1
fi

# Create directories
echo ""
echo "Creating directories..."
mkdir -p ~/.local/share/sentinx/sentinel
mkdir -p ~/.local/share/sentinx/skills
mkdir -p ~/.config/sentinel
mkdir -p ~/.local/bin

# Copy binary
echo "Installing binary..."
if [ -f "./target/release/sentinel" ]; then
    sudo cp ./target/release/sentinel /usr/local/bin/sentinel
    sudo chmod +x /usr/local/bin/sentinel
else
    echo "Binary not found. Run: cargo build --release"
    exit 1
fi

# Create desktop autostart entry
echo "Setting up autostart..."
mkdir -p ~/.config/autostart
cat > ~/.config/autostart/sentinel.desktop << 'EOF'
[Desktop Entry]
Type=Application
Name=Sentinel
Exec=sentinel-launch
Hidden=false
NoDisplay=false
X-GNOME-Autostart-enabled=true
StartupNotify=false
EOF

# Create launcher script
cat > ~/.local/bin/sentinel-launch << 'EOF'
#!/bin/bash
if ! pgrep -x sentinel > /dev/null 2>&1; then
    sentinel daemon &
    sleep 1
fi
EOF
chmod +x ~/.local/bin/sentinel-launch

# Update PATH if needed
if ! grep -q '\.local/bin' ~/.bashrc 2>/dev/null; then
    echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
fi

if ! grep -q '\.local/bin' ~/.zshrc 2>/dev/null; then
    echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
fi

echo ""
echo "✓ Sentinel installed successfully!"
echo ""
echo "Next steps:"
echo "  1. Start Ollama:       ollama serve &"
echo "  2. Start Sentinel:     sentinel-launch"
echo "  3. (Optional) Reboot for autostart"
