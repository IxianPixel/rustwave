use crate::models::SoundCloudPlaylist;
use crate::Message;
use iced::widget::{image, image::Handle, container, column};
use iced::widget::{mouse_area, text, MouseArea, Row};
use crate::utilities::{get_asset_path, NumberFormat};

pub fn get_playlist_widget<F>(playlist: &'_ SoundCloudPlaylist, image_handle: Option<Handle>, load_playlist: F) -> MouseArea<'_, Message>
where
    F: Fn(SoundCloudPlaylist) -> Message + 'static,
{
    let mut row = Row::new();

    // Add image if handle is available, otherwise show placeholder text
    if let Some(handle) = image_handle {
        row = row.push(image(handle).width(100).height(100));
    } else {
        row = row.push(image(get_asset_path("assets/icon.png")).width(100).height(100));
    }

    row = row.push(
        column![
            text(playlist.title.clone()).shaping(text::Shaping::Advanced).size(20),
            text(format!("{} tracks", playlist.track_count.unwrap_or(0).format_compact_number())).size(20),
        ]
    );

    mouse_area(
        container(row.spacing(10).padding(5))
    ).on_press(load_playlist(playlist.clone()))
}