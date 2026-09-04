CREATE TABLE mcp_server_panes (
  id INTEGER PRIMARY KEY NOT NULL,
  kind TEXT NOT NULL DEFAULT 'mcp_server' CHECK (kind = 'mcp_server'),

  FOREIGN KEY (id, kind) REFERENCES pane_leaves (pane_node_id, kind)
);

CREATE TABLE mcp_environment_variables (
    mcp_server_uuid BLOB PRIMARY KEY NOT NULL,
    environment_variables TEXT NOT NULL
);

CREATE TABLE active_mcp_servers (
    id INTEGER PRIMARY KEY NOT NULL,
    mcp_server_uuid TEXT NOT NULL,
    UNIQUE(mcp_server_uuid)
);

CREATE TABLE mcp_server_installations (
    id TEXT NOT NULL PRIMARY KEY,
    templatable_mcp_server TEXT NOT NULL,
    template_version_ts TIMESTAMP NOT NULL,
    variable_values TEXT NOT NULL,
    restore_running BOOLEAN NOT NULL,
    last_modified_at TIMESTAMP NOT NULL
);
