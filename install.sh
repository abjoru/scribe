#!/bin/bash
set -e

echo "Building scribe..."
cargo build --release
strip target/release/scribe

echo "Installing binary..."
install -Dm755 target/release/scribe ~/.local/bin/scribe

echo "Installing config..."
mkdir -p ~/.config/scribe
if [ ! -f ~/.config/scribe/config.toml ]; then
    cp config/default.toml ~/.config/scribe/config.toml
    echo "Created config at ~/.config/scribe/config.toml"
else
    echo "Config already exists at ~/.config/scribe/config.toml (not overwriting)"
fi

echo "Installing systemd service..."
mkdir -p ~/.config/systemd/user
cp scribe.service ~/.config/systemd/user/scribe.service
systemctl --user daemon-reload

echo "Setting up permissions..."
sudo tee /etc/udev/rules.d/99-scribe.rules > /dev/null <<EOF
KERNEL=="uinput", MODE="0660", GROUP="input", OPTIONS+="static_node=uinput"
EOF
sudo udevadm control --reload-rules
sudo udevadm trigger
sudo usermod -aG input $USER

echo ""
echo "==> Installation complete!"
echo ""
echo "Next steps:"
echo "  1. Log out and back in (for group change to take effect)"
echo "  2. Edit config: \$EDITOR ~/.config/scribe/config.toml"
echo ""
echo "  3. If using OpenAI API backend with systemd:"
echo "     a) Create API key file:"
echo "        echo 'OPENAI_API_KEY=sk-your-key-here' > ~/.config/scribe/api-key.env"
echo "        chmod 600 ~/.config/scribe/api-key.env"
echo ""
echo "     b) Edit systemd service to add this line in [Service] section:"
echo "        vim ~/.config/systemd/user/scribe.service"
echo "        # Add: EnvironmentFile=%h/.config/scribe/api-key.env"
echo ""
echo "     c) Reload systemd:"
echo "        systemctl --user daemon-reload"
echo ""
echo "  4. Run: scribe"
echo ""
echo "Optional: Enable systemd service to start on login:"
echo "  systemctl --user enable --now scribe"
