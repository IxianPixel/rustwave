mod playback_bar;
mod playlist_widget;
mod section;
mod spinner;
mod track_widget;
mod user_widget;
mod waveform_widget;

pub use playback_bar::get_playback_bar;
pub use playlist_widget::get_playlist_widget;
pub use section::{empty_state, loading_state, section};
pub use spinner::spinner;
pub use track_widget::get_track_widget;
pub use user_widget::get_user_widget;
pub use waveform_widget::get_waveform_widget;

use iced::Theme;
use iced::widget::scrollable;

/// Shared scrollbar style: an accent-coloured scroller that pops against the
/// dark theme, reusing the framework defaults for everything else.
pub fn scrollbar_style(theme: &Theme, status: scrollable::Status) -> scrollable::Style {
    let palette = theme.extended_palette();
    let accent = palette.primary.strong.color;

    let mut style = scrollable::default(theme, status);
    style.vertical_rail.scroller.background = accent.into();
    style.horizontal_rail.scroller.background = accent.into();
    style
}
