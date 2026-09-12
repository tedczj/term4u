pub mod manager;
pub mod model;
pub mod notebook;

pub use model::{NotebookId, NotebookLocation};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkdownDisplayMode {
    Rendered,
    Raw,
}
