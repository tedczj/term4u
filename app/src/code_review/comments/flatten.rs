use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use super::comment::{
    AttachedReviewComment, AttachedReviewCommentTarget, CommentId, CommentOrigin,
};
use super::pending_imported::{PendingImportedReviewComment, PendingImportedReviewCommentTarget};
use crate::code::buffer_location::LocalOrRemotePath;

const THREAD_REPLY_DIVIDER: &str = "\n---\n";

pub(super) trait ReviewCommentThreadItem {
    fn comment_id(&self) -> &str;
    fn parent_comment_id(&self) -> Option<&str>;
    fn author(&self) -> &str;
    fn body(&self) -> &str;
    fn compare_last_modified(&self, other: &Self) -> Ordering;
}

struct ReviewCommentThread<'a, T> {
    comments: Vec<&'a T>,
    missing_parent_id: Option<&'a str>,
}

impl<'a, T> ReviewCommentThread<'a, T> {
    fn root(&self) -> &'a T { self.comments[0] }
    fn comments(&self) -> &[&'a T] { &self.comments }
    fn missing_parent_id(&self) -> Option<&'a str> { self.missing_parent_id }
}

fn group_review_comment_threads<T: ReviewCommentThreadItem>(comments: &[T]) -> Vec<ReviewCommentThread<'_, T>> {
    let existing_ids: HashSet<&str> = comments.iter().map(T::comment_id).collect();
    let mut roots = HashMap::new();
    let mut children: HashMap<&str, Vec<&T>> = HashMap::new();
    for comment in comments {
        match comment.parent_comment_id() {
            Some(parent) if existing_ids.contains(parent) => children.entry(parent).or_default().push(comment),
            missing => { roots.insert(comment.comment_id(), (comment, missing)); }
        }
    }
    let mut roots: Vec<_> = roots.into_values().collect();
    roots.sort_by(|(a, _), (b, _)| a.comment_id().cmp(b.comment_id()));
    roots.into_iter().map(|(root, missing_parent_id)| {
        let mut comments = Vec::new();
        collect_thread(root, &children, &mut comments);
        ReviewCommentThread { comments, missing_parent_id }
    }).collect()
}

fn collect_thread<'a, T: ReviewCommentThreadItem>(
    comment: &'a T,
    children_map: &HashMap<&str, Vec<&'a T>>,
    result: &mut Vec<&'a T>,
) {
    result.push(comment);
    if let Some(children) = children_map.get(comment.comment_id()) {
        let mut children = children.clone();
        children.sort_by(|a, b| a.compare_last_modified(b));
        for child in children {
            collect_thread(child, children_map, result);
        }
    }
}

fn format_review_comment_thread<T: ReviewCommentThreadItem>(thread: &ReviewCommentThread<'_, T>) -> String {
    thread.comments().iter().map(|comment| format!("**@{}**:\n{}", comment.author(), comment.body())).collect::<Vec<_>>().join(THREAD_REPLY_DIVIDER)
}

/// Converts pending imported provider comments into attached review comments by:
/// * flattening threaded replies
/// * formatting markdown bodies
/// * converting repo-relative file paths to absolute file paths
pub(crate) fn attach_pending_imported_comments(
    pending_comments: Vec<PendingImportedReviewComment>,
    repo_path: &LocalOrRemotePath,
) -> Vec<AttachedReviewComment> {
    if pending_comments.is_empty() {
        return Vec::new();
    }

    group_review_comment_threads(&pending_comments)
        .into_iter()
        .map(|thread| {
            if let Some(missing_parent_id) = thread.missing_parent_id() {
                log::warn!(
                    "Importing orphaned comment (ID {:?}) with parent ID {:?}",
                    thread.root().github_comment_id(),
                    missing_parent_id
                );
            }
            attach_pending_imported_thread(thread, repo_path)
        })
        .collect()
}

fn attach_pending_imported_thread(
    thread: ReviewCommentThread<'_, PendingImportedReviewComment>,
    repo_path: &LocalOrRemotePath,
) -> AttachedReviewComment {
    let root = thread.root();
    let last_update_time = thread
        .comments()
        .iter()
        .map(|comment| comment.last_update_time)
        .max()
        .unwrap_or(root.last_update_time);

    let target = match &root.target {
        PendingImportedReviewCommentTarget::Line {
            relative_file_path,
            line,
            diff_content,
        } => AttachedReviewCommentTarget::Line {
            absolute_file_path: repo_path.join(&relative_file_path.to_string_lossy()),
            line: line.clone(),
            content: diff_content.clone(),
        },
        PendingImportedReviewCommentTarget::File { relative_file_path } => {
            AttachedReviewCommentTarget::File {
                absolute_file_path: repo_path.join(&relative_file_path.to_string_lossy()),
            }
        }
        PendingImportedReviewCommentTarget::General => AttachedReviewCommentTarget::General,
    };

    let origin = CommentOrigin::ImportedFromGitHub(root.github_details_without_parent());

    AttachedReviewComment {
        id: CommentId::new(),
        content: format_review_comment_thread(&thread),
        target,
        last_update_time,
        base: None,
        head: None,
        outdated: false,
        origin,
    }
}
