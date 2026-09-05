pub mod manager;
pub mod model;
pub mod notebook;

pub use model::{Notebook, NotebookId, NotebookLocation};
pub use crate::util::openable_file_type::renders_in_warp_notebook_viewer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkdownDisplayMode {
    Rendered,
    Raw,
}
