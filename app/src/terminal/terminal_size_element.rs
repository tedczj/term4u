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
    resize_tx: Option<Sender<Vector2F>>,
    size: Option<Vector2F>,
    origin: Option<Point>,
    bounds: Option<RectF>,
    receives_input: bool,
    accepts_files: bool,
}

impl TerminalSizeElement {
    pub fn new(resize_tx: Sender<Vector2F>, child: Box<dyn Element>, receives_input: bool) -> Self {
        Self {
            child,
            resize_tx: Some(resize_tx),
            size: None,
            origin: None,
            bounds: None,
            receives_input,
            accepts_files: false,
        }
    }

    /// Owns file drops over the entire pane, including the command editor. Keeping it outside
    /// the output-size wrapper avoids reporting the input's height as usable PTY rows.
    pub fn file_drop_target(child: Box<dyn Element>) -> Self {
        Self {
            child,
            resize_tx: None,
            size: None,
            origin: None,
            bounds: None,
            receives_input: false,
            accepts_files: true,
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
        let child_size = self.child.layout(constraint, ctx, app);
        let size = if self.resize_tx.is_some() {
            constraint.max
        } else {
            child_size
        };
        self.size = Some(size);
        size
    }

    fn after_layout(&mut self, ctx: &mut AfterLayoutContext, app: &AppContext) {
        self.child.after_layout(ctx, app);
        if let (Some(tx), Some(size)) = (&self.resize_tx, self.size)
            && size.x().is_finite()
            && size.y().is_finite()
            && size.x() > 0.
            && size.y() > 0.
        {
            // A read-only terminal can outlive its shell and closed resize channel.
            let _ = tx.try_send(size);
        }
    }

    fn paint(&mut self, origin: Vector2F, ctx: &mut PaintContext, app: &AppContext) {
        self.origin = Some(Point::from_vec2f(origin, ctx.scene.z_index()));
        self.bounds = self.size.map(|size| RectF::new(origin, size));
        self.child.paint(origin, ctx, app);
    }

    fn size(&self) -> Option<Vector2F> {
        self.size
    }
    fn origin(&self) -> Option<Point> {
        self.origin
    }
    fn bounds(&self) -> Option<RectF> {
        self.bounds
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
        if self.child.dispatch_event(event, ctx, app) {
            return true;
        }
        let Some(z_index) = self.z_index() else {
            return false;
        };
        let Some(event) = event.at_z_index(z_index, ctx) else {
            return false;
        };
        match event {
            Event::MouseMoved { .. } if self.resize_tx.is_some() => {
                ctx.dispatch_typed_action(TerminalAction::MaybeLinkHover { position: None });
                false
            }
            Event::KeyDown {
                chars,
                is_composing: false,
                ..
            } if self.receives_input
                && !chars.is_empty()
                && chars.chars().all(char::is_control) =>
            {
                ctx.dispatch_typed_action(TerminalAction::KeyDown(chars.clone()));
                true
            }
            Event::TypedCharacters { chars } if self.receives_input && !chars.is_empty() => {
                ctx.dispatch_typed_action(TerminalAction::TypedCharacters(chars.clone()));
                true
            }
            Event::SetMarkedText {
                marked_text,
                selected_range,
            } if self.receives_input => {
                ctx.dispatch_typed_action(TerminalAction::SetMarkedText {
                    text: marked_text.clone(),
                    selected_range: selected_range.clone(),
                });
                true
            }
            Event::ClearMarkedText if self.receives_input => {
                ctx.dispatch_typed_action(TerminalAction::ClearMarkedText);
                true
            }
            Event::DragFiles { location } if self.accepts_files => {
                ctx.dispatch_typed_action(if self.contains(*location) {
                    TerminalAction::StartFileDropTarget
                } else {
                    TerminalAction::StopFileDropTarget
                });
                // Hover/exit must reach sibling panes so their stale highlight can be cleared.
                false
            }
            Event::DragFileExit if self.accepts_files => {
                ctx.dispatch_typed_action(TerminalAction::StopFileDropTarget);
                false
            }
            Event::DragAndDropFiles { paths, location }
                if self.accepts_files && self.contains(*location) && !paths.is_empty() =>
            {
                ctx.dispatch_typed_action(TerminalAction::DragAndDropFiles(
                    paths.iter().map(std::path::PathBuf::from).collect(),
                ));
                true
            }
            _ => false,
        }
    }
}

impl TerminalSizeElement {
    fn contains(&self, position: Vector2F) -> bool {
        self.bounds.is_some_and(|bounds| {
            // Half-open bounds assign shared split edges to at most one pane.
            position.x() >= bounds.origin().x()
                && position.y() >= bounds.origin().y()
                && position.x() < bounds.origin().x() + bounds.size().x()
                && position.y() < bounds.origin().y() + bounds.size().y()
        })
    }
}
