// Integration tests for spkrd server using temporary files as mock devices

use std::fs;
use std::time::Duration;
use tempfile::NamedTempFile;

#[tokio::test]
async fn test_server_with_file_device() {
    // Create a temporary file to act as the speaker device
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let device_path = temp_file.path().to_string_lossy().to_string();
    
    // Start the server on a random available port
    let port = find_available_port().await;
    let server_handle = tokio::spawn(async move {
        let _ = spkrd::server::run(port, Duration::from_secs(30), device_path).await;
    });

    // Wait a moment for the server to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Send a melody to the server
    let melody = "cdefgab";
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/play", port);
    
    let response = client
        .put(&url)
        .body(melody)
        .send()
        .await
        .expect("Failed to send request");

    // Verify the response is successful
    assert_eq!(response.status(), 200);
    assert_eq!(response.text().await.unwrap(), "");

    // Verify the melody was written to the file
    let file_contents = fs::read_to_string(temp_file.path()).expect("Failed to read temp file");
    assert_eq!(file_contents, melody);

    // Clean up: abort the server
    server_handle.abort();
}

#[tokio::test]
async fn test_melody_validation() {
    // Create a temporary file to act as the speaker device
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let device_path = temp_file.path().to_string_lossy().to_string();
    
    // Start the server on a random available port
    let port = find_available_port().await;
    let server_handle = tokio::spawn(async move {
        let _ = spkrd::server::run(port, Duration::from_secs(30), device_path).await;
    });

    // Wait a moment for the server to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Send a melody that's too long (> 1000 characters)
    let melody = "c".repeat(1001);
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/play", port);
    
    let response = client
        .put(&url)
        .body(melody)
        .send()
        .await
        .expect("Failed to send request");

    // Verify the response is a bad request
    assert_eq!(response.status(), 400);
    let error_message = response.text().await.unwrap();
    assert!(error_message.contains("exceeds 1000 characters"));

    // Verify nothing was written to the file
    let file_contents = fs::read_to_string(temp_file.path()).expect("Failed to read temp file");
    assert_eq!(file_contents, "");

    // Clean up: abort the server
    server_handle.abort();
}

#[tokio::test]
async fn test_multiple_requests() {
    // Create a temporary file to act as the speaker device
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let device_path = temp_file.path().to_string_lossy().to_string();
    
    // Start the server on a random available port
    let port = find_available_port().await;
    let server_handle = tokio::spawn(async move {
        let _ = spkrd::server::run(port, Duration::from_secs(30), device_path).await;
    });

    // Wait a moment for the server to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/play", port);
    
    // Send first melody
    let melody1 = "cdefg";
    let response1 = client
        .put(&url)
        .body(melody1)
        .send()
        .await
        .expect("Failed to send first request");
    assert_eq!(response1.status(), 200);
    
    // Send second melody
    let melody2 = "abcde";
    let response2 = client
        .put(&url)
        .body(melody2)
        .send()
        .await
        .expect("Failed to send second request");
    assert_eq!(response2.status(), 200);

    // The file should contain the last melody written
    let file_contents = fs::read_to_string(temp_file.path()).expect("Failed to read temp file");
    assert_eq!(file_contents, melody2);

    // Clean up: abort the server
    server_handle.abort();
}

#[tokio::test]
async fn test_invalid_utf8() {
    // Create a temporary file to act as the speaker device
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let device_path = temp_file.path().to_string_lossy().to_string();
    
    // Start the server on a random available port
    let port = find_available_port().await;
    let server_handle = tokio::spawn(async move {
        let _ = spkrd::server::run(port, Duration::from_secs(30), device_path).await;
    });

    // Wait a moment for the server to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Send invalid UTF-8 data
    let invalid_utf8 = vec![0xFF, 0xFE, 0xFD];
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/play", port);
    
    let response = client
        .put(&url)
        .body(invalid_utf8)
        .send()
        .await
        .expect("Failed to send request");

    // Verify the response is a bad request
    assert_eq!(response.status(), 400);
    let error_message = response.text().await.unwrap();
    assert!(error_message.contains("Invalid UTF-8"));

    // Verify nothing was written to the file
    let file_contents = fs::read_to_string(temp_file.path()).expect("Failed to read temp file");
    assert_eq!(file_contents, "");

    // Clean up: abort the server
    server_handle.abort();
}

// Helper function to find an available port
async fn find_available_port() -> u16 {
    use tokio::net::TcpListener;
    
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}