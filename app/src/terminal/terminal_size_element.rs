use std::any::Any;

use async_channel::Sender;
use warpui::elements::Point;
use warpui::event::DispatchedEvent;
use warpui::geometry::rect::RectF;
use warpui::geometry::vector::Vector2F;
use warpui::{
    AfterLayoutContext, AppContext, Element, Event, EventContext, LayoutContext, PaintContext,
    SizeConstraint,
};

use super::view::TerminalAction;

pub struct TerminalSizeElement {
    child: Box<dyn Element>,
    resize_tx: Sender<Vector2F>,
    size: Option<Vector2F>,
    receives_input: bool,
}

impl TerminalSizeElement {
    pub fn new(resize_tx: Sender<Vector2F>, child: Box<dyn Element>, receives_input: bool) -> Self {
        TerminalSizeElement {
            child,
            resize_tx,
            size: None,
            receives_input,
        }
    }
}

impl Element for TerminalSizeElement {
    fn layout(
        &mut self,
        constraint: SizeConstraint,
        ctx: &mut LayoutContext,
        app: &AppContext,
    ) -> Vector2F {
        self.size = Some(constraint.max);
        self.child.layout(constraint, ctx, app);
        constraint.max
    }

    fn after_layout(&mut self, ctx: &mut AfterLayoutContext, app: &AppContext) {
        self.child.after_layout(ctx, app);

        let terminal_size = self.size().expect("Size should be present");
        // It's possible that the underlying shell session has been terminated
        // but we're showing a read-only terminal view, in which case, the
        // channel will be closed.  If we're unable to send a resize through
        // the channel, that's fine, just ignore the error.
        let _ = self.resize_tx.try_send(terminal_size);
    }

    fn paint(&mut self, origin: Vector2F, ctx: &mut PaintContext, app: &AppContext) {
        self.child.paint(origin, ctx, app)
    }

    fn size(&self) -> Option<Vector2F> {
        self.size
    }

    fn origin(&self) -> Option<Point> {
        self.child.origin()
    }

    fn bounds(&self) -> Option<RectF> {
        self.child.bounds()
    }

    fn parent_data(&self) -> Option<&dyn Any> {
        self.child.parent_data()
    }

    fn dispatch_event(
        &mut self,
        event: &DispatchedEvent,
        ctx: &mut EventContext,
        app: &AppContext,
    ) -> bool {
        let handled_by_child = self.child.dispatch_event(event, ctx, app);
        let Some(z_index) = self.z_index() else {
            return false;
        };

        if !handled_by_child && let Some(event_at_z_index) = event.at_z_index(z_index, ctx) {
            match event_at_z_index {
                Event::KeyDown {
                    chars,
                    is_composing: false,
                    ..
                } if self.receives_input
                    && !chars.is_empty()
                    && chars.chars().all(char::is_control) =>
                {
                    ctx.dispatch_typed_action(TerminalAction::KeyDown(chars.clone()));
                    return true;
                }
                Event::TypedCharacters { chars } if self.receives_input && !chars.is_empty() => {
                    ctx.dispatch_typed_action(TerminalAction::TypedCharacters(chars.clone()));
                    return true;
                }
                Event::DragFiles { location } => {
                    if self.mouse_position_is_in_bounds(*location) {
                        ctx.dispatch_typed_action(TerminalAction::StartFileDropTarget);
                    } else {
                        ctx.dispatch_typed_action(TerminalAction::StopFileDropTarget);
                    }
                    return true;
                }
                Event::DragFileExit => {
                    ctx.dispatch_typed_action(TerminalAction::StopFileDropTarget);
                    return true;
                }
                Event::DragAndDropFiles { paths, location } => {
                    if self.mouse_position_is_in_bounds(*location) && !paths.is_empty() {
                        let paths = paths.iter().map(std::path::PathBuf::from).collect();
                        ctx.dispatch_typed_action(TerminalAction::DragAndDropFiles(paths));
                    }
                    return true;
                }
                _ => {}
            };
        }
        handled_by_child
    }
}

impl TerminalSizeElement {
    fn mouse_position_is_in_bounds(&self, position: Vector2F) -> bool {
        let Some(bounds) = self.bounds() else {
            return false;
        };

        bounds.contains_point(position)
    }
}
