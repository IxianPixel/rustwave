use crate::Message;
use iced::widget::{Container, Svg, column, container, row, svg, text};
use iced::{Alignment, Element, Font, Length, Theme, border};

/// Rounded, subtly tinted panel with a heading row (title plus an optional
/// count pill) above a body that fills the remaining height. Used to frame
/// each quadrant of multi-section pages.
pub fn section<'a>(
    title: &'a str,
    badge: Option<String>,
    body: impl Into<Element<'a, Message>>,
) -> Container<'a, Message> {
    let bold = Font {
        weight: iced::font::Weight::Bold,
        ..Font::DEFAULT
    };

    let mut heading = row![text(title).size(18).font(bold)]
        .spacing(8)
        .align_y(Alignment::Center);

    if let Some(label) = badge {
        heading = heading.push(container(text(label).size(12)).padding([2, 8]).style(
            |theme: &Theme| {
                let palette = theme.extended_palette();
                container::Style {
                    background: Some(palette.primary.weak.color.into()),
                    text_color: Some(palette.primary.weak.text),
                    border: border::rounded(999),
                    ..container::Style::default()
                }
            },
        ));
    }

    container(column![heading, body.into()].spacing(10))
        .padding(12)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|theme: &Theme| {
            let palette = theme.extended_palette();
            container::Style {
                background: Some(palette.background.weak.color.into()),
                border: border::rounded(12),
                ..container::Style::default()
            }
        })
}

/// Centered muted icon, title, and subtitle. Used for placeholder, empty,
/// and error states so a section never renders as a blank panel.
pub fn empty_state<'a>(
    icon_path: Option<String>,
    title: String,
    subtitle: String,
) -> Element<'a, Message> {
    let mut col = column![].spacing(8).align_x(Alignment::Center);

    if let Some(path) = icon_path {
        col = col.push(
            Svg::new(path)
                .width(42)
                .height(42)
                .style(|theme: &Theme, _status| svg::Style {
                    color: Some(theme.extended_palette().background.strong.color),
                }),
        );
    }

    col = col.push(text(title).size(16));
    if !subtitle.is_empty() {
        col = col.push(text(subtitle).size(13).style(text::secondary));
    }

    container(col).center(Length::Fill).into()
}

/// Centered indeterminate spinner for a section whose content is loading.
pub fn loading_state<'a>() -> Element<'a, Message> {
    container(super::spinner(36.0)).center(Length::Fill).into()
}
