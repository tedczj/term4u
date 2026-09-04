use std::fmt;

use serde::Serialize;
use warp_graphql::managed_secrets::ManagedSecretType;

/// Maximum length in bytes of a `KEY=VALUE` env string, one less than Linux's `MAX_ARG_STRLEN`
/// (128 KiB). The kernel stores each env string NUL-terminated, so the `KEY=VALUE` content must
/// be strictly shorter than `MAX_ARG_STRLEN` to leave room for the trailing `\0`.
pub(crate) const MAX_SECRET_FIELD_BYTES: usize = 128 * 1024 - 1;

pub(crate) const ENV_VAR_ANTHROPIC_API_KEY: &str = "ANTHROPIC_API_KEY";
pub(crate) const ENV_VAR_OPENAI_API_KEY: &str = "OPENAI_API_KEY";

#[derive(Serialize)]
#[serde(untagged)]
pub enum ManagedSecretValue {
    RawValue {
        value: String,
    },
    AnthropicApiKey {
        api_key: String,
    },
    OpenaiApiKey {
        api_key: String,
        /// Optional base URL for the OpenAI API (e.g. regional endpoints).
        /// When absent, the harness uses the provider's default endpoint.
        #[serde(skip_serializing_if = "Option::is_none")]
        base_url: Option<String>,
    },
    DockerRegistry {
        registry_host: String,
        username: String,
        password: String,
    },
}

impl ManagedSecretValue {
    pub fn raw_value(s: impl Into<String>) -> Self {
        Self::RawValue { value: s.into() }
    }

    pub fn anthropic_api_key(s: impl Into<String>) -> Self {
        Self::AnthropicApiKey { api_key: s.into() }
    }

    /// Construct an OpenAI API key secret value with an optional base URL.
    pub fn openai_api_key(api_key: impl Into<String>, base_url: Option<String>) -> Self {
        Self::OpenaiApiKey {
            api_key: api_key.into(),
            base_url,
        }
    }

    /// Construct a container registry credential secret value.
    pub fn docker_registry(
        registry_host: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self::DockerRegistry {
            registry_host: registry_host.into(),
            username: username.into(),
            password: password.into(),
        }
    }

    /// Returns an error if any env var produced by this secret would exceed [`MAX_SECRET_FIELD_BYTES`] bytes.
    pub fn validate_field_sizes(&self, name: &str) -> anyhow::Result<()> {
        let check = |env_key: &str, value: &str| -> anyhow::Result<()> {
            // Guard against a pathologically long key name causing usize underflow below.
            if env_key.len() + 1 >= MAX_SECRET_FIELD_BYTES {
                anyhow::bail!(
                    "Secret name is too long ({} bytes) to be used as an environment variable \
                     name; the maximum is {} bytes.",
                    env_key.len(),
                    MAX_SECRET_FIELD_BYTES - 2,
                );
            }
            let max_value_len = MAX_SECRET_FIELD_BYTES - env_key.len() - 1 /* '=' */;
            if value.len() > max_value_len {
                anyhow::bail!(
                    "Secret '{env_key}' value is too large to inject as an environment variable \
                     ({} bytes); the maximum is {max_value_len} bytes. Use a shorter value.",
                    value.len()
                );
            }
            Ok(())
        };

        match self {
            ManagedSecretValue::RawValue { value } => check(name, value),
            ManagedSecretValue::AnthropicApiKey { api_key } => {
                check(ENV_VAR_ANTHROPIC_API_KEY, api_key)
            }
            ManagedSecretValue::OpenaiApiKey { api_key, .. } => {
                // base_url goes to a config file, not an env var argument.
                check(ENV_VAR_OPENAI_API_KEY, api_key)
            }
            ManagedSecretValue::DockerRegistry { .. } => {
                // Never injected as an environment variable - it authenticates an image
                // pull, not the agent process - so the MAX_ARG_STRLEN concern this
                // function exists for does not apply.
                Ok(())
            }
        }
    }

    pub fn secret_type(&self) -> ManagedSecretType {
        match self {
            ManagedSecretValue::RawValue { .. } => ManagedSecretType::RawValue,
            ManagedSecretValue::AnthropicApiKey { .. } => ManagedSecretType::AnthropicApiKey,
            ManagedSecretValue::OpenaiApiKey { .. } => ManagedSecretType::OpenaiApiKey,
            ManagedSecretValue::DockerRegistry { .. } => ManagedSecretType::DockerRegistry,
        }
    }
}

impl fmt::Debug for ManagedSecretValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ManagedSecretValue::RawValue { .. } => f
                .debug_struct("ManagedSecret::RawValue")
                .finish_non_exhaustive(),
            ManagedSecretValue::AnthropicApiKey { .. } => f
                .debug_struct("ManagedSecret::AnthropicApiKey")
                .finish_non_exhaustive(),
            ManagedSecretValue::OpenaiApiKey { .. } => f
                .debug_struct("ManagedSecret::OpenaiApiKey")
                .finish_non_exhaustive(),
            ManagedSecretValue::DockerRegistry { .. } => f
                .debug_struct("ManagedSecret::DockerRegistry")
                .finish_non_exhaustive(),
        }
    }
}

#[cfg(test)]
#[path = "secret_value_tests.rs"]
mod tests;
