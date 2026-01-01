# Scribe

Fast, lean, Rust-based voice dictation system

## Status

🚧 **In Development** - Phase 0 (Project Setup) Complete

## Features (Planned)

- **Fast**: Compiled binary, <500ms startup time
- **Lean**: <50MB memory footprint
- **High Quality**: Whisper-based transcription (local or API)
- **Tray-Only UI**: Minimal, non-intrusive interface for Xmonad/Polybar
- **IPC Control**: Unix socket interface for XMonad integration
- **Smart VAD**: WebRTC voice activity detection with scientifically-tuned parameters

## Development Setup

### Prerequisites

- Rust 1.70+ (2021 edition)
- System dependencies:
  - `libasound2-dev` (ALSA)
  - `dotool` (text injection)

### Build

```bash
# Clone the repository
git clone <repo-url>
cd scribe

# Set up git hooks
./setup-hooks.sh

# Build
cargo build

# Run
cargo run
```

### Development Tools

- **Format**: `cargo fmt`
- **Lint**: `cargo clippy --all-targets --all-features`
- **Test**: `cargo test --all-features`
- **Check**: `cargo check`

Pre-commit hooks will automatically run these checks.

### Configuration

Default config at `config/default.toml`. User config will be at `~/.config/scribe/config.toml`.

## Architecture

```
scribe/
├── src/
│   ├── audio/          # Audio capture + VAD
│   ├── transcription/  # Whisper (local/API)
│   ├── ipc/            # Unix socket server/client
│   ├── input/          # Text injection (dotool)
│   ├── config/         # TOML configuration
│   ├── tray/           # System tray icon
│   └── notifications/  # Desktop notifications
```

## Usage (Future)

```bash
# Start daemon
scribe

# From XMonad (toggle recording)
scribe --toggle

# Commands
scribe --start    # Start recording
scribe --stop     # Stop recording
scribe --status   # Get current status
```

## XMonad Integration (Future)

```haskell
-- In xmonad.hs
myKeys =
    [ ((mod4Mask, xK_F9), spawn "scribe --toggle")
    ]
```

## License

TBD

## Documentation

TBD
