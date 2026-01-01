use clap::{Parser, Subcommand};
use scribe::error::Result;

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
        Some(Commands::Toggle) => run_client(scribe::ipc::Command::Toggle).await,
        Some(Commands::Start) => run_client(scribe::ipc::Command::Start).await,
        Some(Commands::Stop) => run_client(scribe::ipc::Command::Stop).await,
        Some(Commands::Status) => run_client(scribe::ipc::Command::Status).await,
    }
}

#[allow(clippy::unused_async)] // TODO: Will be async when implemented
async fn run_daemon() -> Result<()> {
    // TODO: Implement daemon mode
    tracing::info!("Daemon mode not yet implemented");
    Ok(())
}

#[allow(clippy::unused_async)] // TODO: Will be async when implemented
async fn run_client(cmd: scribe::ipc::Command) -> Result<()> {
    // TODO: Implement client mode
    tracing::info!("Client mode not yet implemented: {:?}", cmd);
    Ok(())
}
