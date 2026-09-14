use std::collections::HashMap;
use std::ops::RangeInclusive;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use pathfinder_geometry::vector::{Vector2F, vec2f};
use warpui::Event;
use warpui::elements::{
    AfterLayoutContext, AppContext, Element, EventContext, LayoutContext, PaintContext, Point,
    SizeConstraint,
};
use warpui::event::DispatchedEvent;
use warpui::fonts::Properties;
use warpui::geometry::rect::RectF;
use warpui::text::SelectionType;
use warpui::units::{IntoLines, Lines};

use super::blockgrid_renderer::{BlockGridRenderer, GridRenderParams};
use crate::appearance::Appearance;
use crate::settings::EnforceMinimumContrast;
use crate::terminal::blockgrid_renderer::BlockGridParams;
use crate::terminal::grid_renderer::CellGlyphCache;
use crate::terminal::model::ObfuscateSecrets;
use crate::terminal::model::blockgrid::BlockGrid;
use crate::terminal::model::blocks::{BlockListPoint, SelectionRange};
use crate::terminal::model::grid::Dimensions;
use crate::terminal::model::index::{Point as GridPoint, Side};
use crate::terminal::model::selection::{SelectAction, SelectionPoint};
use crate::terminal::model::terminal_model::BlockIndex;
use crate::terminal::view::TerminalAction;
use crate::terminal::{
    SizeInfo, color, context_menu_offset, grid_renderer, should_right_click_paste,
};

struct GridSelection {
    first_row: Lines,
    ranges: Vec<(SelectionPoint, SelectionPoint)>,
    // Drag events can cross grids before another frame copies state from the view.
    dragging: Arc<AtomicBool>,
}

pub struct BlockGridElement {
    block_grid: BlockGrid,
    block_grid_params: BlockGridParams,
    size: Vector2F,
    origin: Option<Point>,
    bounds: Option<RectF>,
    selection: Option<GridSelection>,
    find_matches: Vec<RangeInclusive<GridPoint>>,
    focused_find_match: Option<RangeInclusive<GridPoint>>,
    context_menu: Option<ContextMenuAnchor>,
}

/// Set when this grid belongs to a block that can raise the transcript context menu.
struct ContextMenuAnchor {
    /// `SavePosition` id of the terminal view's content container, used to turn a window
    /// position into an offset the menu overlay understands.
    position_id: String,
    block_index: BlockIndex,
}

impl BlockGridElement {
    pub fn new(
        block_grid: &BlockGrid,
        appearance: &Appearance,
        enforce_minimum_contrast: EnforceMinimumContrast,
        obfuscate_secrets: ObfuscateSecrets,
        size_info: SizeInfo,
    ) -> Self {
        let theme = appearance.theme();
        let cell_size = Vector2F::new(
            size_info.cell_width_px().as_f32(),
            size_info.cell_height_px().as_f32(),
        );
        let size = vec2f(
            block_grid.grid_handler().columns() as f32,
            block_grid.len_displayed() as f32,
        ) * cell_size;
        Self {
            block_grid: block_grid.clone(),
            block_grid_params: BlockGridParams {
                grid_render_params: GridRenderParams {
                    warp_theme: theme.clone(),
                    font_family: appearance.monospace_font_family(),
                    font_size: appearance.monospace_font_size(),
                    font_weight: appearance.monospace_font_weight(),
                    line_height_ratio: appearance.ui_builder().line_height_ratio(),
                    enforce_minimum_contrast,
                    obfuscate_secrets,
                    size_info,
                    cell_size,
                    use_ligature_rendering: false,
                    hide_cursor_cell: false,
                },
                colors: color::List::from(&color::Colors::from(theme.clone())),
                override_colors: color::OverrideList::empty(),
                bounds: Default::default(),
            },
            size,
            origin: None,
            bounds: None,
            selection: None,
            find_matches: Vec::new(),
            focused_find_match: None,
            context_menu: None,
        }
    }

    /// Lets a right-click inside this grid open the transcript context menu for `block_index`.
    pub fn with_context_menu(mut self, position_id: String, block_index: BlockIndex) -> Self {
        self.context_menu = Some(ContextMenuAnchor {
            position_id,
            block_index,
        });
        self
    }

    pub fn with_find_matches(
        mut self,
        matches: Vec<RangeInclusive<GridPoint>>,
        focused: Option<RangeInclusive<GridPoint>>,
    ) -> Self {
        self.find_matches = matches;
        self.focused_find_match = focused;
        self
    }

    pub fn with_selection(
        mut self,
        first_row: Lines,
        ranges: &[SelectionRange],
        dragging: Arc<AtomicBool>,
    ) -> Self {
        let last_row = first_row + (self.block_grid.len_displayed() as f32).into_lines();
        let ranges = ranges
            .iter()
            .filter_map(|range| {
                if range.end.row < first_row || range.start.row >= last_row {
                    return None;
                }
                let start = SelectionPoint {
                    row: (range.start.row - first_row).max(Lines::zero()),
                    col: if range.start.row < first_row {
                        0
                    } else {
                        range.start.column
                    },
                };
                let end = SelectionPoint {
                    row: (range.end.row - first_row).min(
                        (self.block_grid.len_displayed().saturating_sub(1) as f32).into_lines(),
                    ),
                    col: if range.end.row >= last_row {
                        self.block_grid.grid_handler().columns()
                    } else {
                        range.end.column
                    },
                };
                Some((start, end))
            })
            .collect();
        self.selection = Some(GridSelection {
            first_row,
            ranges,
            dragging,
        });
        self
    }

    fn selection_point(&self, position: Vector2F) -> (BlockListPoint, Side) {
        let origin = self.bounds.expect("grid was painted").origin();
        let local = position - origin;
        let cell = self.block_grid_params.grid_render_params.cell_size;
        let row = (local.y() / cell.y()).floor().max(0.);
        let column = (local.x() / cell.x()).max(0.);
        let first_row = self
            .selection
            .as_ref()
            .expect("grid is selectable")
            .first_row;
        let point = BlockListPoint::new(
            first_row + row.into_lines(),
            (column.floor() as usize)
                .min(self.block_grid.grid_handler().columns().saturating_sub(1)),
        );
        let side = if column.fract() < 0.5 {
            Side::Left
        } else {
            Side::Right
        };
        (point, side)
    }

    pub fn with_ligature_rendering(mut self) -> Self {
        self.block_grid_params
            .grid_render_params
            .use_ligature_rendering = true;
        self
    }

    /// Returns the underlying text of the `BlockGrid`.
    pub fn text(&self) -> String {
        self.block_grid.contents_to_string(false, None)
    }
}

impl Element for BlockGridElement {
    fn layout(
        &mut self,
        constraint: SizeConstraint,
        _ctx: &mut LayoutContext,
        _app: &AppContext,
    ) -> Vector2F {
        self.size = self.size.min(constraint.max);
        self.size
    }

    fn after_layout(&mut self, _ctx: &mut AfterLayoutContext, _app: &AppContext) {}

    fn paint(&mut self, origin: Vector2F, ctx: &mut PaintContext, app: &AppContext) {
        self.origin = Some(Point::from_vec2f(origin, ctx.scene.z_index()));
        let bounds = RectF::new(origin, self.size);

        self.block_grid_params.bounds = bounds;
        self.bounds = Some(bounds);
        self.block_grid.draw(
            origin,
            origin,
            &mut CellGlyphCache::default(),
            255,
            None,
            None,
            None,
            Some(self.find_matches.iter()),
            self.focused_find_match.as_ref(),
            Properties::default(),
            &self.block_grid_params,
            None,
            &HashMap::new(),
            ctx,
            app,
        );
        if let Some(selection) = &self.selection {
            for (start, end) in &selection.ranges {
                grid_renderer::render_selection(
                    start,
                    end,
                    &self.block_grid_params.grid_render_params.size_info,
                    Lines::zero(),
                    origin,
                    self.block_grid_params
                        .grid_render_params
                        .warp_theme
                        .text_selection_color()
                        .into_solid(),
                    ctx,
                );
            }
        }
    }

    fn dispatch_event(
        &mut self,
        event: &DispatchedEvent,
        ctx: &mut EventContext,
        app: &AppContext,
    ) -> bool {
        let Some(z_index) = self.z_index() else {
            return false;
        };

        // Right-click is handled before the selection guard below: the context menu must open
        // whether or not this grid currently carries a selection.
        if let Some(Event::RightMouseDown {
            position, shift, ..
        }) = event.at_z_index(z_index, ctx)
            && let Some(anchor) = &self.context_menu
            && self
                .bounds
                .is_some_and(|bounds| bounds.contains_point(*position))
        {
            let action = if should_right_click_paste(*shift, app) {
                TerminalAction::Paste
            } else {
                TerminalAction::BlockContextMenu {
                    position: context_menu_offset(ctx, Some(&anchor.position_id), *position),
                    block_index: Some(anchor.block_index),
                }
            };
            ctx.dispatch_typed_action(action);
            return true;
        }

        let Some(selection) = &self.selection else {
            return false;
        };
        match event.at_z_index(z_index, ctx) {
            Some(Event::LeftMouseDown {
                position,
                click_count,
                ..
            }) if self
                .bounds
                .is_some_and(|bounds| bounds.contains_point(*position)) =>
            {
                selection.dragging.store(true, Ordering::Relaxed);
                let (point, side) = self.selection_point(*position);
                ctx.dispatch_typed_action(TerminalAction::SelectOutput(SelectAction::Begin {
                    point,
                    side,
                    selection_type: SelectionType::from_click_count(*click_count),
                    position: *position,
                }));
                true
            }
            Some(Event::LeftMouseDragged { position, .. })
                if selection.dragging.load(Ordering::Relaxed)
                    && self
                        .bounds
                        .is_some_and(|bounds| bounds.contains_point(*position)) =>
            {
                let (point, side) = self.selection_point(*position);
                ctx.dispatch_typed_action(TerminalAction::SelectOutput(SelectAction::Update {
                    point,
                    side,
                    delta: Lines::zero(),
                    position: *position,
                }));
                true
            }
            Some(Event::LeftMouseUp { .. })
                if selection.dragging.swap(false, Ordering::Relaxed) =>
            {
                ctx.dispatch_typed_action(TerminalAction::SelectOutput(SelectAction::End));
                true
            }
            _ => false,
        }
    }

    fn size(&self) -> Option<Vector2F> {
        Some(self.size)
    }

    fn origin(&self) -> Option<Point> {
        self.origin
    }
}
