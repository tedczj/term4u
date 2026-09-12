use pathfinder_geometry::vector::vec2f;
use warp_core::ui;
use warp_core::ui::color::blend::Blend as _;
use warpui::elements::{
    Align, ConstrainedBox, Container, CornerRadius, CrossAxisAlignment, Flex, MainAxisAlignment,
    MainAxisSize, MouseStateHandle, ParentElement as _, Radius,
};
use warpui::keymap::EditableBinding;
use warpui::platform::Cursor;
use warpui::ui_components::button::{ButtonVariant, TextAndIcon, TextAndIconAlignment};
use warpui::ui_components::components::{Coords, UiComponent, UiComponentStyles};
use warpui::{
    AppContext, Element, Entity, ModelHandle, SingletonEntity as _, TypedActionView, View,
    ViewContext, id,
};

use crate::appearance::Appearance;
use crate::pane_group::focus_state::PaneFocusHandle;
use crate::pane_group::pane::view;
use crate::pane_group::{BackingView, PaneConfiguration, PaneEvent};
use crate::util::bindings::{BindingGroup, CustomAction, keybinding_name_to_display_string};
use crate::workspace::WorkspaceAction;

pub fn init(app: &mut AppContext) {
    app.register_editable_bindings([EditableBinding::new(
        "workspace:new_tab",
        "Terminal session",
        GetStartedAction::TerminalSession,
    )
    .with_context_predicate(id!("GetStartedView"))
    .with_group(BindingGroup::Terminal.as_str())
    .with_custom_action(CustomAction::NewTab)]);
}

pub struct GetStartedView {
    pane_configuration: ModelHandle<PaneConfiguration>,
    focus_handle: Option<PaneFocusHandle>,
    terminal_session_button: MouseStateHandle,
}

impl GetStartedView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        Self {
            pane_configuration: ctx.add_model(|_| PaneConfiguration::new("Get started")),
            focus_handle: None,
            terminal_session_button: Default::default(),
        }
    }

    pub fn pane_configuration(&self) -> ModelHandle<PaneConfiguration> {
        self.pane_configuration.clone()
    }

    fn render_main_content(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let theme = appearance.theme();

        Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_children([
                appearance
                    .ui_builder()
                    .paragraph("Welcome to Term4u")
                    .with_style(UiComponentStyles {
                        font_size: Some(20.),
                        ..Default::default()
                    })
                    .build()
                    .finish(),
                Container::new(
                    appearance
                        .ui_builder()
                        .paragraph("A local terminal workspace")
                        .with_style(UiComponentStyles {
                            font_size: Some(14.),
                            font_family_id: Some(appearance.monospace_font_family()),
                            font_color: Some(
                                theme.disabled_text_color(theme.background()).into_solid(),
                            ),
                            ..Default::default()
                        })
                        .build()
                        .finish(),
                )
                .with_margin_top(4.)
                .with_margin_bottom(16.)
                .finish(),
                ConstrainedBox::new(
                    appearance
                        .ui_builder()
                        .button(ButtonVariant::Text, self.terminal_session_button.clone())
                        .with_style(UiComponentStyles {
                            padding: Some(Coords::uniform(8.)),
                            ..Default::default()
                        })
                        .with_hovered_styles(UiComponentStyles {
                            border_radius: Some(CornerRadius::with_all(Radius::Pixels(4.))),
                            background: Some(
                                theme.background().blend(&theme.surface_overlay_1()).into(),
                            ),
                            ..Default::default()
                        })
                        .with_text_and_icon_label(TextAndIcon::new(
                            TextAndIconAlignment::IconFirst,
                            format!(
                                "New terminal session  {}",
                                keybinding_name_to_display_string("workspace:new_tab", app)
                                    .unwrap_or_default()
                            ),
                            ui::Icon::Terminal.to_warpui_icon(theme.foreground()),
                            MainAxisSize::Min,
                            MainAxisAlignment::Center,
                            vec2f(16., 16.),
                        ))
                        .build()
                        .on_click(|ctx, _, _| {
                            ctx.dispatch_typed_action(GetStartedAction::TerminalSession)
                        })
                        .with_cursor(Cursor::PointingHand)
                        .finish(),
                )
                .with_max_width(360.)
                .finish(),
            ])
            .finish()
    }
}

impl Entity for GetStartedView {
    type Event = PaneEvent;
}

#[derive(Debug)]
pub enum GetStartedAction {
    TerminalSession,
}

impl TypedActionView for GetStartedView {
    type Action = GetStartedAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            GetStartedAction::TerminalSession => {
                ctx.dispatch_typed_action(&WorkspaceAction::AddTerminalTab {
                    hide_homepage: true,
                });
                self.close(ctx);
            }
        }
    }
}

impl View for GetStartedView {
    fn ui_name() -> &'static str {
        "GetStartedView"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        Align::new(self.render_main_content(app)).finish()
    }
}

impl BackingView for GetStartedView {
    type PaneHeaderOverflowMenuAction = ();
    type CustomAction = ();
    type AssociatedData = ();

    fn handle_pane_header_overflow_menu_action(
        &mut self,
        _: &Self::PaneHeaderOverflowMenuAction,
        _: &mut ViewContext<Self>,
    ) {
    }

    fn close(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.emit(PaneEvent::Close);
    }

    fn focus_contents(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.focus_self();
    }

    fn render_header_content(
        &self,
        _: &view::HeaderRenderContext<'_>,
        _: &AppContext,
    ) -> view::HeaderContent {
        view::HeaderContent::simple("Get started")
    }

    fn set_focus_handle(&mut self, focus_handle: PaneFocusHandle, _: &mut ViewContext<Self>) {
        self.focus_handle = Some(focus_handle);
    }
}
