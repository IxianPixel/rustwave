use crate::models::SoundCloudUser;
use crate::Message;
use iced::widget::{image, image::Handle, container, column};
use iced::widget::{mouse_area, text, MouseArea, Row};
use crate::utilities::NumberFormat;

pub fn get_user_widget<F>(user: &'_ SoundCloudUser, image_handle: Option<Handle>, load_user: F) -> MouseArea<'_, Message>
where
    F: Fn(String) -> Message + 'static,
{
    let mut row = Row::new();

    // Add image if handle is available, otherwise show placeholder text
    if let Some(handle) = image_handle {
        row = row.push(image(handle).width(100).height(100));
    } else {
        row = row.push(text("Loading image..."));
    }

    row = row.push(
        column![
            text(user.username.clone()).shaping(text::Shaping::Advanced).size(20),
            text(format!("{} followers", user.followers_count.unwrap_or(0).format_compact_number())).size(20),
        ]
    );

    mouse_area(
        container(row.spacing(10).padding(5))
    ).on_press(load_user(user.urn.clone()))
}