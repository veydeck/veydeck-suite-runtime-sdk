use std::{
    path::Path,
    sync::{Arc, Mutex},
};
use tempfile::tempdir;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixListener,
};
use veydeck_suite_runtime::{RuntimeClient, StartWorkloadRequest, WorkloadTransport};

const WORKLOAD_FIXTURE: &str = include_str!("fixtures/workloads/start-request.json");

#[test]
fn workload_fixture_matches_rust_wire_types() {
    let shared = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../contracts/fixtures/workloads/start-request.json");
    if shared.exists() {
        assert_eq!(
            std::fs::read_to_string(shared).unwrap().trim(),
            WORKLOAD_FIXTURE.trim()
        );
    }
    let request: StartWorkloadRequest = serde_json::from_str(WORKLOAD_FIXTURE).unwrap();
    assert_eq!(request.ports.len(), 2);
    assert_eq!(request.ports[0].endpoint_id, "dns-tcp");
    assert_eq!(request.ports[0].protocol, WorkloadTransport::Tcp);
    assert_eq!(request.ports[1].protocol, WorkloadTransport::Udp);
    assert_eq!(
        serde_json::to_value(request).unwrap(),
        serde_json::from_str::<serde_json::Value>(WORKLOAD_FIXTURE).unwrap()
    );
}

#[tokio::test]
async fn workload_client_uses_authenticated_v1_routes_and_decodes_responses() {
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
            "suiteId": "veydeck.protocol-simulation",
            "instanceId": "instance-1",
            "endpoint": {
                "kind": "unix",
                "socketPath": socket,
                "baseUrl": "http://local"
            },
            "credential": { "tokenPath": token },
            "capabilities": ["workloads.manage", "captures.manage"]
        })
        .to_string(),
    )
    .unwrap();

    let listener = UnixListener::bind(&socket).unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let captured = Arc::clone(&requests);
    let responses = [
        ("200 OK", r#"{"workloadId":"workload-1","containerId":"container-1","status":"running"}"#.as_bytes()),
        ("200 OK", r#"[{"workloadId":"workload-1","suiteId":"veydeck.protocol-simulation","suiteInstanceId":"instance-1","workloadKind":"simulation-rule","name":"dns-decoy","image":"engine:alpha","status":"running","containerId":"container-1"}]"#.as_bytes()),
        ("200 OK", r#"{"captureId":"capture-1","status":"capturing","endpoints":[{"endpointId":"dns-udp","hostPort":1053,"protocol":"udp"}]}"#.as_bytes()),
        ("200 OK", b"pcap".as_slice()),
        ("204 No Content", b"".as_slice()),
    ];
    let server = tokio::spawn(async move {
        for (status, body) in responses {
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
            captured
                .lock()
                .unwrap()
                .push(String::from_utf8_lossy(&request).into_owned());
            stream
                .write_all(
                    format!(
                        "HTTP/1.1 {status}\r\ncontent-length: {}\r\ncontent-type: application/json\r\n\r\n",
                        body.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
            stream.write_all(body).await.unwrap();
        }
    });

    let client = RuntimeClient::from_path(&descriptor, "workloads.manage")
        .await
        .unwrap();
    let payload: StartWorkloadRequest = serde_json::from_str(WORKLOAD_FIXTURE).unwrap();
    assert_eq!(
        client.start_workload(&payload).await.unwrap().workload_id,
        "workload-1"
    );
    assert_eq!(
        client.list_workloads().await.unwrap()[0].container_id,
        "container-1"
    );
    let capture = client.start_capture("workload-1").await.unwrap();
    assert_eq!(capture.endpoints[0].protocol, WorkloadTransport::Udp);
    assert_eq!(
        client
            .finish_capture("workload-1", "capture-1")
            .await
            .unwrap(),
        b"pcap"
    );
    client.delete_workload("workload-1").await.unwrap();
    server.await.unwrap();

    let requests = requests.lock().unwrap();
    let request_lines = requests
        .iter()
        .map(|request| request.lines().next().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        request_lines,
        [
            "POST /api/v1/agent/suite-runtime/workloads HTTP/1.1",
            "GET /api/v1/agent/suite-runtime/workloads HTTP/1.1",
            "POST /api/v1/agent/suite-runtime/workloads/workload-1/captures HTTP/1.1",
            "POST /api/v1/agent/suite-runtime/workloads/workload-1/captures/capture-1/finish HTTP/1.1",
            "DELETE /api/v1/agent/suite-runtime/workloads/workload-1 HTTP/1.1",
        ]
    );
    assert!(
        requests
            .iter()
            .all(|request| request.contains("authorization: Bearer trusted-token"))
    );
}
