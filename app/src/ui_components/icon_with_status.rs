use warp_core::ui::icons::Icon as WarpIcon;
use warp_core::ui::theme::color::internal_colors;
use warp_core::ui::theme::{Fill as WarpThemeFill, WarpTheme};
use warpui::elements::{ConstrainedBox, Container, Element, ParentElement};

pub(crate) const CIRCLE_RATIO: f32 = 0.76;
const NEUTRAL_GLYPH_RATIO: f32 = 16.0 / 24.0;

pub(crate) enum IconWithStatusVariant {
    Neutral {
        icon: WarpIcon,
        icon_color: WarpThemeFill,
    },
    NeutralElement {
        icon_element: Box<dyn Element>,
    },
}

pub(crate) fn render_icon_with_status(
    variant: IconWithStatusVariant,
    total_size: f32,
    _overlay_extra_overhang_ratio: f32,
    theme: &WarpTheme,
    _status_container_background: WarpThemeFill,
) -> Box<dyn Element> {
    let icon = match variant {
        IconWithStatusVariant::Neutral { icon, icon_color } => {
            icon.to_warpui_icon(icon_color).finish()
        }
        IconWithStatusVariant::NeutralElement { icon_element } => icon_element,
    };
    Container::new(
        ConstrainedBox::new(icon)
            .with_width(total_size * NEUTRAL_GLYPH_RATIO)
            .with_height(total_size * NEUTRAL_GLYPH_RATIO)
            .finish(),
    )
    .with_background(internal_colors::fg_overlay_2(theme))
    .with_width(total_size)
    .with_height(total_size)
    .finish()
}
