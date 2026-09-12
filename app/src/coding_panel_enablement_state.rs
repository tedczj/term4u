#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CodingPanelEnablementState {
    Enabled,
    /// The active session is on a remote host.
    ///
    /// `has_remote_server` is `true` when the session is registered with
    /// `RemoteServerManager` (i.e. Auto SSH Warpification / mode 1). When
    /// `true`, remote repo metadata may arrive and the file tree should show
    /// a loading state. When `false` (tmux or subshell SSH), no data will
    /// arrive and the file tree should show a disabled message.
    RemoteSession {
        has_remote_server: bool,
    },
    UnsupportedSession,
}
