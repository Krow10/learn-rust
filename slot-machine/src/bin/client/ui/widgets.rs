use ansi_to_tui::IntoText;
use cfonts::{render::RenderedString, Fonts, Options};
use ratatui::{
    prelude::{Buffer, Rect},
    style::{Modifier, Style, Stylize},
    widgets::Widget,
};

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum AlignCenter {
    None,
    Vertical,
    Horizontal,
    Both,
}

#[derive(Debug, Clone)]
pub struct CFontTextWidget {
    options: Options,
    enable_text_scaling: bool,
    align_center: AlignCenter,
    text_style_modifiers: Vec<Modifier>,
    animated_bold_line: isize,
}

#[allow(dead_code)]
impl CFontTextWidget {
    pub fn options(mut self, options: Options) -> Self {
        self.options = options;
        self
    }

    pub fn enable_text_scaling(mut self, enable_text_scaling: bool) -> Self {
        self.enable_text_scaling = enable_text_scaling;
        self
    }

    pub fn align_center(mut self, align_center: AlignCenter) -> Self {
        self.align_center = align_center;
        self
    }

    pub fn text_style_modifiers(mut self, text_style_modifiers: Vec<Modifier>) -> Self {
        self.text_style_modifiers = text_style_modifiers;
        self
    }

    pub fn animated_bold_line(mut self, animated_bold_line: isize) -> Self {
        self.animated_bold_line = animated_bold_line;
        self
    }

    fn stylized_text(&self, options_override: Option<Options>) -> RenderedString {
        if options_override.is_some() {
            cfonts::render(options_override.unwrap())
        } else {
            cfonts::render(self.options.clone())
        }
    }
}

impl Default for CFontTextWidget {
    fn default() -> Self {
        Self {
            options: Options::default(),
            enable_text_scaling: true,
            align_center: AlignCenter::None,
            text_style_modifiers: vec![],
            animated_bold_line: -1,
        }
    }
}

impl Widget for CFontTextWidget {
    fn render(mut self, area: Rect, buf: &mut Buffer) {
        let mut stylized_text = self.stylized_text(None).vec;
        let mut text_height = stylized_text.len() as u16;

        // Check that the current stylized text can fit on the screen (height check only)
        // If not, use a smaller font if text scaling is enabled
        if self.enable_text_scaling
            && self.options.font != Fonts::FontConsole
            && stylized_text.len() as u16
                + (area.height.saturating_sub(1).saturating_sub(text_height)) / 2
                >= area.height
        {
            let options = self.options.clone();
            stylized_text = self
                .stylized_text(Some(Options {
                    // Progressive font scaling from (any) => (tiny) => (console) to fit on the screen
                    font: if options.font == Fonts::FontTiny {
                        Fonts::FontConsole
                    } else {
                        Fonts::FontTiny
                    },
                    ..options
                }))
                .vec;
            text_height = stylized_text.len() as u16;
        }

        if self.animated_bold_line > 0 {
            self.animated_bold_line %= text_height as isize;
        }

        stylized_text.iter().enumerate().for_each(|(j, s)| {
            let mut lines = s.replace("\n", "").as_bytes().into_text().unwrap().lines;
            let text_width = lines.iter().fold(0u16, |acc, l| acc.max(l.width() as u16));

            if !self.text_style_modifiers.is_empty() {
                lines.iter_mut().for_each(|l| {
                    l.spans.iter_mut().for_each(|s| {
                        s.patch_style(
                            Style::new().add_modifier(
                                self.text_style_modifiers
                                    .iter()
                                    .fold(Modifier::empty(), |acc, m| acc | *m),
                            ),
                        )
                    });
                })
            }

            for l in lines.iter_mut() {
                if j as isize == self.animated_bold_line {
                    l.spans.iter_mut().for_each(|s| {
                        s.patch_style(Style::new().bold());
                    });
                }

                let mut center_shift = (
                    area.width.saturating_sub(text_width) / 2,
                    (area.height.saturating_sub(text_height) / 2).wrapping_add(0),
                );

                center_shift = match self.align_center {
                    AlignCenter::None => (0, 0),
                    AlignCenter::Vertical => (0, center_shift.1),
                    AlignCenter::Horizontal => (center_shift.0, 0),
                    AlignCenter::Both => center_shift,
                };

                buf.set_line(
                    area.left() + center_shift.0,
                    area.top() + j as u16 + center_shift.1,
                    &l,
                    area.width,
                );
            }
        });
    }
}
