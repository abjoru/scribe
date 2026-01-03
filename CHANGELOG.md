# Changelog

All notable changes to scribe will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.5] - 2026-01-03

### Added
- Cancel command to abort recording without transcription
- `scribe cancel` CLI command for discarding audio without wasting transcription resources
- Recording cancellation notification (optional)
- Integration test for cancel command
- Documentation for cancel usage in README and TROUBLESHOOTING

### Changed
- Updated XMonad integration examples with cancel keybinding (Shift+F9)

## [0.1.4] - 2025-12-XX

### Added
- Version flag to CLI (`scribe --version`)

### Changed
- Documentation improvements

## [0.1.3] - 2025-12-XX

### Changed
- Switch to native-tls for AUR compatibility
- Fix ring version conflicts for reproducible builds

## [0.1.2] - 2025-12-XX

### Fixed
- AUR build compatibility issues
- Downgrade rustls to use ring 0.16
- Override CachyOS RUSTFLAGS that break ring

### Changed
- Add Cargo.lock to repository for reproducible builds
- Fix release workflow and flaky integration test

## [0.1.1] - 2025-12-XX

### Added
- GitHub release workflow for AUR compatibility

## [0.1.0] - 2025-12-XX

### Added
- Initial release
- Audio capture via ALSA/cpal
- WebRTC voice activity detection (VAD)
- Whisper transcription (local via Candle or OpenAI API)
- Text injection via dotool
- Unix socket IPC interface
- System tray icon with status indicators
- Desktop notifications
- Model management CLI commands
- Configuration system (TOML)
- XMonad/Polybar integration
- Systemd service support
- AUR package (PKGBUILD)

### Features
- Fast startup (<500ms)
- Lean memory footprint (<50MB idle)
- Smart VAD with scientifically-tuned parameters
- Multiple Whisper model sizes (tiny, base, small, medium, large)
- GPU acceleration support (CUDA)
- Configurable notification preferences
- Comprehensive error handling and logging

[0.1.5]: https://github.com/abjoru/scribe/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/abjoru/scribe/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/abjoru/scribe/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/abjoru/scribe/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/abjoru/scribe/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/abjoru/scribe/releases/tag/v0.1.0
