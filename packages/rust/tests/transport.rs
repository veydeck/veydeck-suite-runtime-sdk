use std::sync::{Arc, Mutex};
use tempfile::tempdir;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixListener,
};
use veydeck_suite_runtime::{OperationEvent, OperationImpact, OperationOutcome, RuntimeClient};

#[tokio::test]
async fn uds_retry_reuses_the_same_event_id() {
    let directory = tempdir().unwrap();
    let socket = directory.path().join("agent.sock");
    let token = directory.path().join("token");
    let descriptor = directory.path().join("runtime.json");
    std::fs::write(&token, "trusted-token").unwrap();
    std::fs::write(
        &descriptor,
        serde_json::json!({
            "schemaVersion": 1,
            "platformVersion": "0.1.0-alpha.1",
            "suiteId": "veydeck.host-scanner",
            "instanceId": "instance-1",
            "endpoint": {
                "kind": "unix",
                "socketPath": socket,
                "baseUrl": "http://local"
            },
            "credential": { "tokenPath": token },
            "capabilities": ["operation-logs.write"]
        })
        .to_string(),
    )
    .unwrap();

    let listener = UnixListener::bind(&socket).unwrap();
    let bodies = Arc::new(Mutex::new(Vec::new()));
    let captured = Arc::clone(&bodies);
    let server = tokio::spawn(async move {
        for attempt in 0..3 {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut buffer = [0_u8; 4096];
            loop {
                let read = stream.read(&mut buffer).await.unwrap();
                request.extend_from_slice(&buffer[..read]);
                if read == 0 || request.windows(4).any(|value| value == b"\r\n\r\n") {
                    break;
                }
            }
            let request = String::from_utf8_lossy(&request);
            captured.lock().unwrap().push(request.to_string());
            let status = if attempt < 2 {
                "503 Service Unavailable"
            } else {
                "200 OK"
            };
            stream
                .write_all(format!("HTTP/1.1 {status}\r\ncontent-length: 0\r\n\r\n").as_bytes())
                .await
                .unwrap();
        }
    });

    let client = RuntimeClient::from_path(&descriptor, "operation-logs.write")
        .await
        .unwrap();
    let event = OperationEvent::builder(
        "scan_submitted",
        "提交扫描",
        "Submit scan",
        OperationOutcome::Success,
        OperationImpact::Info,
    )
    .build()
    .unwrap();
    client.submit_operation_event(&event).await.unwrap();
    server.await.unwrap();

    let bodies = bodies.lock().unwrap();
    assert_eq!(bodies.len(), 3);
    assert!(
        bodies
            .iter()
            .all(|request| request.contains(&event.event_id))
    );
    assert!(
        bodies
            .iter()
            .all(|request| request.contains("authorization: Bearer trusted-token"))
    );
}
