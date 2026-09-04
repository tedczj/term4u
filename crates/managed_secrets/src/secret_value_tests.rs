use crate::secret_value::ManagedSecretValue;

/// Test to ensure that `raw_value` secrets are serialized in the format that the server expects.
#[test]
fn test_serialize_raw_value() {
    let secret = ManagedSecretValue::RawValue {
        value: "secret".to_string(),
    };
    let serialized = serde_json::to_string(&secret).expect("failed to serialize");
    assert_eq!(serialized, "{\"value\":\"secret\"}");
}

/// Test to ensure that the [`ManagedSecretValue`] debug representation does not leak the secret value.
#[test]
fn test_debug_representation_no_secrets() {
    let secret = ManagedSecretValue::RawValue {
        value: "secret".to_string(),
    };
    let debug_representation = format!("{:?}", secret);
    assert!(
        !debug_representation.contains("secret"),
        "debug representation contains secret value: {debug_representation}"
    );
}

/// Test to ensure that `anthropic_api_key` secrets are serialized in the format that the server expects.
#[test]
fn test_serialize_anthropic_api_key() {
    let secret = ManagedSecretValue::AnthropicApiKey {
        api_key: "sk-ant-test-key".to_string(),
    };
    let serialized = serde_json::to_string(&secret).expect("failed to serialize");
    assert_eq!(serialized, "{\"api_key\":\"sk-ant-test-key\"}");
}

/// Test to ensure that the [`ManagedSecretValue::AnthropicApiKey`] debug representation does not leak the API key.
#[test]
fn test_debug_representation_no_secrets_anthropic_api_key() {
    let secret = ManagedSecretValue::AnthropicApiKey {
        api_key: "sk-ant-secret-key".to_string(),
    };
    let debug_representation = format!("{:?}", secret);
    assert!(
        !debug_representation.contains("sk-ant-secret-key"),
        "debug representation contains secret value: {debug_representation}"
    );
}

mod validate_field_sizes {
    use crate::secret_value::{
        ENV_VAR_ANTHROPIC_API_KEY, ENV_VAR_OPENAI_API_KEY, MAX_SECRET_FIELD_BYTES,
        ManagedSecretValue,
    };

    const NAME: &str = "my-secret";

    fn oversized_for_key(env_key: &str) -> String {
        "x".repeat(MAX_SECRET_FIELD_BYTES - env_key.len() + 1)
    }

    #[test]
    fn combined_at_limit_is_valid() {
        let name = "K";
        let value = "x".repeat(MAX_SECRET_FIELD_BYTES - 2);
        let secret = ManagedSecretValue::raw_value(value);
        assert!(secret.validate_field_sizes(name).is_ok());
    }

    #[test]
    fn combined_one_over_limit_is_rejected() {
        let name = "K";
        let value = "x".repeat(MAX_SECRET_FIELD_BYTES - 1);
        let secret = ManagedSecretValue::raw_value(value);
        assert!(secret.validate_field_sizes(name).is_err());
    }

    #[test]
    fn raw_value_over_limit_is_rejected() {
        let secret = ManagedSecretValue::raw_value(oversized_for_key(NAME));
        let err = secret.validate_field_sizes(NAME).unwrap_err();
        assert!(err.to_string().contains(&format!("'{NAME}'")));
    }

    #[test]
    fn raw_value_combined_name_and_value_over_limit_is_rejected() {
        let half = MAX_SECRET_FIELD_BYTES / 2;
        let name = "x".repeat(half + 1);
        let value = "y".repeat(half);
        let secret = ManagedSecretValue::raw_value(value);
        assert!(secret.validate_field_sizes(&name).is_err());
    }

    #[test]
    fn anthropic_api_key_over_limit_is_rejected() {
        let secret =
            ManagedSecretValue::anthropic_api_key(oversized_for_key(ENV_VAR_ANTHROPIC_API_KEY));
        let err = secret.validate_field_sizes(NAME).unwrap_err();
        assert!(
            err.to_string()
                .contains(&format!("'{ENV_VAR_ANTHROPIC_API_KEY}'"))
        );
    }

    #[test]
    fn openai_api_key_over_limit_is_rejected() {
        let secret =
            ManagedSecretValue::openai_api_key(oversized_for_key(ENV_VAR_OPENAI_API_KEY), None);
        let err = secret.validate_field_sizes(NAME).unwrap_err();
        assert!(
            err.to_string()
                .contains(&format!("'{ENV_VAR_OPENAI_API_KEY}'"))
        );
    }

    #[test]
    fn valid_secret_passes() {
        let secret = ManagedSecretValue::anthropic_api_key("sk-test");
        assert!(secret.validate_field_sizes(NAME).is_ok());
    }

    #[test]
    fn raw_value_name_at_limit_is_rejected() {
        // A name this long causes `name.len() + 1 == MAX_SECRET_FIELD_BYTES`, which
        // would underflow the usize subtraction without the upfront guard.
        let name = "x".repeat(MAX_SECRET_FIELD_BYTES - 1);
        let secret = ManagedSecretValue::raw_value("value");
        assert!(secret.validate_field_sizes(&name).is_err());
    }
}

/// Test to ensure that `docker_registry` secrets are serialized in the format that the server expects.
#[test]
fn test_serialize_docker_registry() {
    let secret =
        ManagedSecretValue::docker_registry("us-docker.pkg.dev", "_json_key", "secret-pass");
    let serialized = serde_json::to_string(&secret).expect("failed to serialize");
    assert_eq!(
        serialized,
        "{\"registry_host\":\"us-docker.pkg.dev\",\"username\":\"_json_key\",\"password\":\"secret-pass\"}"
    );
}

/// Test to ensure that the [`ManagedSecretValue::DockerRegistry`] debug representation does not leak the password.
#[test]
fn test_debug_representation_no_secrets_docker_registry() {
    let secret =
        ManagedSecretValue::docker_registry("us-docker.pkg.dev", "_json_key", "secret-pass");
    let debug_representation = format!("{:?}", secret);
    assert!(
        !debug_representation.contains("secret-pass"),
        "debug representation contains password: {debug_representation}"
    );
}

/// A registry credential is never injected as an environment variable, so it has no
/// field-size limit to enforce - unlike every other secret type, which does.
#[test]
fn test_docker_registry_field_sizes_never_rejected() {
    let secret = ManagedSecretValue::docker_registry(
        "us-docker.pkg.dev",
        "_json_key",
        "x".repeat(1024 * 1024),
    );
    assert!(secret.validate_field_sizes("my-secret").is_ok());
}
