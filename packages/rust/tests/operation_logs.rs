use veydeck_suite_runtime::{
    OperationEvent, OperationImpact, OperationOutcome, ParameterValue,
    operation_context_from_header,
};

#[test]
fn rejects_sensitive_parameters() {
    let result = OperationEvent::builder(
        "scan_submitted",
        "提交扫描",
        "Submit scan",
        OperationOutcome::Success,
        OperationImpact::Info,
    )
    .parameter("accessToken", ParameterValue::String("secret".to_string()))
    .build();
    assert!(result.is_err());
}

#[test]
fn operation_context_is_copied_from_trusted_proxy_header() {
    let context = operation_context_from_header(Some(" context-1 ")).unwrap();
    let event = OperationEvent::builder(
        "scan_submitted",
        "提交扫描",
        "Submit scan",
        OperationOutcome::Success,
        OperationImpact::Info,
    )
    .operation_context_id(context)
    .build()
    .unwrap();
    assert_eq!(event.operation_context_id.as_deref(), Some("context-1"));
}
