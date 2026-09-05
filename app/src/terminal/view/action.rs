use std::path::PathBuf;

use pathfinder_geometry::vector::Vector2F;
use warpui::event::ModifiersState;
use warpui::units::Lines;

use crate::terminal::available_shells::AvailableShell;
use crate::terminal::model::index::Point;
use crate::terminal::model::mouse::MouseState;
use crate::terminal::model::selection::SelectAction;
use crate::terminal::model::terminal_model::WithinModel;

#[derive(Clone, Debug)]
pub enum TerminalAction {
    Scroll { delta: Lines },
    AltScroll { delta: i32, point: Point },
    AltSelect(SelectAction<Point>),
    AltMouseAction(MouseState),
    AltScreenContextMenu { position: Vector2F },
    MaybeClearAltSelect,
    ClickOnGrid {
        position: WithinModel<Point>,
        modifiers: ModifiersState,
    },
    MiddleClickOnGrid { position: Option<WithinModel<Point>> },
    MaybeDismissToolTip { from_keybinding: bool },
    MaybeHoverSecret,
    MaybeLinkHover,
    Paste,
    Copy,
    ClearBuffer,
    Focus,
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
    CtrlD,
    CtrlC,
    ClearMarkedText,
    SetMarkedText(String),
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
