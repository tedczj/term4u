//! Local output links. Parsing stays in the terminal model; filesystem checks run off the UI thread.

use warpui::event::ModifiersState;
use warpui::platform::Cursor;
use warpui::ui_components::components::UiComponent;
use warpui::{AppContext, Element, SingletonEntity, ViewContext};
#[cfg(feature = "local_fs")]
use {
    crate::{
        terminal::{ShellLaunchData, model::grid::grid_handler},
        util::file::{FileLink, ShellPathType, absolute_path_if_valid},
    },
    std::path::PathBuf,
    unicode_general_category::{GeneralCategory, get_general_category},
    unicode_width::UnicodeWidthChar,
    warp_util::path::{CleanPathResult, LineAndColumnArg},
};

use super::TerminalView;
use crate::GeneralSettings;
use crate::terminal::links::should_directly_open_link;
use crate::terminal::model::RespectObfuscatedSecrets;
use crate::terminal::model::grid::grid_handler::Link;
use crate::terminal::model::index::Point;
use crate::terminal::model::terminal_model::{WithinBlock, WithinModel};
#[cfg(feature = "local_fs")]
use crate::util::file::external_editor::EditorSettings;
#[cfg(feature = "local_fs")]
use crate::util::openable_file_type::FileTarget;

// "a/" and "b/" are prefixes specific to Git Diff
#[cfg(feature = "local_fs")]
const PREFIXES_TO_REMOVE: [&str; 2] = ["a/", "b/"];

/// "@" is a suffix that can be added to symlinks. It appears in Git Bash's default configuration
/// for `ls`.
#[cfg(feature = "local_fs")]
const SUFFIXES_TO_REMOVE: [&str; 1] = ["@"];

#[cfg(feature = "local_fs")]
struct TrimmedSentencePunctuation<'a> {
    path: &'a str,
    removed_width: usize,
}

#[cfg(feature = "local_fs")]
fn is_trailing_sentence_punctuation(c: char) -> bool {
    if c == '.' {
        return true;
    }
    if c.is_ascii() {
        return false;
    }
    matches!(
        get_general_category(c),
        GeneralCategory::ClosePunctuation
            | GeneralCategory::FinalPunctuation
            | GeneralCategory::OtherPunctuation
    )
}

/// Strips trailing sentence punctuation from a captured path token when the
/// punctuation is prose around the path rather than a meaningful path component.
///
/// File paths written at the end of a sentence frequently capture the trailing
/// punctuation (e.g. `notes/README.md.` or `notes/README.md，`). On Windows the
/// NT path normalizer silently strips a trailing `.` during path resolution, so
/// without trimming, the captured token keeps the period in both the highlight
/// range and the file extension, defeating extension-based classification (e.g.
/// opening markdown in the viewer instead of as raw text).
///
/// Returns `None` when there is no trailing sentence punctuation, or when a
/// trailing period is part of a `.`/`..` path component (e.g. `.`, `..`, `foo/.`,
/// `foo/..`), which are legitimate path segments and must be preserved.
#[cfg(feature = "local_fs")]
fn path_without_trailing_sentence_punctuation(
    path: &str,
) -> Option<TrimmedSentencePunctuation<'_>> {
    let mut trimmed = path;
    let mut removed_width = 0;

    while let Some(c) = trimmed.chars().next_back() {
        if !is_trailing_sentence_punctuation(c) {
            break;
        }

        let new_trimmed = trimmed.strip_suffix(c)?;
        if new_trimmed.is_empty() {
            break;
        }

        if c == '.' {
            match new_trimmed.chars().next_back() {
                // Empty (`.`) or a dot/separator immediately before the trailing
                // `.` means the period is a real path component (`..`, `foo/.`,
                // `foo\.`), not sentence punctuation.
                None | Some('.') | Some('/') | Some('\\') => break,
                _ => {}
            }
        }

        trimmed = new_trimmed;
        removed_width += UnicodeWidthChar::width(c).unwrap_or(1);
    }

    (removed_width > 0).then_some(TrimmedSentencePunctuation {
        path: trimmed,
        removed_width,
    })
}

#[derive(Clone, Debug)]
pub(super) enum GridHighlightedLink {
    Url {
        link: WithinModel<Link>,
        uri: String,
    },
    #[cfg(feature = "local_fs")]
    File(WithinModel<FileLink>),
}

impl GridHighlightedLink {
    pub(super) fn range(&self) -> WithinModel<Link> {
        match self {
            Self::Url { link, .. } => link.clone(),
            #[cfg(feature = "local_fs")]
            Self::File(link) => link.clone().replace_inner(link.get_inner().link.clone()),
        }
    }

    pub(super) fn label(&self) -> String {
        match self {
            Self::Url { uri, .. } => uri.clone(),
            #[cfg(feature = "local_fs")]
            Self::File(file) => file.get_inner().absolute_path.display().to_string(),
        }
    }
}

#[derive(PartialEq, Eq)]
struct LinkContext {
    block_id: crate::terminal::model::BlockId,
    session_id: Option<crate::terminal::model::session::SessionId>,
    active_session_id: Option<crate::terminal::model::session::SessionId>,
    pwd: Option<String>,
    alt_screen: bool,
}

#[derive(Default)]
pub(super) struct LinkState {
    generation: u64,
    position: Option<WithinModel<Point>>,
    pub(super) highlighted: Option<GridHighlightedLink>,
    source_text: String,
    pending_click: Option<ModifiersState>,
    context: Option<LinkContext>,
    scan: Option<warpui::r#async::SpawnedFutureHandle>,
    /// A drag must never turn into a link activation when the mouse button is released.
    pub(super) dragged: bool,
}

fn allowed_url(uri: &str) -> bool {
    url::Url::parse(uri).is_ok_and(|url| matches!(url.scheme(), "http" | "https"))
}

impl TerminalView {
    fn link_context(&self, position: WithinModel<Point>, ctx: &AppContext) -> Option<LinkContext> {
        let model = self.model.lock();
        let block = match position {
            WithinModel::BlockList(point) => model.block_list().block_at(point.block_index)?,
            WithinModel::AltScreen(_) => model.block_list().active_block(),
        };
        Some(LinkContext {
            block_id: block.id().clone(),
            session_id: block.session_id(),
            active_session_id: self.model_events.as_ref(ctx).active_session_id(),
            pwd: block.pwd().cloned(),
            alt_screen: model.is_alt_screen_active(),
        })
    }

    pub(super) fn clear_link(&mut self, ctx: &mut ViewContext<Self>) {
        self.links.generation = self.links.generation.wrapping_add(1);
        self.links.position = None;
        self.links.pending_click = None;
        if let Some(scan) = self.links.scan.take() {
            scan.abort();
        }
        if self.links.highlighted.take().is_some() {
            ctx.reset_cursor();
            ctx.notify();
        }
    }

    pub(super) fn highlighted_grid_link(&self) -> Option<WithinModel<Link>> {
        self.links
            .highlighted
            .as_ref()
            .map(GridHighlightedLink::range)
    }

    pub(super) fn hover_link(
        &mut self,
        position: Option<WithinModel<Point>>,
        click: Option<ModifiersState>,
        ctx: &mut ViewContext<Self>,
    ) {
        if self.links.dragged || (self.is_selecting && click.is_none()) {
            self.clear_link(ctx);
            return;
        }
        if self.links.position == position {
            if self.links.highlighted.is_none() {
                if let Some(click) = click {
                    self.links.pending_click = Some(click);
                }
            } else if click.is_some_and(|modifiers| should_directly_open_link(&modifiers)) {
                self.open_grid_link(self.links.generation, ctx);
            }
            return;
        }
        self.clear_link(ctx);
        let Some(position) = position else { return };
        self.links.position = Some(position);
        self.links.context = self.link_context(position, ctx);
        let generation = self.links.generation;
        let detected = {
            let model = self.model.lock();
            model.hyperlink_at_point(&position).or_else(|| {
                model.url_at_point(&position).map(|link| {
                    let uri = model.link_at_range(&link, RespectObfuscatedSecrets::No);
                    (link, uri)
                })
            })
        };
        if let Some((link, uri)) = detected {
            if allowed_url(&uri) {
                self.finish_link(
                    generation,
                    Some(GridHighlightedLink::Url { link, uri }),
                    click,
                    ctx,
                );
            }
            return;
        }
        #[cfg(feature = "local_fs")]
        {
            // A historical block's cwd and session belong to that block, never to the new prompt.
            let (pwd, session_id, restored_local, paths) = {
                let model = self.model.lock();
                let block = match position {
                    WithinModel::BlockList(point) => model.block_list().block_at(point.block_index),
                    WithinModel::AltScreen(_) => Some(model.block_list().active_block()),
                };
                let Some(block) = block else { return };
                (
                    block.pwd().cloned(),
                    block.session_id(),
                    block.restored_block_was_local(),
                    model.possible_file_paths_at_point(position),
                )
            };
            let session = session_id.and_then(|id| self.sessions.as_ref(ctx).get(id));
            if !session
                .as_ref()
                .map(|session| session.is_local())
                .or(restored_local)
                .unwrap_or(false)
            {
                return;
            }
            let Some(pwd) = pwd else { return };
            let launch = session.and_then(|session| session.launch_data().cloned());
            let columns = self.size_info.columns;
            let paths = paths.collect::<Vec<_>>();
            let signature = paths
                .iter()
                .map(|path| {
                    let path = path.get_inner();
                    (
                        path.path.path.clone(),
                        path.path.line_and_column_num,
                        path.range.clone(),
                    )
                })
                .collect::<Vec<_>>();
            self.links.scan = Some(ctx.spawn(
                async move { Self::compute_valid_paths(&pwd, paths.into_iter(), columns, launch) },
                move |view, link, ctx| {
                    if generation != view.links.generation {
                        return;
                    }
                    let current = view
                        .model
                        .lock()
                        .possible_file_paths_at_point(position)
                        .map(|path| {
                            let path = path.get_inner();
                            (
                                path.path.path.clone(),
                                path.path.line_and_column_num,
                                path.range.clone(),
                            )
                        })
                        .collect::<Vec<_>>();
                    if current == signature {
                        view.finish_link(generation, link, click, ctx);
                    }
                },
            ));
        }
    }

    fn finish_link(
        &mut self,
        generation: u64,
        link: Option<GridHighlightedLink>,
        click: Option<ModifiersState>,
        ctx: &mut ViewContext<Self>,
    ) {
        if generation != self.links.generation {
            return;
        }
        let Some(position) = self.links.position else {
            return;
        };
        if self.links.context != self.link_context(position, ctx) {
            return;
        }
        if let Some(link) = link {
            self.links.source_text = self
                .model
                .lock()
                .link_at_range(&link.range(), RespectObfuscatedSecrets::No);
            self.links.highlighted = Some(link);
            ctx.set_cursor_shape(Cursor::PointingHand);
            ctx.notify();
            if click
                .or(self.links.pending_click.take())
                .is_some_and(|modifiers| should_directly_open_link(&modifiers))
            {
                self.open_grid_link(generation, ctx);
            }
        }
    }

    fn link_is_current(&self, link: &GridHighlightedLink, position: WithinModel<Point>) -> bool {
        // Revalidate at the side-effect boundary, even if a parser wakeup is still queued.
        {
            let model = self.model.lock();
            match link {
                GridHighlightedLink::Url { uri, .. } => {
                    model
                        .hyperlink_at_point(&position)
                        .map(|(_, target)| target)
                        .or_else(|| {
                            model.url_at_point(&position).map(|range| {
                                model.link_at_range(&range, RespectObfuscatedSecrets::No)
                            })
                        })
                        .as_ref()
                        == Some(uri)
                }
                #[cfg(feature = "local_fs")]
                GridHighlightedLink::File(_) => {
                    model.link_at_range(&link.range(), RespectObfuscatedSecrets::No)
                        == self.links.source_text
                }
            }
        }
    }

    pub(super) fn invalidate_changed_link(&mut self, ctx: &mut ViewContext<Self>) {
        if let Some(position) = self.links.position
            && (self.links.context != self.link_context(position, ctx)
                || self
                    .links
                    .highlighted
                    .as_ref()
                    .is_some_and(|link| !self.link_is_current(link, position)))
        {
            self.clear_link(ctx);
        }
    }

    pub(super) fn open_grid_link(&mut self, generation: u64, ctx: &mut ViewContext<Self>) {
        if generation != self.links.generation || self.links.dragged {
            return;
        }
        let Some(link) = self.links.highlighted.clone() else {
            return;
        };
        let Some(position) = self.links.position else {
            return;
        };
        if self.links.context != self.link_context(position, ctx) {
            self.clear_link(ctx);
            return;
        }
        let valid = self.link_is_current(&link, position);
        self.clear_link(ctx);
        if !valid {
            return;
        }
        match link {
            GridHighlightedLink::Url { uri, .. } => {
                if allowed_url(&uri) {
                    ctx.open_url(&uri);
                }
            }
            #[cfg(feature = "local_fs")]
            GridHighlightedLink::File(file) => {
                let file = file.get_inner();
                if !file.absolute_path.exists() {
                    ctx.show_native_platform_modal(
                        warpui::modals::AlertDialogWithCallbacks::for_view(
                            "The linked file no longer exists.",
                            "",
                            vec![warpui::modals::ModalButton::for_view(
                                "OK",
                                |_: &mut Self, _| {},
                            )],
                            |_, _| {},
                        ),
                    );
                    return;
                }
                let target = crate::util::openable_file_type::resolve_file_target(
                    &file.absolute_path,
                    EditorSettings::as_ref(ctx),
                    None,
                );
                // Terminal diagnostics use one-based columns; the built-in editor uses
                // zero-based columns. External editors keep the original diagnostic position.
                let line_col = match target {
                    FileTarget::CodeEditor(_) | FileTarget::MarkdownViewer(_) => {
                        file.line_and_column_num.map(|mut position| {
                            position.column_num =
                                position.column_num.map(|column| column.saturating_sub(1));
                            position
                        })
                    }
                    FileTarget::ExternalEditor(_)
                    | FileTarget::EnvEditor
                    | FileTarget::SystemDefault
                    | FileTarget::SystemGeneric => file.line_and_column_num,
                };
                ctx.emit(super::Event::OpenFileWithTarget {
                    path: file.absolute_path.clone(),
                    target,
                    line_col,
                });
            }
        }
    }

    pub(super) fn render_link_hint(&self, app: &AppContext) -> Option<Box<dyn warpui::Element>> {
        use warpui::elements::{DispatchEventResult, EventHandler};
        if !*GeneralSettings::as_ref(app).link_tooltip {
            return None;
        }
        let link = self.links.highlighted.as_ref()?;
        let generation = self.links.generation;
        let label = format!("{}  [Cmd + Click]", link.label());
        Some(
            EventHandler::new(
                crate::appearance::Appearance::as_ref(app)
                    .ui_builder()
                    .paragraph(label)
                    .build()
                    .finish(),
            )
            .on_mouse_in(|_, _, _| DispatchEventResult::StopPropagation, None)
            .on_left_mouse_up(move |ctx, _, _| {
                ctx.dispatch_typed_action(super::TerminalAction::OpenGridLink { generation });
                DispatchEventResult::StopPropagation
            })
            .finish(),
        )
    }
}

#[cfg(feature = "local_fs")]
impl TerminalView {
    fn compute_valid_paths(
        working_directory: &str,
        possible_paths: impl Iterator<Item = WithinModel<grid_handler::PossiblePath>>,
        max_columns: usize,
        shell_launch_data: Option<ShellLaunchData>,
    ) -> Option<GridHighlightedLink> {
        let mut link = None;
        'path_loop: for within_model_possible_path in possible_paths {
            let possible_path = within_model_possible_path.get_inner();

            // A file path at the end of a sentence often captures trailing prose
            // punctuation (e.g. `notes/README.md.` or `notes/README.md，`). Try the
            // punctuation-trimmed candidate first so the resolved file, highlight
            // range, and extension-based classification all exclude it. This must
            // run before the untrimmed lookup because on Windows the NT path
            // normalizer strips trailing dots, so the untrimmed path would
            // otherwise resolve and leave the period inside the captured link.
            if let Some(trimmed_path) =
                path_without_trailing_sentence_punctuation(&possible_path.path.path)
            {
                let trimmed_cleaned_path = CleanPathResult {
                    path: trimmed_path.path.into(),
                    line_and_column_num: possible_path.path.line_and_column_num,
                };
                if let Some(absolute_path) = absolute_path_if_valid(
                    &trimmed_cleaned_path,
                    ShellPathType::ShellNative(working_directory.to_string()),
                    shell_launch_data.as_ref(),
                ) {
                    let new_end_point = possible_path
                        .range
                        .end()
                        .wrapping_sub(max_columns, trimmed_path.removed_width);
                    link = Some(Self::create_valid_link(
                        absolute_path,
                        trimmed_cleaned_path.line_and_column_num,
                        *possible_path.range.start()..=new_end_point,
                        &within_model_possible_path,
                    ));
                    break 'path_loop;
                }
            }

            // We want to check if the clean path result is a valid path and get the canonical
            // absolute path back.
            let absolute_path = absolute_path_if_valid(
                &possible_path.path,
                ShellPathType::ShellNative(working_directory.to_string()),
                shell_launch_data.as_ref(),
            );

            if let Some(absolute_path) = absolute_path {
                link = Some(Self::create_valid_link(
                    absolute_path,
                    possible_path.path.line_and_column_num,
                    possible_path.range.clone(),
                    &within_model_possible_path,
                ));
                break;
            }

            for prefix in PREFIXES_TO_REMOVE {
                if let Some(new_possible_path) = possible_path.path.path.strip_prefix(prefix) {
                    let new_possible_cleaned_path = CleanPathResult {
                        path: new_possible_path.into(),
                        line_and_column_num: possible_path.path.line_and_column_num,
                    };
                    let absolute_path = absolute_path_if_valid(
                        &new_possible_cleaned_path,
                        ShellPathType::ShellNative(working_directory.to_string()),
                        shell_launch_data.as_ref(),
                    );

                    // check if new_possible_path is valid
                    if let Some(absolute_path) = absolute_path {
                        let new_start_point = possible_path
                            .range
                            .start()
                            .wrapping_add(max_columns, prefix.len());

                        link = Some(Self::create_valid_link(
                            absolute_path,
                            new_possible_cleaned_path.line_and_column_num,
                            new_start_point..=*possible_path.range.end(),
                            &within_model_possible_path,
                        ));

                        // break outer_loop
                        break 'path_loop;
                    }
                }
            }

            for suffix in SUFFIXES_TO_REMOVE {
                if let Some(new_possible_path) = possible_path.path.path.strip_suffix(suffix) {
                    let new_possible_cleaned_path = CleanPathResult {
                        path: new_possible_path.into(),
                        line_and_column_num: possible_path.path.line_and_column_num,
                    };
                    let absolute_path = absolute_path_if_valid(
                        &new_possible_cleaned_path,
                        ShellPathType::ShellNative(working_directory.to_string()),
                        shell_launch_data.as_ref(),
                    );

                    // check if new_possible_path is valid
                    if let Some(absolute_path) = absolute_path {
                        let new_end_point = possible_path
                            .range
                            .end()
                            .wrapping_sub(max_columns, suffix.len());

                        link = Some(Self::create_valid_link(
                            absolute_path,
                            new_possible_cleaned_path.line_and_column_num,
                            *possible_path.range.start()..=new_end_point,
                            &within_model_possible_path,
                        ));

                        // break outer_loop
                        break 'path_loop;
                    }
                }
            }
        }

        link.map(GridHighlightedLink::File)
    }

    fn create_valid_link(
        absolute_path: PathBuf,
        line_and_column_num: Option<LineAndColumnArg>,
        path_range: std::ops::RangeInclusive<Point>,
        possible_path: &WithinModel<grid_handler::PossiblePath>,
    ) -> WithinModel<FileLink> {
        let inner_link = FileLink {
            link: Link {
                range: path_range,
                is_empty: false,
            },
            absolute_path,
            line_and_column_num,
        };

        match possible_path {
            WithinModel::AltScreen(_) => WithinModel::AltScreen(inner_link),
            WithinModel::BlockList(inner) => {
                WithinModel::BlockList(WithinBlock::new(inner_link, inner.block_index, inner.grid))
            }
        }
    }
}

#[cfg(all(test, feature = "local_fs"))]
#[path = "link_detection_tests.rs"]
mod tests;
