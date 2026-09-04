use crate::error::UserFacingError;
use crate::request_context::RequestContext;
use crate::schema;

/// A GraphQL query to fetch secrets for a specific task.
///
/// This query is used by Agent Mode tasks to retrieve secrets that have been made available
/// to them via a short-lived workload token.
#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "RootQuery", variables = "TaskSecretsVariables")]
pub struct TaskSecrets {
    #[arguments(input: $input, requestContext: $request_context)]
    pub task_secrets: TaskSecretsResult,
}

crate::client::define_operation! {
    task_secrets(TaskSecretsVariables) -> TaskSecrets;
}

#[derive(cynic::QueryVariables, Debug)]
pub struct TaskSecretsVariables {
    pub input: TaskSecretsInput,
    pub request_context: RequestContext,
}

#[derive(cynic::InputObject, Debug)]
pub struct TaskSecretsInput {
    pub task_id: cynic::Id,
    pub workload_token: String,
}

#[derive(cynic::InlineFragments, Debug)]
pub enum TaskSecretsResult {
    TaskSecretsOutput(TaskSecretsOutput),
    UserFacingError(UserFacingError),
    #[cynic(fallback)]
    Unknown,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct TaskSecretsOutput {
    pub secrets: Vec<TaskSecretEntry>,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct TaskSecretEntry {
    pub name: String,
    pub value: ManagedSecretValue,
}

#[derive(cynic::InlineFragments, Debug)]
pub enum ManagedSecretValue {
    ManagedSecretRawValue(ManagedSecretRawValue),
    ManagedSecretAnthropicApiKeyValue(ManagedSecretAnthropicApiKeyValue),
    ManagedSecretOpenAiApiKeyValue(ManagedSecretOpenAiApiKeyValue),
    #[cynic(fallback)]
    Unknown,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct ManagedSecretRawValue {
    pub value: String,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct ManagedSecretAnthropicApiKeyValue {
    pub api_key: String,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct ManagedSecretOpenAiApiKeyValue {
    pub api_key: String,
    /// Optional base URL for regional endpoints.
    pub base_url: Option<String>,
}
