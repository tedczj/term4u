use std::collections::HashMap;
use std::ops::Range;

use serde::{Deserialize, Serialize};
use warp_editor::render::model::LineCount;

use crate::code_review::diff_state::{DiffLineType, FileDiff};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CurrentHead {
    BranchName(String),
    HeadlessCommitSha(String),
}

impl CurrentHead {
    pub fn title(&self) -> String {
        match self {
            Self::BranchName(name) => name.clone(),
            Self::HeadlessCommitSha(sha) => format!("Commit {}", sha.chars().take(7).collect::<String>()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffBase {
    BranchName(String),
    HeadlessCommitSha(String),
    UncommittedChanges,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffSetHunk {
    pub line_range: Range<LineCount>,
    pub diff_content: String,
    pub lines_added: u32,
    pub lines_removed: u32,
}

pub fn convert_file_diffs_to_diffset_hunks<'a>(
    files: impl Iterator<Item = &'a FileDiff>,
) -> HashMap<String, Vec<DiffSetHunk>> {
    files
        .filter_map(|file| {
            let hunks = file
                .hunks
                .iter()
                .map(|hunk| {
                    let mut lines_added = 0;
                    let mut lines_removed = 0;
                    let diff_content = hunk
                        .lines
                        .iter()
                        .filter_map(|line| {
                            let prefix = match line.line_type {
                                DiffLineType::Add => {
                                    lines_added += 1;
                                    "+"
                                }
                                DiffLineType::Delete => {
                                    lines_removed += 1;
                                    "-"
                                }
                                DiffLineType::Context => "",
                                DiffLineType::HunkHeader => return None,
                            };
                            Some(format!("{prefix}{}", line.text))
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    DiffSetHunk {
                        line_range: LineCount::from(hunk.new_start_line.saturating_sub(1))
                            ..LineCount::from(
                                hunk.new_start_line.saturating_sub(1) + hunk.new_line_count,
                            ),
                        diff_content,
                        lines_added,
                        lines_removed,
                    }
                })
                .collect::<Vec<_>>();
            (!hunks.is_empty()).then(|| (file.file_path.clone(), hunks))
        })
        .collect()
}
