<p align="center">
  <img src="assets/logo.svg" alt="Scribe Logo" width="300">
</p>

<h1 align="center">Scribe</h1>

<p align="center">Fast, lean, Rust-based voice dictation system</p>

## Status

🚧 **In Development** - Phase 0-7 Complete
- ✅ Phase 0: Project setup
- ✅ Phase 1: Audio capture and VAD
- ✅ Phase 2: Unix socket IPC
- ✅ Phase 3: Text injection via dotool
- ✅ Phase 4: Configuration system
- ✅ Phase 5a: OpenAI API transcription backend
- ✅ Phase 5b: Local Whisper (Candle) transcription backend
- ✅ Phase 6: Main application loop
- ✅ Phase 7: System tray icon + notifications

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

#### Transcription Backends

Scribe supports two transcription backends:

**1. Local Backend (Whisper via Candle)**

Uses Hugging Face's Candle framework for pure Rust ML inference. Models are automatically downloaded from Hugging Face Hub on first use.

```toml
[transcription]
backend = "local"
model = "tiny"     # Options: tiny, base, small, medium, large
device = "auto"    # Options: cpu, cuda, auto
language = "en"    # 2-letter ISO code or empty for auto-detect
```

Supported models:
- `tiny` - Fastest, ~75MB, good for real-time
- `base` - Balanced, ~150MB, recommended default
- `small` - Better accuracy, ~500MB
- `medium` - High accuracy, ~1.5GB
- `large` - Best accuracy, ~3GB

Models are cached in `~/.cache/huggingface/hub/` and reused across runs.

**2. OpenAI API Backend**

Uses OpenAI's Whisper API for transcription (requires API key).

```toml
[transcription]
backend = "openai"
api_key_env = "OPENAI_API_KEY"
api_model = "whisper-1"
api_timeout_secs = 30
```

Set your API key:
```bash
export OPENAI_API_KEY="your-key-here"
```

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
