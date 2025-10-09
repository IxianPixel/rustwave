use crate::Message;
use iced::widget::canvas::{Frame, Geometry, Path, Program};
use iced::widget::{canvas};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Size, Theme};

struct WaveformCanvas {
    peaks: Vec<f32>,
    progress: f32,
}

impl WaveformCanvas {
    fn new(peaks: Vec<f32>, progress: f32) -> Self {
        Self { peaks, progress }
    }
}

impl Program<Message> for WaveformCanvas {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<Geometry> {
        // Don't use cache since progress changes every frame
        // Draw directly for better performance
        let mut frame = Frame::new(renderer, bounds.size());

        let width = bounds.width;
        let height = bounds.height;

        if !self.peaks.is_empty() {
            let bar_width = width / self.peaks.len() as f32;
            let progress_x = width * self.progress;

            for (i, &peak) in self.peaks.iter().enumerate() {
                let x = i as f32 * bar_width;
                let bar_height = peak * height * 0.8; // 80% of height for padding
                let y_start = (height - bar_height) / 2.0;

                let color = if x < progress_x {
                    Color::from_rgb(0.34, 0.59, 0.97) // Blue
                } else {
                    Color::from_rgb(0.4, 0.42, 0.49) // Grey
                };

                let path = Path::rectangle(
                    Point::new(x, y_start),
                    Size::new(bar_width.max(1.0), bar_height),
                );

                frame.fill(&path, color);
            }
        }

        vec![frame.into_geometry()]
    }

    fn update(
        &self,
        _state: &mut Self::State,
        event: canvas::Event,
        bounds: Rectangle,
        cursor: iced::mouse::Cursor,
    ) -> (canvas::event::Status, Option<Message>) {
        use canvas::event::Status;

        match event {
            canvas::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)) => {
                if let Some(position) = cursor.position_in(bounds) {
                    // Calculate seek position as percentage
                    let percent = (position.x / bounds.width * 100.0).clamp(0.0, 100.0);
                    return (Status::Captured, Some(Message::SeekToPosition(percent)));
                }
            }
            _ => {}
        }

        (Status::Ignored, None)
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: iced::mouse::Cursor,
    ) -> iced::mouse::Interaction {
        if cursor.is_over(bounds) {
            iced::mouse::Interaction::Pointer
        } else {
            iced::mouse::Interaction::default()
        }
    }
}

/// Creates an interactive waveform widget that displays track progress and allows seeking
///
/// # Arguments
/// * `waveform_peaks` - Optional peak data extracted from waveform
/// * `progress` - Current playback progress (0.0 to 1.0)
///
/// # Returns
/// A canvas widget that emits SeekToPosition messages when clicked
pub fn get_waveform_widget(
    waveform_peaks: Option<Vec<f32>>,
    progress: f32,
) -> Element<'static, Message> {
    // Use real peak data if available, otherwise use dummy data
    let peaks = waveform_peaks.unwrap_or_else(|| {
        // Fallback to dummy sine wave if no peaks available
        (0..200)
            .map(|i| ((i as f32 / 10.0).sin().abs() + 0.2).min(1.0))
            .collect()
    });

    let waveform_canvas = WaveformCanvas::new(peaks, progress);
    canvas(waveform_canvas)
        .width(Length::Fill)
        .height(100)
        .into()
}
