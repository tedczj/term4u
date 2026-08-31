pub mod templatable_manager;

#[cfg(not(target_family = "wasm"))]
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use strum::IntoEnumIterator;
use strum_macros::EnumIter;
#[cfg(not(target_family = "wasm"))]
pub use templatable_manager::McpIntegration;
pub use templatable_manager::TemplatableMCPServerManager;
use warp_core::ui::Icon;
use warp_core::ui::appearance::Appearance;

use crate::cloud_object::model::generic_string_model::StringModel;
use crate::cloud_object::model::json_model::JsonModel;
use crate::cloud_object::{
    CloudObjectUuid, GenericStringObjectFormat, GenericStringObjectUniqueKey, JsonObjectType,
    Revision,
};
use crate::drive::CloudObjectTypeAndId;
use crate::drive::items::WarpDriveItem;
use crate::drive::items::mcp_server::WarpDriveMCPServer;
use crate::server::ids::SyncId;
use crate::server::sync_queue::QueueItem;

cfg_if::cfg_if! {
    if #[cfg(not(feature = "local_fs"))] {
        mod dummy_file_based_manager;
        pub use dummy_file_based_manager::FileBasedMCPManager;
        mod dummy_file_mcp_watcher;
        pub use dummy_file_mcp_watcher::FileMCPWatcher;
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "local_fs")] {
        pub mod file_based_manager;
        pub use file_based_manager::FileBasedMCPManager;
        pub mod file_mcp_watcher;
        pub use file_mcp_watcher::{FileMCPWatcher, FileMCPWatcherEvent};
    }
}

pub mod gallery;
pub use gallery::MCPGalleryManager;
#[cfg(not(target_family = "wasm"))]
pub mod builtin;
pub mod templatable;
#[cfg(not(target_family = "wasm"))]
pub use cloud_object_models::{
    CLIServer, JSONMCPServer, JSONTransportType, ServerSentEvents, StaticEnvVar, StaticHeader,
};
pub use cloud_object_models::{
    CloudMCPServer, CloudMCPServerModel, MCPServer, MCPServerState, TransportType,
};
pub use templatable::{JsonTemplate, TemplatableMCPServer, TemplateVariable};
pub mod logs;
pub mod templatable_installation;
pub use templatable_installation::TemplatableMCPServerInstallation;
#[cfg(not(target_family = "wasm"))]
pub use templatable_installation::{VariableType, VariableValue};
pub mod parsing;
#[cfg(not(target_family = "wasm"))]
pub use parsing::ParsedTemplatableMCPServerResult;
#[cfg(not(target_family = "wasm"))]
pub mod reconnecting_peer;

impl CloudObjectUuid for MCPServer {
    fn uuid(&self) -> uuid::Uuid {
        self.uuid
    }
}

impl StringModel for MCPServer {
    type CloudObjectType = CloudMCPServer;

    fn model_type_name(&self) -> &'static str {
        "MCP server"
    }

    fn should_enforce_revisions() -> bool {
        true
    }

    fn model_format() -> GenericStringObjectFormat {
        GenericStringObjectFormat::Json(JsonObjectType::MCPServer)
    }

    fn should_show_activity_toasts() -> bool {
        true
    }

    fn warn_if_unsaved_at_quit() -> bool {
        true
    }

    fn display_name(&self) -> String {
        self.name.clone()
    }

    fn update_object_queue_item(
        &self,
        revision_ts: Option<Revision>,
        object: &Self::CloudObjectType,
    ) -> QueueItem {
        QueueItem::UpdateMCPServer {
            model: object.model().clone().into(),
            id: object.id,
            revision: revision_ts.or(object.metadata.revision),
        }
    }

    fn uniqueness_key(&self) -> Option<GenericStringObjectUniqueKey> {
        None
    }

    fn renders_in_warp_drive(&self) -> bool {
        false
    }

    fn to_warp_drive_item(
        &self,
        id: SyncId,
        _appearance: &Appearance,
        mcp_server: &CloudMCPServer,
    ) -> Option<Box<dyn WarpDriveItem>> {
        Some(Box::new(WarpDriveMCPServer::new(
            CloudObjectTypeAndId::GenericStringObject {
                object_type: GenericStringObjectFormat::Json(JsonObjectType::MCPServer),
                id,
            },
            mcp_server.clone(),
        )))
    }
}

impl JsonModel for MCPServer {
    fn json_object_type() -> JsonObjectType {
        JsonObjectType::MCPServer
    }
}

/// Trait for types that have a name and value field.
/// Used for shared operations on `StaticEnvVar` and `StaticHeader`.
#[cfg(not(target_family = "wasm"))]
trait NameValuePair {
    fn name(&self) -> &str;
    fn value(&self) -> &str;
    fn new(name: String, value: String) -> Self;
}

#[cfg(not(target_family = "wasm"))]
impl NameValuePair for StaticEnvVar {
    fn name(&self) -> &str {
        &self.name
    }
    fn value(&self) -> &str {
        &self.value
    }
    fn new(name: String, value: String) -> Self {
        Self { name, value }
    }
}
#[cfg(not(target_family = "wasm"))]
impl NameValuePair for StaticHeader {
    fn name(&self) -> &str {
        &self.name
    }
    fn value(&self) -> &str {
        &self.value
    }
    fn new(name: String, value: String) -> Self {
        Self { name, value }
    }
}

/// Converts a HashMap to a Vec of name/value pair items.
#[cfg(not(target_family = "wasm"))]
fn items_from_hashmap<T: NameValuePair>(map: &HashMap<String, String>) -> Vec<T> {
    map.iter()
        .map(|(name, value)| T::new(name.to_owned(), value.to_owned()))
        .collect()
}

/// Converts a slice of name/value pair items to a HashMap.
#[cfg(not(target_family = "wasm"))]
#[allow(dead_code)]
fn items_to_hashmap<T: NameValuePair>(items: &[T]) -> HashMap<String, String> {
    items
        .iter()
        .map(|item| (item.name().to_owned(), item.value().to_owned()))
        .collect()
}

/// Processes name/value pair items for template conversion.
/// Returns a tuple of:
/// - HashMap with template placeholders (e.g., `{{name}}`)
/// - Vec of TemplateVariables
/// - HashMap of VariableValues
#[cfg(not(target_family = "wasm"))]
/// Applies values from a persisted HashMap to a collection of name/value pairs.
#[cfg(not(target_family = "wasm"))]
#[cfg(not(target_family = "wasm"))]
fn find_server_map(
    config: serde_json::Value,
) -> serde_json::Result<HashMap<String, JSONMCPServer>> {
    // We want to be quite permissive in parsing user input. They may specify more than one
    // server. They might paste things in Claude Desktop style or VSCode style. All are
    // accepted here.
    //
    // VSCode:
    // {
    //   "mcp": {
    //     "servers": {
    //          [map of mcp servers]
    //     }
    //   }
    // }
    //   ---  OR  ---
    // {
    //   "servers": {
    //     [map of mcp servers]
    //   }
    // }
    //
    // Claude Desktop:
    // {
    //   "mcpServers": {
    //     [map of mcp servers]
    //   }
    // }
    // Also allowed:
    // {
    //   [map of mcp servers]
    // }

    let pointers = ["/mcp/servers", "/servers", "/mcpServers"];
    for pointer in pointers.into_iter() {
        if let Some(value) = config.pointer(pointer)
            && let Ok(servers) =
                serde_json::from_value::<HashMap<String, JSONMCPServer>>(value.clone())
        {
            return Ok(servers);
        }
    }
    serde_json::from_value::<HashMap<String, JSONMCPServer>>(config)
}

#[cfg(not(target_family = "wasm"))]
pub trait MCPServerExt {
    fn from_user_json(json: &str) -> serde_json::Result<Vec<MCPServer>>;
    #[cfg(test)]
    fn to_user_json(&self) -> String;
}

#[cfg(not(target_family = "wasm"))]
impl MCPServerExt for MCPServer {
    fn from_user_json(json: &str) -> serde_json::Result<Vec<MCPServer>> {
        // Some docs don't show curly braces around the json object, so add them if necessary.
        let json = json.trim();
        let json = if json.starts_with("{") {
            json.to_owned()
        } else {
            format!("{{{json}}}")
        };

        let config: serde_json::Value = serde_json::from_str(&json)?;

        let servers = find_server_map(config)?;
        Ok(servers
            .iter()
            .map(|(name, server)| {
                let transport_type = match &server.transport_type {
                    JSONTransportType::CLIServer {
                        command,
                        args,
                        env,
                        working_directory,
                    } => TransportType::CLIServer(CLIServer {
                        command: command.clone(),
                        args: args.clone(),
                        cwd_parameter: working_directory.to_owned(),
                        static_env_vars: items_from_hashmap(env),
                    }),
                    JSONTransportType::SSEServer { url, headers } => {
                        TransportType::ServerSentEvents(ServerSentEvents {
                            url: url.to_owned(),
                            headers: items_from_hashmap(headers),
                        })
                    }
                };
                MCPServer {
                    name: name.to_owned(),
                    transport_type,
                    uuid: uuid::Uuid::new_v4(),
                }
            })
            .collect())
    }

    /// Includes the environment variable values, should only be shown to users,
    /// not sent to our servers.
    #[cfg(test)]
    fn to_user_json(&self) -> String {
        let transport_type = match &self.transport_type {
            TransportType::CLIServer(cli_server) => JSONTransportType::CLIServer {
                command: cli_server.command.clone(),
                args: cli_server.args.clone(),
                env: items_to_hashmap(&cli_server.static_env_vars),
                working_directory: cli_server.cwd_parameter.to_owned(),
            },
            TransportType::ServerSentEvents(sse_server) => JSONTransportType::SSEServer {
                url: sse_server.url.to_owned(),
                headers: items_to_hashmap(&sse_server.headers),
            },
        };
        serde_json::to_string_pretty(
            &std::iter::once((self.name.to_owned(), JSONMCPServer { transport_type }))
                .collect::<HashMap<_, _>>(),
        )
        .unwrap_or_default()
    }
}

#[derive(Debug, Clone)]
pub enum Author {
    CurrentUser,
    OtherUser { name: String },
    Unknown,
}

#[derive(Debug, Clone)]
pub enum MCPServerUpdate {
    CloudTemplate {
        publisher: Author,
        new_version_ts: i64,
        json_template: JsonTemplate,
    },
    Gallery {
        name: String,
        new_version: i32,
        json_template: JsonTemplate,
    },
}

pub(crate) fn home_config_file_path(provider: MCPProvider) -> Option<PathBuf> {
    match provider {
        MCPProvider::Warp => warp_core::paths::warp_home_mcp_config_file_path(),
        _ => dirs::home_dir().map(|home_dir| home_dir.join(provider.home_config_path())),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter)]
pub enum MCPProvider {
    Warp,
    Claude,
    Codex,
    Agents,
}

impl MCPProvider {
    pub fn display_name(&self) -> &str {
        match self {
            MCPProvider::Warp => "Warp",
            MCPProvider::Claude => "Claude",
            MCPProvider::Codex => "Codex",
            MCPProvider::Agents => "Other Agents",
        }
    }

    pub fn icon(&self) -> Icon {
        match self {
            // Warp's own agent MCP config — use the Warp agent brand mark.
            MCPProvider::Warp => Icon::Agent,
            MCPProvider::Claude => Icon::ClaudeLogo,
            MCPProvider::Codex => Icon::OpenAILogo,
            // "Other Agents" is the cross-tool .agents/.mcp.json convention for
            // third-party agent tooling (not Warp-branded). Use a neutral AI
            // icon so this row never carries the Warp agent mark, and the two
            // rows remain visually distinct once Icon::Agent gets its own asset.
            MCPProvider::Agents => Icon::AiAssistant,
        }
    }

    /// Returns the path of the provider's config file relative to the home directory.
    pub fn home_config_path(&self) -> &'static Path {
        match self {
            MCPProvider::Warp => Path::new(".warp/.mcp.json"),
            MCPProvider::Claude => Path::new(".claude.json"),
            MCPProvider::Codex => Path::new(".codex/config.toml"),
            MCPProvider::Agents => Path::new(".agents/.mcp.json"),
        }
    }

    /// Returns the path of the provider's config file relative to a project root.
    pub fn project_config_path(&self) -> &'static Path {
        match self {
            MCPProvider::Warp => Path::new(".warp/.mcp.json"),
            MCPProvider::Claude => Path::new(".mcp.json"),
            MCPProvider::Codex => Path::new(".codex/config.toml"),
            MCPProvider::Agents => Path::new(".agents/.mcp.json"),
        }
    }
}

/// Returns the [`MCPProvider`] that owns `file_path` as a config file, if any.
///
/// Matches against both home-level configs (e.g. `~/.claude.json`) and
/// project-level configs (e.g. `.mcp.json` anywhere in the path).
pub fn mcp_provider_from_file_path(file_path: &Path) -> Option<MCPProvider> {
    // Try exact home-config match first (unambiguous).
    for provider in MCPProvider::iter() {
        if home_config_file_path(provider)
            .as_ref()
            .is_some_and(|home_config_path| file_path == home_config_path)
        {
            return Some(provider);
        }
    }
    // Fall back to project-config suffix match, preferring the longest
    // (most-specific) suffix.
    // This avoids `.mcp.json` shadowing `.warp/.mcp.json`, for example.
    let mut best: Option<(MCPProvider, usize)> = None;
    for provider in MCPProvider::iter() {
        let cfg = provider.project_config_path();
        if file_path.ends_with(cfg) {
            let len = cfg.as_os_str().len();
            if best.is_none_or(|(_, best_len)| len > best_len) {
                best = Some((provider, len));
            }
        }
    }
    best.map(|(p, _)| p)
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
