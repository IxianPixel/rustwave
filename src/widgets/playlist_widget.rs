use crate::Message;
use crate::models::SoundCloudPlaylist;
use crate::utilities::{NumberFormat, get_asset_path};
use iced::widget::{MouseArea, Row, mouse_area, text};
use iced::widget::{column, container, image, image::Handle};

// Artwork is 100px tall; keep the title short enough that the card stays that height.
const MAX_TITLE_CHARS: usize = 40;

fn truncate_title(title: &str) -> String {
    if title.chars().count() > MAX_TITLE_CHARS {
        let truncated: String = title.chars().take(MAX_TITLE_CHARS).collect();
        format!("{}…", truncated.trim_end())
    } else {
        title.to_string()
    }
}

pub fn get_playlist_widget<F>(
    playlist: &'_ SoundCloudPlaylist,
    image_handle: Option<Handle>,
    load_playlist: F,
) -> MouseArea<'_, Message>
where
    F: Fn(SoundCloudPlaylist) -> Message + 'static,
{
    let mut row = Row::new();

    // Add image if handle is available, otherwise show placeholder text
    if let Some(handle) = image_handle {
        row = row.push(image(handle).width(100).height(100));
    } else {
        row = row.push(
            image(get_asset_path("assets/icon.png"))
                .width(100)
                .height(100),
        );
    }

    row = row.push(
        container(column![
            text(truncate_title(&playlist.title))
                .shaping(text::Shaping::Advanced)
                .size(20),
            text(format!(
                "{} tracks",
                playlist.track_count.unwrap_or(0).format_compact_number()
            ))
            .size(20),
        ])
        // Never let the text grow taller than the artwork.
        .height(100)
        .clip(true),
    );

    mouse_area(container(row.spacing(10).padding(5))).on_press(load_playlist(playlist.clone()))
}
