use crate::Message;
use crate::models::SoundCloudUser;
use crate::utilities::{NumberFormat, get_asset_path, truncate_string};
use iced::widget::{MouseArea, Row, mouse_area, text};
use iced::widget::{column, container, image, image::Handle};

pub fn get_user_widget<F>(
    user: &'_ SoundCloudUser,
    image_handle: Option<Handle>,
    load_user: F,
) -> MouseArea<'_, Message>
where
    F: Fn(String) -> Message + 'static,
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

    row = row.push(column![
        text(truncate_string(user.username.clone(), 20))
            .shaping(text::Shaping::Advanced)
            .size(20),
        text(format!(
            "{} followers",
            user.followers_count.unwrap_or(0).format_compact_number()
        ))
        .size(20),
    ]);

    mouse_area(container(row.spacing(10).padding(5))).on_press(load_user(user.urn.clone()))
}
