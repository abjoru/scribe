use scribe::ipc::{client::IpcClient, server::IpcServer, AppStatus, Command, Response};
use std::path::PathBuf;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{sleep, Duration};

/// Get unique socket path for test
fn get_test_socket_path(test_name: &str) -> PathBuf {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| format!("/run/user/{}", nix::unistd::getuid()));
    PathBuf::from(runtime_dir).join(format!("scribe-test-{test_name}.sock"))
}

#[tokio::test]
async fn test_ipc_communication() {
    let socket_path = get_test_socket_path("ipc_communication");
    let _ = std::fs::remove_file(&socket_path);

    // Set up channels
    let (command_tx, mut command_rx) = mpsc::channel::<Command>(32);
    let (status_tx, status_rx) = mpsc::channel::<AppStatus>(32);
    let (ready_tx, ready_rx) = oneshot::channel();

    // Start server in background
    let server = IpcServer::new(command_tx, status_rx)
        .expect("Failed to create server")
        .with_socket_path(socket_path.clone())
        .with_ready_signal(ready_tx);
    let server_handle = tokio::spawn(async move {
        server.start().await.ok();
    });

    // Send initial status
    status_tx
        .send(AppStatus::Idle)
        .await
        .expect("Failed to send initial status");

    // Wait for server to signal it's started (or timeout)
    tokio::time::timeout(Duration::from_secs(2), ready_rx)
        .await
        .expect("Server didn't start in time")
        .ok();

    // Give a bit more time for socket to be fully ready
    sleep(Duration::from_millis(100)).await;

    // Verify socket exists
    assert!(
        socket_path.exists(),
        "Socket file doesn't exist at {}",
        socket_path.display()
    );

    // Create client and send command
    let client = IpcClient::with_socket_path(socket_path.clone());
    let response = client
        .send_command(Command::Toggle)
        .await
        .expect("Failed to send command");

    // Verify response
    assert_eq!(response, Response::Ok);

    // Verify command received by server
    let received = tokio::time::timeout(Duration::from_secs(1), command_rx.recv())
        .await
        .expect("Timeout waiting for command")
        .expect("Channel closed");
    assert_eq!(received, Command::Toggle);

    // Test status command
    let response = client
        .send_command(Command::Status)
        .await
        .expect("Failed to send status command");

    match response {
        Response::Status(AppStatus::Idle) => {}
        _ => panic!("Expected Status(Idle), got {response:?}"),
    }

    // Clean up
    server_handle.abort();
}

#[tokio::test]
async fn test_multiple_clients() {
    let socket_path = get_test_socket_path("multiple_clients");
    let _ = std::fs::remove_file(&socket_path);

    // Set up channels
    let (command_tx, mut command_rx) = mpsc::channel::<Command>(32);
    let (status_tx, status_rx) = mpsc::channel::<AppStatus>(32);
    let (ready_tx, ready_rx) = oneshot::channel();

    // Start server
    let server = IpcServer::new(command_tx, status_rx)
        .expect("Failed to create server")
        .with_socket_path(socket_path.clone())
        .with_ready_signal(ready_tx);
    let server_handle = tokio::spawn(async move {
        server.start().await.ok();
    });

    status_tx
        .send(AppStatus::Idle)
        .await
        .expect("Failed to send initial status");

    // Wait for server to signal it's started
    tokio::time::timeout(Duration::from_secs(2), ready_rx)
        .await
        .expect("Server didn't start in time")
        .ok();

    sleep(Duration::from_millis(100)).await;

    // Verify socket exists
    assert!(
        socket_path.exists(),
        "Socket file doesn't exist at {}",
        socket_path.display()
    );

    // Create multiple clients and send commands
    let client1 = IpcClient::with_socket_path(socket_path.clone());
    let client2 = IpcClient::with_socket_path(socket_path.clone());

    let resp1 = client1.send_command(Command::Start).await;
    let resp2 = client2.send_command(Command::Stop).await;

    assert!(resp1.is_ok());
    assert!(resp2.is_ok());

    // Verify both commands received
    let cmd1 = tokio::time::timeout(Duration::from_secs(1), command_rx.recv())
        .await
        .expect("Timeout")
        .expect("Channel closed");
    let cmd2 = tokio::time::timeout(Duration::from_secs(1), command_rx.recv())
        .await
        .expect("Timeout")
        .expect("Channel closed");

    assert!(cmd1 == Command::Start || cmd1 == Command::Stop);
    assert!(cmd2 == Command::Start || cmd2 == Command::Stop);
    assert_ne!(cmd1, cmd2);

    // Clean up
    server_handle.abort();
}

#[tokio::test]
async fn test_client_error_daemon_not_running() {
    // Try to connect without daemon running
    let client = IpcClient::new().expect("Failed to create client");
    let result = client.send_command(Command::Toggle).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Could not connect to daemon"));
}
