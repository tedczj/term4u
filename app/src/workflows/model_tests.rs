use super::{ArgumentType, Workflow};

#[test]
fn legacy_workflow_json_preserves_local_command_fields() {
    let json = r#"{
        "name": "Deploy local",
        "command": "./deploy --target {{target}}",
        "tags": ["project"],
        "description": "Run the local deployment",
        "arguments": [{
            "name": "target",
            "arg_type": "Text",
            "description": "Deployment target",
            "default_value": "dev",
            "unknown_argument_field": true
        }],
        "source_url": null,
        "author": "Local user",
        "author_url": null,
        "shells": [],
        "unknown_workflow_field": "ignored"
    }"#;

    let workflow: Workflow = serde_json::from_str(json).unwrap();

    assert_eq!(workflow.name(), "Deploy local");
    assert_eq!(workflow.command(), Some("./deploy --target {{target}}"));
    assert_eq!(workflow.arguments().len(), 1);
    assert_eq!(workflow.arguments()[0].arg_type, ArgumentType::Text);
    assert_eq!(workflow.arguments()[0].default_value.as_deref(), Some("dev"));
}

#[test]
fn missing_legacy_optional_fields_have_stable_defaults() {
    let workflow: Workflow = serde_json::from_str(r#"{"name":"List","command":"ls"}"#).unwrap();

    assert_eq!(workflow.tags, Vec::<String>::new());
    assert_eq!(workflow.arguments, Vec::new());
    assert_eq!(workflow.description, None);
    assert_eq!(workflow.environment_variables, None);
}
