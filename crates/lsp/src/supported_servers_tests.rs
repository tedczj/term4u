#[cfg(unix)]
mod unix {
    use std::fs;
    use std::os::unix::fs::PermissionsExt as _;

    use super::super::LSPServerType;
    use crate::CommandBuilder;

    fn fake_binary(directory: &std::path::Path, name: &str) {
        let path = directory.join(name);
        fs::write(&path, "#!/bin/sh\nexit 0\n").unwrap();
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).unwrap();
    }

    #[tokio::test]
    async fn detects_rust_analyzer_from_path() {
        let directory = tempfile::tempdir().unwrap();
        fake_binary(directory.path(), "rust-analyzer");
        let executor = CommandBuilder::new(Some(directory.path().to_string_lossy().into_owned()));

        assert!(
            LSPServerType::RustAnalyzer
                .candidate()
                .is_installed(&executor)
                .await
        );
    }

    #[tokio::test]
    async fn missing_server_reports_manual_install_guidance() {
        let message = LSPServerType::Clangd.manual_install_message();

        assert_eq!(
            message,
            "clangd is not installed. Install it manually and make sure it is available on PATH."
        );
    }
}
