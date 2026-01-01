#![allow(clippy::multiple_crate_versions)] // TODO: Resolve dependency conflicts in Phase 1+

use clap::{Parser, Subcommand};
use scribe::audio::{capture::AudioCapture, vad::VadConfig, vad::VoiceActivityDetector};
use scribe::config::Config;
use scribe::error::{Result, ScribeError};
use scribe::input::inject::TextInjector;
use scribe::ipc::{client::IpcClient, server::IpcServer, AppStatus, Command, Response};
use scribe::transcription::Backend;
use tokio::signal;
use tokio::sync::mpsc;

#[derive(Parser)]
#[command(name = "scribe")]
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
    let backend = Backend::from_config(&config.transcription)?;
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

    // Application state
    let mut app_state = AppState::Idle;
    let mut current_status = AppStatus::Idle;

    // Send initial status
    status_tx.send(current_status.clone()).await.ok();

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
                                        status_tx.send(current_status.clone()).await.ok();
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
                                    status_tx.send(current_status.clone()).await.ok();

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
                                    status_tx.send(current_status.clone()).await.ok();

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
                                    status_tx.send(current_status.clone()).await.ok();
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
                            status_tx.send(current_status.clone()).await.ok();

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
                            status_tx.send(current_status.clone()).await.ok();

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
                    _ => None,
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
    // Convert config to audio::vad::VadConfig
    let vad_cfg = VadConfig {
        sample_rate: config.audio.sample_rate,
        aggressiveness: config.vad.aggressiveness,
        silence_ms: config.vad.silence_ms,
        min_duration_ms: config.vad.min_duration_ms,
        skip_initial_ms: config.vad.skip_initial_ms,
    };

    // Extract speech using VAD
    tracing::debug!(
        frame_count = frames.len(),
        aggressiveness = vad_cfg.aggressiveness,
        "Running VAD on recorded frames"
    );
    let mut vad = VoiceActivityDetector::new(&vad_cfg)?;
    let speech_audio = vad.extract_speech_from_frames(frames)?;

    if let Some(audio) = speech_audio {
        #[allow(clippy::cast_precision_loss)]
        let duration_s = audio.len() as f32 / config.audio.sample_rate as f32;
        tracing::info!(
            sample_count = audio.len(),
            duration_s = %format!("{duration_s:.2}"),
            "Speech detected, starting transcription"
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
    } else {
        tracing::debug!("VAD detected no speech in recording");
        Ok(None)
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
