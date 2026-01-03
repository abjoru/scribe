# Scribe Troubleshooting Guide

This guide covers common issues and their solutions.

## Table of Contents

- [Recording Issues](#recording-issues)
- [Audio Issues](#audio-issues)
- [Text Injection Issues](#text-injection-issues)
- [IPC Connection Issues](#ipc-connection-issues)
- [Transcription Issues](#transcription-issues)
  - [Model Management](#local-model-model-not-found)
- [Configuration Issues](#configuration-issues)
- [Permission Issues](#permission-issues)
- [Performance Issues](#performance-issues)
- [Debug Logging](#debug-logging)
- [Model Management Tips](#model-management-tips)
- [Getting Help](#getting-help)

---

## Recording Issues

### I accidentally started recording

**Symptoms:**
- Recording started unintentionally
- Don't want to waste transcription resources on garbage audio
- Need to abort without transcribing

**Solutions:**

1. **Cancel the recording (recommended):**
   ```bash
   scribe cancel
   ```
   This immediately discards the audio without transcription.

2. **Difference between Stop and Cancel:**
   - `scribe stop` - Processes audio → transcription → text injection
   - `scribe cancel` - Discards audio immediately, no transcription

3. **Set up a cancel keybinding:**
   ```haskell
   -- In your xmonad.hs
   , ("<F9>", spawn "scribe toggle")      -- Toggle recording
   , ("S-<F9>", spawn "scribe cancel")    -- Cancel with Shift+F9
   ```

**Note:** Cancel only works while actively recording (red microphone icon in tray). It has no effect in other states.

---

## Audio Issues

### No audio device found

**Symptoms:**
```
Audio device error: Failed to get default input device
```

**Solutions:**

1. **Check available devices:**
   ```bash
   arecord -L
   ```

2. **Specify a device explicitly in config:**
   ```toml
   [audio]
   device = "default"  # or "hw:0,0", "plughw:0,0", etc.
   ```

3. **Check audio permissions:**
   ```bash
   # Add yourself to audio group
   sudo usermod -a -G audio $USER

   # Log out and back in
   ```

4. **Test audio capture:**
   ```bash
   arecord -d 5 -f S16_LE -r 16000 test.wav
   ```

### Audio device busy

**Symptoms:**
```
Audio device error: Device or resource busy
```

**Solutions:**

1. **Check if another app is using the mic:**
   ```bash
   lsof /dev/snd/*
   ```

2. **Kill processes using audio:**
   ```bash
   # Identify and stop the process
   fuser -v /dev/snd/*
   ```

3. **Use a different device:**
   See "No audio device found" above.

### Poor audio quality / Not detecting speech

**Symptoms:**
- VAD not detecting speech
- Transcriptions are empty or incorrect

**Solutions:**

1. **Check microphone levels:**
   ```bash
   alsamixer  # Press F4 for capture devices
   ```

2. **Test recording quality:**
   ```bash
   arecord -d 5 -f S16_LE -r 16000 test.wav
   aplay test.wav
   ```

3. **Adjust VAD settings in config:**
   ```toml
   [vad]
   aggressiveness = 1  # Try lower (0-1) if too aggressive
   silence_ms = 1200   # Try higher if cutting off too soon
   min_duration_ms = 300  # Try lower if rejecting short utterances
   ```

4. **Check sample rate:**
   ```toml
   [audio]
   sample_rate = 16000  # Must be 16000 for Whisper
   ```

---

## Text Injection Issues

### dotool not found

**Symptoms:**
```
Text injection error: dotool binary not found in PATH
```

**Solutions:**

1. **Install dotool:**
   - See installation guide: https://sr.ht/~geb/dotool/
   - Or build from source: https://git.sr.ht/~geb/dotool

2. **Verify dotool is in PATH:**
   ```bash
   which dotool
   # Should output: /usr/local/bin/dotool or similar
   ```

3. **Test dotool manually:**
   ```bash
   echo "type hello world" | dotool
   ```

### Permission denied for /dev/uinput

**Symptoms:**
```
Text injection error: Permission denied
```

**Solutions:**

1. **Check uinput permissions:**
   ```bash
   ls -l /dev/uinput
   # Should show: crw-rw---- 1 root input
   ```

2. **Add udev rule:**
   ```bash
   echo 'KERNEL=="uinput", GROUP="input", MODE="0660"' | \
       sudo tee /etc/udev/rules.d/99-input.rules
   ```

3. **Add yourself to input group:**
   ```bash
   sudo usermod -a -G input $USER
   ```

4. **Reload udev and reboot:**
   ```bash
   sudo udevadm control --reload-rules
   sudo udevadm trigger
   # Then log out and back in
   ```

5. **Load uinput module:**
   ```bash
   sudo modprobe uinput

   # Make persistent:
   echo "uinput" | sudo tee /etc/modules-load.d/uinput.conf
   ```

### Text not injecting / Wrong window

**Symptoms:**
- Text appears in wrong application
- Text doesn't appear at all

**Solutions:**

1. **Ensure target window has focus before stopping recording**

2. **Increase injection delay:**
   ```toml
   [injection]
   delay_ms = 10  # Try higher if text is getting lost
   ```

3. **Test dotool focus:**
   ```bash
   # Click on a text editor, then run:
   echo "type test text" | dotool
   ```

---

## IPC Connection Issues

### Cannot connect to daemon

**Symptoms:**
```
IPC error: Connection refused

Troubleshooting:
- Is the daemon running? Start with: scribe
- Check socket path: $XDG_RUNTIME_DIR/scribe.sock
- Try restarting the daemon
```

**Solutions:**

1. **Check if daemon is running:**
   ```bash
   scribe status
   ```

2. **Start the daemon:**
   ```bash
   scribe
   ```

3. **Check socket file:**
   ```bash
   ls -l $XDG_RUNTIME_DIR/scribe.sock
   # Should exist when daemon is running
   ```

4. **Kill and restart daemon:**
   ```bash
   pkill scribe
   scribe
   ```

5. **Check for errors in daemon logs:**
   ```bash
   # Run daemon in foreground with debug logging
   RUST_LOG=debug scribe
   ```

### Socket already exists

**Symptoms:**
```
IPC error: Address already in use
```

**Solutions:**

1. **Remove stale socket:**
   ```bash
   rm $XDG_RUNTIME_DIR/scribe.sock
   scribe
   ```

2. **Kill existing daemon:**
   ```bash
   pkill scribe
   scribe
   ```

---

## Transcription Issues

### OpenAI API: Invalid API key

**Symptoms:**
```
Invalid API key

Troubleshooting:
- Check OPENAI_API_KEY environment variable
- Verify API key at: https://platform.openai.com/api-keys
- Ensure api_key_env is set correctly in config
```

**Solutions:**

1. **Set API key environment variable:**
   ```bash
   export OPENAI_API_KEY="sk-..."

   # Make persistent:
   echo 'export OPENAI_API_KEY="sk-..."' >> ~/.bashrc
   ```

2. **Verify API key is valid:**
   - Visit https://platform.openai.com/api-keys
   - Check key hasn't been revoked

3. **Check config:**
   ```toml
   [transcription]
   backend = "openai"
   api_key_env = "OPENAI_API_KEY"  # Name of env var
   ```

### OpenAI API: Quota exceeded

**Symptoms:**
```
API quota exceeded

Troubleshooting:
- Check your OpenAI account quota and billing
- Consider using local backend: set backend = "local" in config
- Reduce API usage or upgrade your plan
```

**Solutions:**

1. **Check OpenAI account:**
   - Visit https://platform.openai.com/account/billing

2. **Switch to local backend:**
   ```toml
   [transcription]
   backend = "local"
   model = "base"  # or "tiny" for faster
   ```

3. **Reduce usage:**
   - Use local backend for development/testing
   - Use API only for production

### Local model: Download failed

**Symptoms:**
```
Model loading error: Failed to download model

Troubleshooting:
- Ensure sufficient disk space in ~/.cache/huggingface/
- Check internet connection for model download
- Try a smaller model (tiny or base)
```

**Solutions:**

1. **Check disk space:**
   ```bash
   df -h ~/.cache
   # Ensure several GB free
   ```

2. **Check internet connection:**
   ```bash
   ping huggingface.co
   ```

3. **Download model manually:**
   ```bash
   scribe model download base
   # Or try smaller model:
   scribe model download tiny
   ```

4. **List available models:**
   ```bash
   scribe model list-available
   ```

5. **Clear cache and retry:**
   ```bash
   rm -rf ~/.cache/huggingface/hub/models--openai--whisper-*
   scribe model download base
   ```

6. **Check Hugging Face Hub status:**
   - Visit https://status.huggingface.co/

### Local model: Model not found

**Symptoms:**
```
Model error: Model 'base' not found
```

**Solutions:**

1. **List installed models:**
   ```bash
   scribe model list
   ```

2. **Download the model:**
   ```bash
   scribe model download base
   ```

3. **Check available models:**
   ```bash
   scribe model list-available
   ```

4. **Set a different model:**
   ```bash
   scribe model set tiny  # Use already-installed model
   ```

### Local model: Out of memory

**Symptoms:**
- System becomes unresponsive
- OOM killer terminates scribe

**Solutions:**

1. **Use smaller model:**
   ```toml
   [transcription]
   model = "tiny"   # ~75MB
   # Or "base"      # ~150MB
   # Instead of "large" (~3GB)
   ```

2. **Use CPU instead of CUDA:**
   ```toml
   [transcription]
   device = "cpu"
   ```

3. **Close other applications**

4. **Add swap space** (if needed)

### Transcription is empty

**Symptoms:**
- Recording succeeds but no text appears
- Logs show "No speech detected"

**Solutions:**

1. **Check audio quality** (see Audio Issues above)

2. **Adjust VAD settings:**
   ```toml
   [vad]
   aggressiveness = 1      # Lower = less aggressive
   min_duration_ms = 300   # Accept shorter utterances
   ```

3. **Test with longer, clearer speech**

4. **Enable debug logging:**
   ```bash
   RUST_LOG=debug scribe
   ```

---

## Configuration Issues

### Config file not found

**Symptoms:**
```
Config error: Failed to read config file
```

**Solutions:**

1. **Create config directory:**
   ```bash
   mkdir -p ~/.config/scribe
   ```

2. **Copy default config:**
   ```bash
   cp config/default.toml ~/.config/scribe/config.toml
   ```

3. **Or just use defaults** (no config file needed)

### Invalid configuration value

**Symptoms:**
```
Config error: Invalid sample_rate: 44100. Must be one of: [8000, 16000, 48000]

Troubleshooting:
- Check config file: ~/.config/scribe/config.toml
- See example: config/default.toml
- Run with RUST_LOG=debug for more details
```

**Solutions:**

1. **Check config syntax:**
   ```bash
   cat ~/.config/scribe/config.toml
   ```

2. **Validate TOML:**
   - Use online validator: https://www.toml-lint.com/

3. **Reset to defaults:**
   ```bash
   mv ~/.config/scribe/config.toml ~/.config/scribe/config.toml.bak
   cp config/default.toml ~/.config/scribe/config.toml
   ```

4. **Check error message for specific field**

---

## Permission Issues

### General permission errors

**Solutions:**

1. **Check all required groups:**
   ```bash
   groups $USER
   # Should include: audio, input
   ```

2. **Add to groups:**
   ```bash
   sudo usermod -a -G audio,input $USER
   ```

3. **Log out and back in** (required for group changes)

4. **Verify permissions:**
   ```bash
   # Audio
   ls -l /dev/snd/*

   # uinput
   ls -l /dev/uinput
   ```

---

## Performance Issues

### High CPU usage

**Solutions:**

1. **Use smaller model:**
   ```toml
   [transcription]
   model = "tiny"  # Fastest
   ```

2. **Use API instead of local:**
   ```toml
   [transcription]
   backend = "openai"
   ```

3. **Check for loops in logs:**
   ```bash
   RUST_LOG=debug scribe 2>&1 | grep -i error
   ```

### Slow transcription

**Solutions:**

1. **For local backend:**
   - Use smaller model (tiny or base)
   - Use CUDA if available:
     ```toml
     [transcription]
     device = "cuda"
     ```

2. **For API backend:**
   - Check internet connection
   - Increase timeout:
     ```toml
     [transcription]
     api_timeout_secs = 60
     ```

### High memory usage

**Solutions:**

1. **Use smaller model:**
   ```toml
   [transcription]
   model = "tiny"  # ~75MB vs 3GB for large
   ```

2. **Use API backend** (minimal memory)

---

## Debug Logging

### Enable debug logging

```bash
# All debug logs
RUST_LOG=debug scribe

# Only scribe logs
RUST_LOG=scribe=debug scribe

# Specific module
RUST_LOG=scribe::audio=debug scribe

# Multiple modules
RUST_LOG=scribe::audio=debug,scribe::transcription=debug scribe

# Trace level (very verbose)
RUST_LOG=trace scribe
```

### Log to file

```toml
[logging]
level = "debug"
file = "/tmp/scribe.log"
```

Or redirect stderr:
```bash
scribe 2>&1 | tee scribe.log
```

### Understanding log levels

- **ERROR**: Something failed (action required)
- **WARN**: Unexpected but handled (check if repeated)
- **INFO**: Normal operations (default)
- **DEBUG**: Detailed flow (for troubleshooting)
- **TRACE**: Very detailed (rarely needed)

---

## Model Management Tips

### Pre-download models

Download models before first use to avoid delays:

```bash
# Download your preferred model
scribe model download base

# Or download multiple models
scribe model download tiny
scribe model download base
```

### Switch between models

```bash
# List installed models
scribe model list

# Set active model
scribe model set tiny  # Use faster model

# Get model info
scribe model info base
```

### Clean up disk space

```bash
# List installed models
scribe model list

# Remove unused models
scribe model remove large
scribe model remove medium
```

---

## Getting Help

If you're still having issues:

1. **Enable debug logging:**
   ```bash
   RUST_LOG=debug scribe 2>&1 | tee scribe-debug.log
   ```

2. **Reproduce the issue**

3. **Check the log for errors**

4. **File an issue** with:
   - Scribe version: `scribe --version`
   - Operating system
   - Debug log excerpt
   - Steps to reproduce
   - Expected vs actual behavior

---

## Quick Diagnostic Checklist

Run through this checklist to identify issues:

```bash
# 1. Check Rust/cargo installed
cargo --version

# 2. Check system dependencies
which dotool
arecord -L

# 3. Check permissions
groups $USER  # Should include: audio, input
ls -l /dev/uinput
ls -l /dev/snd/*

# 4. Test audio
arecord -d 3 -f S16_LE -r 16000 test.wav
aplay test.wav

# 5. Test dotool
echo "type test" | dotool

# 6. Check config
cat ~/.config/scribe/config.toml

# 7. Start daemon with debug
RUST_LOG=debug scribe

# 8. Test IPC (in another terminal)
scribe status
scribe toggle

# 9. Check models
scribe model list
scribe model list-available
```
