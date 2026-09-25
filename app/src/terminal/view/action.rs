use std::path::PathBuf;

use pathfinder_geometry::vector::Vector2F;
use warpui::event::ModifiersState;
use warpui::units::Lines;

use crate::terminal::available_shells::AvailableShell;
use crate::terminal::model::index::Point;
use crate::terminal::model::mouse::MouseState;
use crate::terminal::model::selection::SelectAction;
use crate::terminal::model::terminal_model::{BlockIndex, WithinModel};

#[derive(Clone, Debug)]
pub enum TerminalAction {
    LocalOutput(super::LocalOutputAction),
    Scroll {
        delta: Lines,
    },
    AltScroll {
        delta: i32,
        point: Point,
    },
    AltSelect(SelectAction<Point>),
    SelectOutput(SelectAction<crate::terminal::model::blocks::BlockListPoint>),
    AltMouseAction(MouseState),
    AltScreenContextMenu {
        position: Vector2F,
    },
    /// Opens the transcript context menu. `block_index` identifies the block that was
    /// right-clicked, and is `None` when the click landed outside of any block.
    BlockContextMenu {
        position: Vector2F,
        block_index: Option<BlockIndex>,
    },
    CloseContextMenu,
    /// Runs after queued focus effects from a selected menu action.
    RestoreContextMenuFocus {
        generation: u64,
    },
    CopyBlockCommand(BlockIndex),
    CopyBlockOutput(BlockIndex),
    CopyBlock(BlockIndex),
    InsertSelectedTextIntoInput,
    MaybeClearAltSelect,
    ClickOnGrid {
        position: WithinModel<Point>,
        modifiers: ModifiersState,
    },
    MiddleClickOnGrid {
        position: Option<WithinModel<Point>>,
    },
    MaybeDismissToolTip {
        from_keybinding: bool,
    },
    MaybeHoverSecret,
    MaybeLinkHover {
        position: Option<WithinModel<Point>>,
    },
    OpenGridLink {
        generation: u64,
    },
    Paste,
    Copy,
    ClearBuffer,
    Focus,
    FinishSelection,
    FocusInputAndClearSelection,
    ShowFindBar,
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
    UserInputSequence(Vec<u8>),
    ControlSequence(Vec<u8>),
    KeyDown(String),
    TypedCharacters(String),
    CtrlC,
    ClearMarkedText,
    SetMarkedText {
        text: String,
        selected_range: std::ops::Range<usize>,
    },
    Close,
    ToggleMaximizePane,
    SplitRight(Option<AvailableShell>),
    SplitLeft(Option<AvailableShell>),
    SplitDown(Option<AvailableShell>),
    SplitUp(Option<AvailableShell>),
    StartFileDropTarget,
    StopFileDropTarget,
    DragAndDropFiles(Vec<PathBuf>),
}
