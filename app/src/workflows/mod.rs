use std::sync::Arc;

use serde::{Deserialize, Serialize};
use warpui::AppContext;

pub mod categories;
pub mod command_parser;
pub mod local_workflows;
pub mod manager;
pub mod model;
pub mod workflow;
pub mod workflow_view;

pub use categories::{CategoriesView, CategoriesViewEvent, WorkflowsViewAction};
pub use model::{Argument, ArgumentType, Workflow, WorkflowId};

use crate::notebooks::NotebookLocation;

pub fn init(app: &mut AppContext) {
    categories::init(app);
    workflow_view::init(app);
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize, Eq, PartialEq, Hash)]
pub enum WorkflowSource {
    Global,
    Local,
    Project,
    Notebook { location: NotebookLocation },
    App,
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize, Eq, PartialEq, Hash, PartialOrd)]
pub enum WorkflowSelectionSource {
    CommandPalette,
    UniversalSearch,
    Voltron,
    Notebook,
    SlashMenu,
    UpArrowHistory,
    WorkflowView,
    Undefined,
    Alias,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowViewMode {
    View,
    Edit,
    Create,
}

impl WorkflowViewMode {
    pub fn supported_edit_mode() -> Self {
        Self::Edit
    }

    pub fn supported_view_mode() -> Self {
        Self::Edit
    }

    pub fn is_editable(&self) -> bool {
        match self {
            Self::View => false,
            Self::Edit | Self::Create => true,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum WorkflowType {
    Local(Workflow),
    Notebook(Workflow),
}

impl WorkflowType {
    pub fn as_workflow(&self) -> &Workflow {
        match self {
            Self::Local(workflow) | Self::Notebook(workflow) => workflow,
        }
    }

    pub fn take_workflow(self) -> Workflow {
        match self {
            Self::Local(workflow) | Self::Notebook(workflow) => workflow,
        }
    }

    pub(super) fn should_show_env_var_selection(&self) -> bool {
        true
    }
}

impl From<Workflow> for WorkflowType {
    fn from(workflow: Workflow) -> Self {
        Self::Local(workflow)
    }
}

pub type SharedWorkflow = Arc<WorkflowType>;
