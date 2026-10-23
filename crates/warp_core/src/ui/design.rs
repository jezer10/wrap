//! Shared Soft UI styling for the desktop application's controls and floating surfaces.

use warpui_core::color::ColorU;
use warpui_core::geometry::vector::vec2f;
use warpui_core::scene::{DropShadow, Radius};

use super::theme::color::internal_colors;
use super::theme::{ColorScheme, Fill, WarpTheme};

pub const COMPACT_RADIUS: Radius = Radius::Pixels(4.);
pub const CONTROL_RADIUS: Radius = Radius::Pixels(8.);
pub const SURFACE_RADIUS: Radius = Radius::Pixels(12.);

pub fn hover_background(theme: &WarpTheme) -> Fill {
    theme.surface_3().with_opacity(45)
}

pub fn selected_background(theme: &WarpTheme) -> Fill {
    theme.surface_2().with_opacity(45)
}

/// Decorative separators; interactive boundaries retain their own contrast and focus styles.
pub fn subtle_border(theme: &WarpTheme) -> Fill {
    theme.outline().with_opacity(35)
}

pub fn control_border(theme: &WarpTheme) -> Fill {
    theme.foreground().with_opacity(20)
}

pub fn popup_shadow(theme: &WarpTheme) -> DropShadow {
    let alpha = match theme.inferred_color_scheme() {
        ColorScheme::DarkOnLight => 24,
        ColorScheme::LightOnDark => 64,
    };
    DropShadow {
        color: ColorU::new(0, 0, 0, alpha),
        offset: vec2f(0., 4.),
        blur_radius: 12.,
        spread_radius: 16.,
    }
}

pub fn control_shadow(theme: &WarpTheme) -> DropShadow {
    DropShadow {
        offset: vec2f(0., 1.),
        blur_radius: 4.,
        spread_radius: 0.,
        ..popup_shadow(theme)
    }
}

pub fn switch_track(theme: &WarpTheme, checked: bool, disabled: bool) -> Fill {
    if disabled {
        internal_colors::neutral_3(theme).into()
    } else if checked {
        theme.accent()
    } else {
        internal_colors::neutral_5(theme).into()
    }
}
