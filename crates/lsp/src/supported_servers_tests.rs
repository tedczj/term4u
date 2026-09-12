#[cfg(unix)]
mod unix {
    use std::fs;
    use std::os::unix::fs::PermissionsExt as _;

    use strum::IntoEnumIterator;

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
    async fn every_server_uses_only_the_supplied_path_and_rejects_broken_binaries() {
        let directory = tempfile::tempdir().unwrap();
        let executor = CommandBuilder::new(Some(directory.path().to_string_lossy().into_owned()));
        for server in LSPServerType::iter() {
            assert!(!server.is_working_on_path(&executor).await, "{server:?}");
            fake_binary(directory.path(), server.binary_name());
            assert!(server.is_working_on_path(&executor).await, "{server:?}");
            let path = directory.path().join(server.binary_name());
            fs::write(&path, "#!/bin/sh\nexit 23\n").unwrap();
            assert!(!server.is_working_on_path(&executor).await, "{server:?}");
            fs::remove_file(path).unwrap();
            assert!(
                server
                    .manual_install_message()
                    .contains(server.binary_name())
            );
            assert!(
                server
                    .manual_install_message()
                    .contains("Install it manually")
            );
        }
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
