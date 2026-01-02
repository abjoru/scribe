#![allow(clippy::multiple_crate_versions)] // TODO: Resolve dependency conflicts in Phase 1+

use clap::{Parser, Subcommand};
use scribe::audio::capture::AudioCapture;
use scribe::config::Config;
use scribe::error::{Result, ScribeError};
use scribe::input::inject::TextInjector;
use scribe::ipc::{client::IpcClient, server::IpcServer, AppStatus, Command, Response};
use scribe::transcription::Backend;
use scribe::tray::TrayIcon;
use std::sync::{Arc, Mutex};
use tokio::signal;
use tokio::sync::mpsc;

#[derive(Parser)]
#[command(name = "scribe")]
#[command(version)]
#[command(about = "Fast voice dictation system", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the daemon (default)
    Daemon,
    /// Toggle recording on/off
    Toggle,
    /// Start recording
    Start,
    /// Stop recording
    Stop,
    /// Get current status
    Status,
    /// Manage Whisper models
    Model {
        #[command(subcommand)]
        command: ModelCommands,
    },
}

#[derive(Subcommand)]
enum ModelCommands {
    /// List installed models
    List,
    /// List available models for download
    ListAvailable,
    /// Download a model from `HuggingFace`
    Download { name: String },
    /// Set active model (updates config)
    Set { name: String },
    /// Remove an installed model
    Remove { name: String },
    /// Show detailed information about a model
    Info { name: String },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load config early for logging setup
    let config = Config::load()?;

    // Initialize logging with config-based level
    let log_level = config.logging.level.as_str();
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level));

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .init();

    match cli.command {
        None | Some(Commands::Daemon) => {
            tracing::info!("Starting Scribe daemon");
            run_daemon(config).await
        }
        Some(Commands::Toggle) => run_client(Command::Toggle).await,
        Some(Commands::Start) => run_client(Command::Start).await,
        Some(Commands::Stop) => run_client(Command::Stop).await,
        Some(Commands::Status) => run_client(Command::Status).await,
        Some(Commands::Model { command }) => run_model_command(command).await,
    }
}

/// Application state machine
enum AppState {
    Idle,
    Recording {
        audio_stream: scribe::audio::capture::AudioStream,
        frames: Vec<Vec<i16>>,
    },
    Transcribing,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idle => write!(f, "Idle"),
            Self::Recording { frames, .. } => write!(f, "Recording(frames: {})", frames.len()),
            Self::Transcribing => write!(f, "Transcribing"),
        }
    }
}

#[allow(clippy::too_many_lines)] // Complex state machine requires many lines
#[allow(clippy::future_not_send)] // Not spawning across threads, runs in main event loop
async fn run_daemon(config: Config) -> Result<()> {
    tracing::info!("Initializing components");

    // Initialize transcription backend
    tracing::debug!(
        backend = %config.transcription.backend,
        model = %config.transcription.model,
        "Loading transcription backend"
    );
    let backend = Backend::from_config(&config.transcription).await?;
    tracing::info!(
        backend = %backend.backend_name(),
        "Transcription backend initialized"
    );

    // Initialize text injector
    tracing::debug!(
        method = %config.injection.method,
        delay_ms = config.injection.delay_ms,
        "Initializing text injector"
    );
    let mut text_injector = TextInjector::new(config.injection.delay_ms)?;
    tracing::info!("Text injector initialized");

    // Create channels for IPC communication
    let (command_tx, mut command_rx) = mpsc::channel::<Command>(32);
    let (status_tx, status_rx) = mpsc::channel::<AppStatus>(32);
    tracing::debug!("IPC channels created");

    // Start IPC server in background
    let ipc_server = IpcServer::new(command_tx.clone(), status_rx)?;
    tracing::info!("Starting IPC server");
    tokio::spawn(async move {
        if let Err(e) = ipc_server.start().await {
            tracing::error!(error = %e, "IPC server error");
        }
    });

    // Initialize system tray icon with shared status
    let tray_status = Arc::new(Mutex::new(AppStatus::Idle));
    let tray_icon = TrayIcon::new(Arc::clone(&tray_status));
    tracing::debug!("Creating tray icon service");

    // Create tray service and get handle before spawning
    let service = ksni::TrayService::new(tray_icon);
    let tray_handle = service.handle();

    // Spawn tray service in blocking thread (ksni requires blocking runtime)
    std::thread::spawn(move || {
        tracing::debug!("Tray service thread started");
        if let Err(e) = service.run() {
            tracing::error!(error = %e, "Tray service error");
        }
    });
    tracing::info!("System tray icon initialized");

    // Application state
    let mut app_state = AppState::Idle;
    let mut current_status = AppStatus::Idle;

    // Helper to update both IPC and tray status
    let update_status = |status: AppStatus| {
        // Update tray status and signal refresh
        tray_handle.update(|tray| {
            if let Ok(mut tray_status) = tray.status_handle().lock() {
                *tray_status = status.clone();
            }
        });
        status_tx.send(status)
    };

    // Send initial status
    update_status(current_status.clone()).await.ok();

    tracing::info!("Daemon fully initialized and ready");
    tracing::debug!("Entering main event loop");

    // Set up signal handler for graceful shutdown
    let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())
        .map_err(|e| ScribeError::Other(format!("Failed to register SIGTERM handler: {e}")))?;
    let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())
        .map_err(|e| ScribeError::Other(format!("Failed to register SIGINT handler: {e}")))?;

    // Main event loop
    loop {
        tokio::select! {
            // Handle shutdown signals
            _ = sigterm.recv() => {
                tracing::info!("Received SIGTERM, shutting down gracefully");
                break;
            }
            _ = sigint.recv() => {
                tracing::info!("Received SIGINT (Ctrl+C), shutting down gracefully");
                break;
            }

            // Handle IPC commands
            Some(cmd) = command_rx.recv() => {
                tracing::debug!("Received command: {:?}", cmd);

                match cmd {
                    Command::Toggle => {
                        tracing::debug!(state = ?app_state, "Processing Toggle command");
                        match &mut app_state {
                            AppState::Idle => {
                                // Start recording
                                tracing::info!("Toggle: starting recording");
                                match start_recording(&config) {
                                    Ok((stream, frames)) => {
                                        tracing::info!("Recording started successfully");
                                        app_state = AppState::Recording { audio_stream: stream, frames };
                                        current_status = AppStatus::Recording;
                                        update_status(current_status.clone()).await.ok();
                                    }
                                    Err(e) => {
                                        tracing::error!(error = %e, "Failed to start recording");
                                    }
                                }
                            }
                            AppState::Recording { .. } => {
                                // Stop recording and transcribe
                                tracing::info!("Toggle: stopping recording");
                                if let AppState::Recording { audio_stream, frames } =
                                    std::mem::replace(&mut app_state, AppState::Transcribing)
                                {
                                    audio_stream.stop();
                                    tracing::info!(
                                        frame_count = frames.len(),
                                        "Recording stopped, processing audio"
                                    );
                                    current_status = AppStatus::Transcribing;
                                    update_status(current_status.clone()).await.ok();

                                    // Process recording
                                    match process_recording(frames, &config, &backend, &mut text_injector).await {
                                        Ok(Some(text)) => {
                                            tracing::info!(
                                                text_length = text.len(),
                                                text = %text,
                                                "Transcription and injection successful"
                                            );
                                        }
                                        Ok(None) => {
                                            tracing::info!("No speech detected in recording");
                                        }
                                        Err(e) => {
                                            tracing::error!(error = %e, "Transcription failed");
                                        }
                                    }
                                    current_status = AppStatus::Idle;
                                    update_status(current_status.clone()).await.ok();

                                    app_state = AppState::Idle;
                                    tracing::debug!("Returned to idle state");
                                }
                            }
                            AppState::Transcribing => {
                                tracing::warn!("Ignoring toggle command: currently transcribing");
                            }
                        }
                    }

                    Command::Start => {
                        tracing::debug!(state = ?app_state, "Processing Start command");
                        if matches!(app_state, AppState::Idle) {
                            tracing::info!("Starting recording");
                            match start_recording(&config) {
                                Ok((stream, frames)) => {
                                    tracing::info!("Recording started successfully");
                                    app_state = AppState::Recording { audio_stream: stream, frames };
                                    current_status = AppStatus::Recording;
                                    update_status(current_status.clone()).await.ok();
                                }
                                Err(e) => {
                                    tracing::error!(error = %e, "Failed to start recording");
                                }
                            }
                        } else {
                            tracing::warn!(state = ?app_state, "Cannot start: not in idle state");
                        }
                    }

                    Command::Stop => {
                        tracing::debug!(state = ?app_state, "Processing Stop command");
                        if let AppState::Recording { audio_stream, frames } =
                            std::mem::replace(&mut app_state, AppState::Transcribing)
                        {
                            audio_stream.stop();
                            tracing::info!(
                                frame_count = frames.len(),
                                "Recording stopped, processing audio"
                            );
                            current_status = AppStatus::Transcribing;
                            update_status(current_status.clone()).await.ok();

                            // Process recording synchronously
                            match process_recording(frames, &config, &backend, &mut text_injector).await {
                                Ok(Some(text)) => {
                                    tracing::info!(
                                        text_length = text.len(),
                                        text = %text,
                                        "Transcription and injection successful"
                                    );
                                }
                                Ok(None) => {
                                    tracing::info!("No speech detected in recording");
                                }
                                Err(e) => {
                                    tracing::error!(error = %e, "Transcription failed");
                                }
                            }
                            current_status = AppStatus::Idle;
                            update_status(current_status.clone()).await.ok();

                            app_state = AppState::Idle;
                            tracing::debug!("Returned to idle state");
                        } else {
                            tracing::warn!(state = ?app_state, "Cannot stop: not currently recording");
                        }
                    }

                    Command::Status => {
                        // Status is handled by IPC server directly via status_rx
                    }
                }
            }

            // Collect audio frames while recording
            frame = async {
                match &mut app_state {
                    AppState::Recording { audio_stream, frames } => {
                        audio_stream.recv().await.map(|f| (f, frames))
                    }
                    _ => {
                        // Sleep indefinitely when not recording to avoid busy loop
                        std::future::pending::<Option<(Vec<i16>, &mut Vec<Vec<i16>>)>>().await
                    }
                }
            } => {
                if let Some((frame, frames)) = frame {
                    frames.push(frame);
                    if frames.len() % 100 == 0 {
                        tracing::trace!(frame_count = frames.len(), "Collecting audio frames");
                    }
                }
            }
        }
    }

    // Cleanup
    tracing::info!("Cleaning up resources");
    text_injector.cleanup();
    tracing::info!("Shutdown complete");

    Ok(())
}

/// Start audio recording
fn start_recording(
    config: &Config,
) -> Result<(scribe::audio::capture::AudioStream, Vec<Vec<i16>>)> {
    tracing::debug!(
        sample_rate = config.audio.sample_rate,
        device = ?config.audio.device,
        "Initializing audio capture"
    );
    let audio_capture =
        AudioCapture::new(config.audio.sample_rate, config.audio.device.as_deref())?;

    let audio_stream = audio_capture.start_recording()?;
    let frames = Vec::new();

    tracing::debug!("Audio stream started");
    Ok((audio_stream, frames))
}

/// Process recorded frames: VAD extraction -> transcription -> text injection
#[allow(clippy::future_not_send)] // Not spawning across threads, runs in main event loop
async fn process_recording(
    frames: Vec<Vec<i16>>,
    config: &Config,
    backend: &Backend,
    text_injector: &mut TextInjector,
) -> Result<Option<String>> {
    // Flatten all frames into single audio buffer (bypass VAD extraction for manual toggle)
    let audio: Vec<i16> = frames.into_iter().flatten().collect();

    #[allow(clippy::cast_precision_loss)]
    let duration_seconds = audio.len() as f32 / config.audio.sample_rate as f32;

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let duration_ms = (duration_seconds * 1000.0) as u32;

    // Check minimum duration
    if duration_ms < config.vad.min_duration_ms {
        tracing::debug!(
            duration_ms,
            min_duration_ms = config.vad.min_duration_ms,
            "Recording too short, discarding"
        );
        return Ok(None);
    }

    tracing::info!(
        sample_count = audio.len(),
        duration_s = %format!("{duration_seconds:.2}"),
        "Processing recording for transcription"
    );

    // Transcribe
    let text = backend.transcribe(&audio).await?;

    if text.trim().is_empty() {
        tracing::debug!("Transcription returned empty text");
        Ok(None)
    } else {
        // Inject text
        tracing::debug!(text = %text, "Injecting transcribed text");
        text_injector.inject(&text)?;
        Ok(Some(text))
    }
}

async fn run_client(cmd: Command) -> Result<()> {
    tracing::debug!(command = ?cmd, "Sending IPC command");
    let client = IpcClient::new()?;
    let response = client.send_command(cmd).await?;

    match response {
        Response::Ok => {
            tracing::debug!("Command successful");
            println!("OK");
        }
        Response::Status(status) => {
            tracing::debug!(status = ?status, "Received status");
            println!("{status:?}");
        }
        Response::Error(e) => {
            tracing::error!(error = %e, "Command failed");
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Handle model management commands
#[allow(clippy::too_many_lines)]
async fn run_model_command(command: ModelCommands) -> Result<()> {
    use scribe::models::{ModelInfo, ModelManager};

    match command {
        ModelCommands::List => {
            let manager = ModelManager::new()?;
            let installed = manager.list_installed();
            let active = manager.get_active();

            if installed.is_empty() {
                println!("No models installed.");
                println!("\nDownload a model:");
                println!("  scribe model download base");
                return Ok(());
            }

            println!("Installed models:\n");
            for model in installed {
                let is_active = active == Some(model.name.as_str());
                let marker = if is_active { " (active)" } else { "" };
                let size_mb = model.size_bytes / 1_000_000;
                println!("  {} - {} MB{}", model.name, size_mb, marker);
            }

            if active.is_none() {
                println!("\nNo active model set. Set one with:");
                println!("  scribe model set <name>");
            }
        }

        ModelCommands::ListAvailable => {
            use scribe::models::registry::MODELS;

            println!("Available models:\n");
            for model in MODELS {
                let recommended = if model.recommended {
                    " (recommended)"
                } else {
                    ""
                };
                println!(
                    "  {:8} - {:>6} MB  {:>5} params  {}{}",
                    model.name, model.size_mb, model.parameters, model.description, recommended
                );
            }

            println!("\nDownload a model:");
            println!("  scribe model download <name>");
        }

        ModelCommands::Download { name } => {
            let mut manager = ModelManager::new()?;

            // Find model info
            let model_info = ModelInfo::find(&name).ok_or_else(|| {
                let suggestion = ModelInfo::suggest(&name);
                let mut msg = format!("Unknown model: '{name}'");
                if let Some(suggestion) = suggestion {
                    msg.push_str("\n  Did you mean: ");
                    msg.push_str(suggestion);
                    msg.push('?');
                }
                msg.push_str("\n  Available: ");
                msg.push_str(&ModelInfo::all_names().join(", "));
                ScribeError::NotFound(msg)
            })?;

            println!(
                "Downloading {} model ({} MB)...",
                model_info.name, model_info.size_mb
            );
            manager.download(model_info).await?;
            println!("✓ Model '{}' downloaded successfully", model_info.name);

            // Suggest setting it as active if no active model
            if manager.get_active().is_none() {
                println!("\nSet as active model:");
                println!("  scribe model set {}", model_info.name);
            }
        }

        ModelCommands::Set { name } => {
            let mut manager = ModelManager::new()?;

            // Check if model exists in registry
            if ModelInfo::find(&name).is_none() {
                let suggestion = ModelInfo::suggest(&name);
                let mut msg = format!("Unknown model: '{name}'");
                if let Some(suggestion) = suggestion {
                    msg.push_str("\n  Did you mean: ");
                    msg.push_str(suggestion);
                    msg.push('?');
                }
                msg.push_str("\n  Available: ");
                msg.push_str(&ModelInfo::all_names().join(", "));
                return Err(ScribeError::NotFound(msg));
            }

            manager.set_active(&name)?;
        }

        ModelCommands::Remove { name } => {
            let mut manager = ModelManager::new()?;
            manager.remove(&name)?;
        }

        ModelCommands::Info { name } => {
            let manager = ModelManager::new()?;

            // Try registry first
            if let Some(info) = ModelInfo::find(&name) {
                println!("Model: {}", info.name);
                println!("Parameters: {}", info.parameters);
                println!("Download size: {} MB", info.size_mb);
                println!("Description: {}", info.description);
                if info.recommended {
                    println!("Status: Recommended");
                }

                // Check if installed
                if let Some(installed) = manager.get_installed_info(&name) {
                    println!("\nInstalled:");
                    println!("  Size: {} MB", installed.size_bytes / 1_000_000);
                    println!("  Downloaded: {}", installed.downloaded_at);
                    if manager.get_active() == Some(&name) {
                        println!("  Active: Yes");
                    }
                } else {
                    println!("\nStatus: Not installed");
                    println!("\nDownload with:");
                    println!("  scribe model download {name}");
                }
            } else {
                let suggestion = ModelInfo::suggest(&name);
                let mut msg = format!("Unknown model: '{name}'");
                if let Some(suggestion) = suggestion {
                    msg.push_str("\n  Did you mean: ");
                    msg.push_str(suggestion);
                    msg.push('?');
                }
                msg.push_str("\n  Available: ");
                msg.push_str(&ModelInfo::all_names().join(", "));
                return Err(ScribeError::NotFound(msg));
            }
        }
    }

    Ok(())
}
