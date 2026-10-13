use std::path::Path;
use veydeck_suite_runtime::{OperationEvent, RuntimeDescriptor};

fn assert_shared_fixture_matches(relative_path: &str, packaged_fixture: &str) {
    let shared = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../contracts/fixtures")
        .join(relative_path);
    if shared.exists() {
        assert_eq!(
            std::fs::read_to_string(shared).unwrap().trim(),
            packaged_fixture.trim()
        );
    }
}

#[test]
fn shared_fixtures_match_rust_types() {
    let runtime = include_str!("fixtures/runtime/uds-valid.json");
    assert_shared_fixture_matches("runtime/uds-valid.json", runtime);
    let descriptor: RuntimeDescriptor = serde_json::from_str(runtime).unwrap();
    assert_eq!(descriptor.schema_version, 1);
    assert!(
        descriptor
            .require_identity("veydeck.host-scanner", "instance-1")
            .is_ok()
    );
    assert!(
        descriptor
            .require_identity("veydeck.packet", "instance-1")
            .is_err()
    );
    let https = include_str!("fixtures/runtime/https-valid.json");
    assert_shared_fixture_matches("runtime/https-valid.json", https);
    let https: RuntimeDescriptor = serde_json::from_str(https).unwrap();
    assert_eq!(https.suite_id, "veydeck.packet");

    let event = include_str!("fixtures/operation-events/success.json");
    assert_shared_fixture_matches("operation-events/success.json", event);
    let event: OperationEvent = serde_json::from_str(event).unwrap();
    assert_eq!(event.event_code, "scan_submitted");
}
