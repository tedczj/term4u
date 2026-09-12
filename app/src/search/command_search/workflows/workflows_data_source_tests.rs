use warpui::App;

use super::*;
use crate::search::command_search::searcher::AcceptedWorkflow;

#[test]
fn project_search_reloads_workflows_and_preserves_arguments() {
    App::test((), |mut app| async move {
        crate::test_util::settings::initialize_settings_for_tests(&mut app);
        app.add_singleton_model(LocalWorkflows::new);
        let directory = tempfile::tempdir().unwrap();
        git2::Repository::init(directory.path()).unwrap();
        let workflows = directory.path().join(".warp/workflows");
        std::fs::create_dir_all(&workflows).unwrap();
        let path = workflows.join("project.yaml");
        std::fs::write(&path, "name: R1R2 local workflow\ncommand: echo {{message}}\narguments:\n  - name: message\n    default_value: hello\n").unwrap();
        for command in ["echo {{message}}", "printf {{message}}"] {
            if command.starts_with("printf") {
                std::fs::write(&path, "name: R1R2 local workflow\ncommand: printf {{message}}\narguments:\n  - name: message\n    default_value: hello\n").unwrap();
            }
            app.update(|ctx| {
                let source = WorkflowsDataSource::new(None, Some(directory.path()), ctx);
                let results = source
                    .run_query(&Query::from("R1R2 local workflow"), ctx)
                    .unwrap();
                let workflow = results
                    .iter()
                    .find_map(|result| match result.accept_result() {
                        CommandSearchItemAction::AcceptWorkflow(AcceptedWorkflow::Local {
                            workflow,
                            source: WorkflowSource::Project,
                        }) => Some(workflow),
                        _ => None,
                    })
                    .expect("project workflow must be searchable without a shell session");
                assert_eq!(workflow.as_workflow().command, command);
                assert_eq!(
                    workflow.as_workflow().arguments[0].default_value.as_deref(),
                    Some("hello")
                );
            });
        }
    });
}
