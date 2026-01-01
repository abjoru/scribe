#![allow(clippy::multiple_crate_versions)] // TODO: Resolve dependency conflicts in Phase 1+

use clap::{Parser, Subcommand};
use scribe::error::Result;
use scribe::ipc::{client::IpcClient, server::IpcServer, AppStatus, Command, Response};
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

async fn run_daemon() -> Result<()> {
    // Create channels for IPC communication
    let (command_tx, mut command_rx) = mpsc::channel::<Command>(32);
    let (status_tx, status_rx) = mpsc::channel::<AppStatus>(32);

    // Start IPC server in background
    let ipc_server = IpcServer::new(command_tx, status_rx)?;
    tokio::spawn(async move {
        if let Err(e) = ipc_server.start().await {
            tracing::error!("IPC server error: {e}");
        }
    });

    // Initialize application state
    let mut current_status = AppStatus::Idle;

    // Send initial status
    status_tx.send(current_status.clone()).await.ok();

    tracing::info!("Daemon started, waiting for commands");

    // Main event loop
    loop {
        tokio::select! {
            Some(cmd) = command_rx.recv() => {
                tracing::info!("Received command: {:?}", cmd);
                match cmd {
                    Command::Toggle => {
                        current_status = match current_status {
                            AppStatus::Idle => {
                                tracing::info!("Starting recording");
                                AppStatus::Recording
                            }
                            AppStatus::Recording => {
                                tracing::info!("Stopping recording");
                                AppStatus::Idle
                            }
                            AppStatus::Transcribing => {
                                tracing::warn!("Cannot toggle while transcribing");
                                current_status
                            }
                        };
                        status_tx.send(current_status.clone()).await.ok();
                    }
                    Command::Start => {
                        if matches!(current_status, AppStatus::Idle) {
                            tracing::info!("Starting recording");
                            current_status = AppStatus::Recording;
                            status_tx.send(current_status.clone()).await.ok();
                        } else {
                            tracing::warn!("Already recording or transcribing");
                        }
                    }
                    Command::Stop => {
                        if matches!(current_status, AppStatus::Recording) {
                            tracing::info!("Stopping recording");
                            current_status = AppStatus::Idle;
                            status_tx.send(current_status.clone()).await.ok();
                        } else {
                            tracing::warn!("Not currently recording");
                        }
                    }
                    Command::Status => {
                        // Status is handled by IPC server directly
                    }
                }
            }
        }
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
