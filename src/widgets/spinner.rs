use crate::Message;
use iced::mouse;
use iced::widget::canvas::{self, Canvas, Geometry, Path, Stroke};
use iced::{Element, Radians, Rectangle, Renderer, Theme};
use std::f32::consts::TAU;
use std::time::{SystemTime, UNIX_EPOCH};

// One full revolution of the spinner arc.
const ROTATION_PERIOD_MS: u128 = 900;
// How much of the circle the moving arc covers.
const ARC_SWEEP: f32 = 0.25 * TAU;

struct Spinner;

impl canvas::Program<Message> for Spinner {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let center = frame.center();
        let stroke_width = (bounds.width.min(bounds.height) / 10.0).max(2.0);
        let radius = bounds.width.min(bounds.height) / 2.0 - stroke_width;

        let palette = theme.extended_palette();

        // Faint full-circle track behind the moving arc.
        frame.stroke(
            &Path::circle(center, radius),
            Stroke::default()
                .with_width(stroke_width)
                .with_color(palette.background.strong.color),
        );

        // The rotation phase comes from wall-clock time, so the widget is
        // stateless; continuous redraws while visible are driven by the
        // page's Page::is_animating() implementation.
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            % ROTATION_PERIOD_MS;
        let start_angle = millis as f32 / ROTATION_PERIOD_MS as f32 * TAU;

        let arc = Path::new(|builder| {
            builder.arc(canvas::path::Arc {
                center,
                radius,
                start_angle: Radians(start_angle),
                end_angle: Radians(start_angle + ARC_SWEEP),
            });
        });
        frame.stroke(
            &arc,
            Stroke::default()
                .with_width(stroke_width)
                .with_color(palette.primary.strong.color)
                .with_line_cap(canvas::LineCap::Round),
        );

        vec![frame.into_geometry()]
    }
}

/// An indeterminate loading spinner: a rotating accent-coloured arc over a
/// faint circular track. `size` is the widget's width and height in pixels.
pub fn spinner<'a>(size: f32) -> Element<'a, Message> {
    Canvas::new(Spinner).width(size).height(size).into()
}
