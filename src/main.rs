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
    // Initialize logging
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        None | Some(Commands::Daemon) => {
            tracing::info!("Starting Scribe daemon");
            run_daemon().await
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

#[allow(clippy::too_many_lines)] // Complex state machine requires many lines
#[allow(clippy::future_not_send)] // Not spawning across threads, runs in main event loop
async fn run_daemon() -> Result<()> {
    tracing::info!("Loading configuration");
    let config = Config::load()?;

    tracing::info!("Initializing components");

    // Initialize transcription backend
    let backend = Backend::from_config(&config.transcription)?;
    tracing::info!("Using transcription backend: {}", backend.backend_name());

    // Initialize text injector
    let mut text_injector = TextInjector::new(config.injection.delay_ms)?;
    tracing::info!(
        "Text injector initialized with {}ms delay",
        config.injection.delay_ms
    );

    // Create channels for IPC communication
    let (command_tx, mut command_rx) = mpsc::channel::<Command>(32);
    let (status_tx, status_rx) = mpsc::channel::<AppStatus>(32);

    // Start IPC server in background
    let ipc_server = IpcServer::new(command_tx.clone(), status_rx)?;
    tokio::spawn(async move {
        if let Err(e) = ipc_server.start().await {
            tracing::error!("IPC server error: {e}");
        }
    });

    // Application state
    let mut app_state = AppState::Idle;
    let mut current_status = AppStatus::Idle;

    // Send initial status
    status_tx.send(current_status.clone()).await.ok();

    tracing::info!("Daemon started, waiting for commands");

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
                        match &mut app_state {
                            AppState::Idle => {
                                // Start recording
                                match start_recording(&config) {
                                    Ok((stream, frames)) => {
                                        tracing::info!("Recording started");
                                        app_state = AppState::Recording { audio_stream: stream, frames };
                                        current_status = AppStatus::Recording;
                                        status_tx.send(current_status.clone()).await.ok();
                                    }
                                    Err(e) => {
                                        tracing::error!("Failed to start recording: {e}");
                                    }
                                }
                            }
                            AppState::Recording { .. } => {
                                // Stop recording and transcribe
                                if let AppState::Recording { audio_stream, frames } =
                                    std::mem::replace(&mut app_state, AppState::Transcribing)
                                {
                                    audio_stream.stop();
                                    tracing::info!("Recording stopped, {} frames collected", frames.len());
                                    current_status = AppStatus::Transcribing;
                                    status_tx.send(current_status.clone()).await.ok();

                                    // Process recording
                                    match process_recording(frames, &config, &backend, &mut text_injector).await {
                                        Ok(Some(text)) => {
                                            tracing::info!("Transcription complete: '{}'", text);
                                        }
                                        Ok(None) => {
                                            tracing::info!("No speech detected in recording");
                                        }
                                        Err(e) => {
                                            tracing::error!("Transcription failed: {e}");
                                        }
                                    }
                                    current_status = AppStatus::Idle;
                                    status_tx.send(current_status.clone()).await.ok();

                                    app_state = AppState::Idle;
                                }
                            }
                            AppState::Transcribing => {
                                tracing::warn!("Cannot toggle while transcribing");
                            }
                        }
                    }

                    Command::Start => {
                        if matches!(app_state, AppState::Idle) {
                            match start_recording(&config) {
                                Ok((stream, frames)) => {
                                    tracing::info!("Recording started");
                                    app_state = AppState::Recording { audio_stream: stream, frames };
                                    current_status = AppStatus::Recording;
                                    status_tx.send(current_status.clone()).await.ok();
                                }
                                Err(e) => {
                                    tracing::error!("Failed to start recording: {e}");
                                }
                            }
                        } else {
                            tracing::warn!("Already recording or transcribing");
                        }
                    }

                    Command::Stop => {
                        if let AppState::Recording { audio_stream, frames } =
                            std::mem::replace(&mut app_state, AppState::Transcribing)
                        {
                            audio_stream.stop();
                            tracing::info!("Recording stopped, {} frames collected", frames.len());
                            current_status = AppStatus::Transcribing;
                            status_tx.send(current_status.clone()).await.ok();

                            // Process recording synchronously
                            match process_recording(frames, &config, &backend, &mut text_injector).await {
                                Ok(Some(text)) => {
                                    tracing::info!("Transcription complete: '{}'", text);
                                }
                                Ok(None) => {
                                    tracing::info!("No speech detected in recording");
                                }
                                Err(e) => {
                                    tracing::error!("Transcription failed: {e}");
                                }
                            }
                            current_status = AppStatus::Idle;
                            status_tx.send(current_status.clone()).await.ok();

                            app_state = AppState::Idle;
                        } else {
                            tracing::warn!("Not currently recording");
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
    let audio_capture =
        AudioCapture::new(config.audio.sample_rate, config.audio.device.as_deref())?;

    let audio_stream = audio_capture.start_recording()?;
    let frames = Vec::new();

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
    tracing::debug!("Processing {} frames with VAD", frames.len());
    let mut vad = VoiceActivityDetector::new(&vad_cfg)?;
    let speech_audio = vad.extract_speech_from_frames(frames)?;

    if let Some(audio) = speech_audio {
        tracing::info!("Speech detected, {} samples, transcribing...", audio.len());

        // Transcribe
        let text = backend.transcribe(&audio).await?;

        if text.trim().is_empty() {
            Ok(None)
        } else {
            // Inject text
            tracing::info!("Injecting text: '{}'", text);
            text_injector.inject(&text)?;
            Ok(Some(text))
        }
    } else {
        tracing::debug!("No speech detected in recording");
        Ok(None)
    }
}

async fn run_client(cmd: Command) -> Result<()> {
    let client = IpcClient::new()?;
    let response = client.send_command(cmd).await?;

    match response {
        Response::Ok => {
            println!("OK");
        }
        Response::Status(status) => {
            println!("{status:?}");
        }
        Response::Error(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }

    Ok(())
}
