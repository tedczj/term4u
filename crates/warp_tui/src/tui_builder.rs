use pathfinder_color::ColorU;
use warp::tui_export::Appearance;
use warp_core::ui::color::Opacity;
use warp_core::ui::color::blend::Blend;
use warp_core::ui::theme::{Fill as ThemeFill, WarpTheme};
use warpui::SingletonEntity;
use warpui_core::AppContext;
use warpui_core::elements::Fill as CoreFill;
use warpui_core::elements::tui::{Color, Modifier, TuiStyle};

use crate::terminal_background::probed_colors;

#[derive(Clone, Debug)]
pub(crate) struct TuiUiBuilder {
    theme: WarpTheme,
}

impl TuiUiBuilder {
    pub(crate) fn from_app(app: &AppContext) -> Self {
        Self {
            theme: Appearance::as_ref(app).theme().clone(),
        }
    }

    pub(crate) fn primary_text_style(&self) -> TuiStyle {
        TuiStyle::default().fg(self.foreground_text_color(self.theme.details().main_text_opacity))
    }

    pub(crate) fn muted_text_style(&self) -> TuiStyle {
        TuiStyle::default().fg(self.foreground_text_color(self.theme.details().sub_text_opacity))
    }

    pub(crate) fn dim_text_style(&self) -> TuiStyle {
        self.muted_text_style().add_modifier(Modifier::DIM)
    }

    pub(crate) fn shell_command_background(&self) -> Color {
        let accent = ThemeFill::from(self.theme.terminal_colors().bright.green);
        cell_color(self.base_background().blend(&accent.with_opacity(10)))
    }

    pub(crate) fn shell_command_prefix_style(&self) -> TuiStyle {
        TuiStyle::default()
            .fg(cell_color(ThemeFill::from(
                self.theme.terminal_colors().bright.green,
            )))
            .add_modifier(Modifier::BOLD)
    }

    pub(crate) fn shell_command_row_style(&self) -> TuiStyle {
        self.shell_command_prefix_style()
            .bg(self.shell_command_background())
    }

    pub(crate) fn selection_style(&self) -> TuiStyle {
        TuiStyle::default()
            .fg(cell_color(self.base_background()))
            .bg(cell_color(self.theme.foreground()))
            .remove_modifier(Modifier::REVERSED)
    }

    fn foreground_text_color(&self, opacity: Opacity) -> Color {
        cell_color(
            self.base_background()
                .blend(&self.theme.foreground().with_opacity(opacity)),
        )
    }

    fn base_background(&self) -> ThemeFill {
        match probed_colors().bg {
            Some(background) => ThemeFill::Solid(ColorU::new(
                background.r,
                background.g,
                background.b,
                u8::MAX,
            )),
            None => self.theme.background(),
        }
    }
}

fn cell_color(fill: ThemeFill) -> Color {
    CoreFill::from(fill).into()
}

#[cfg(test)]
#[path = "tui_builder_tests.rs"]
mod tests;
