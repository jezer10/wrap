use std::borrow::Cow;

use warp_core::ui::appearance::Appearance;
use warp_core::ui::design;
use warpui_core::keymap::Keystroke;
use warpui_core::prelude::*;

use crate::Component;

/// Use a smaller-than-normal font size for the tooltip text to make it more compact.
const UI_FONT_SIZE_ADJUSTMENT: f32 = -2.;

#[derive(Default)]
pub struct Tooltip;

pub struct Params {
    pub label: Cow<'static, str>,
    pub options: Options,
}

impl crate::Params for Params {
    type Options<'a> = Options;
}

pub struct Options {
    pub keyboard_shortcut: Option<Keystroke>,
}

impl crate::Options for Options {
    fn default(_: &Appearance) -> Self {
        Self {
            keyboard_shortcut: None,
        }
    }
}

impl Component for Tooltip {
    type Params<'a> = Params;

    fn render<'a>(
        &self,
        appearance: &Appearance,
        params: Self::Params<'a>,
    ) -> Box<dyn warpui_core::Element> {
        let font_family = appearance.ui_font_family();
        let font_size = appearance.ui_font_size() + UI_FONT_SIZE_ADJUSTMENT;

        let has_shortcut = params.options.keyboard_shortcut.is_some();
        let mut content = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(8.)
            .with_child(
                Text::new(params.label, font_family, font_size)
                    .soft_wrap(false)
                    .with_color(ColorU::new(250, 250, 250, 255))
                    .finish(),
            );

        if let Some(keystroke) = params.options.keyboard_shortcut {
            content.add_child(
                Container::new(
                    Text::new(
                        keystroke.displayed().replace(' ', "+"),
                        font_family,
                        font_size,
                    )
                    .with_color(ColorU::new(250, 250, 250, 255))
                    .with_line_height_ratio(1.)
                    .finish(),
                )
                .with_horizontal_padding(5.)
                .with_vertical_padding(3.)
                .with_background(ColorU::new(80, 81, 82, 255))
                .with_corner_radius(CornerRadius::with_all(design::COMPACT_RADIUS))
                .finish(),
            );
        }

        Container::new(content.finish())
            .with_padding_left(12.)
            .with_padding_right(if has_shortcut { 6. } else { 12. })
            .with_vertical_padding(6.)
            .with_background(ColorU::new(35, 36, 37, 255))
            .with_corner_radius(CornerRadius::with_all(design::SURFACE_RADIUS))
            .with_drop_shadow(DropShadow {
                color: ColorU::new(0, 0, 0, 40),
                offset: vec2f(0., 3.),
                blur_radius: 12.,
                spread_radius: 0.,
            })
            .finish()
    }
}
