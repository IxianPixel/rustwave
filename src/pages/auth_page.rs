use crate::pages::likes_page::LikesPage;
use crate::soundcloud::TokenManager;
use crate::soundcloud::auth;
use crate::widgets::spinner;
use crate::{Message, Page};
use iced::widget::{button, column, container, text};
use iced::{Alignment, Font, Length, Task};

#[derive(Debug, Clone)]
pub enum AuthPageMessage {
    LoginPressed,
    SessionRestored(Option<TokenManager>),
    AuthCompleted(Result<TokenManager, String>),
}

type Ma = AuthPageMessage;

enum AuthState {
    /// Trying to restore a session from a cached token on startup
    CheckingSession,
    /// No usable cached token; the user has to sign in
    SignedOut,
    /// Browser is open on the SoundCloud consent page
    WaitingForBrowser,
    Failed(String),
}

pub struct AuthPage {
    state: AuthState,
}

impl AuthPage {
    pub fn new() -> (Self, Task<Message>) {
        (
            Self {
                state: AuthState::CheckingSession,
            },
            Task::perform(auth::try_cached_authentication(), |result| {
                Message::AuthPage(Ma::SessionRestored(result))
            }),
        )
    }
}

impl Page for AuthPage {
    fn is_animating(&self) -> bool {
        // Keep frames flowing while a spinner is on screen.
        matches!(
            self.state,
            AuthState::CheckingSession | AuthState::WaitingForBrowser
        )
    }

    fn update(&mut self, message: Message) -> (Option<Box<dyn Page>>, Task<Message>) {
        let Message::AuthPage(msg) = message else {
            return (None, Task::none());
        };

        match msg {
            Ma::SessionRestored(Some(token_manager)) | Ma::AuthCompleted(Ok(token_manager)) => {
                let (page, task) = LikesPage::new(token_manager);
                (Some(Box::new(page)), task)
            }
            Ma::SessionRestored(None) => {
                self.state = AuthState::SignedOut;
                (None, Task::none())
            }
            Ma::LoginPressed => {
                if matches!(self.state, AuthState::WaitingForBrowser) {
                    return (None, Task::none());
                }
                self.state = AuthState::WaitingForBrowser;
                (
                    None,
                    Task::perform(auth::authenticate_in_browser(), |result| {
                        Message::AuthPage(Ma::AuthCompleted(result.map_err(|e| e.to_string())))
                    }),
                )
            }
            Ma::AuthCompleted(Err(error)) => {
                self.state = AuthState::Failed(error);
                (None, Task::none())
            }
        }
    }

    fn view(&self) -> iced::Element<'_, Message> {
        let bold = Font {
            weight: iced::font::Weight::Bold,
            ..Font::DEFAULT
        };

        let brand = column![
            text("Rustwave").size(42).font(bold),
            text("Stream your SoundCloud library")
                .size(15)
                .style(text::secondary),
        ]
        .spacing(4)
        .align_x(Alignment::Center);

        let status: iced::Element<'_, Message> = match &self.state {
            AuthState::CheckingSession => column![
                spinner(32.0),
                text("Restoring your session…")
                    .size(14)
                    .style(text::secondary),
            ]
            .spacing(12)
            .align_x(Alignment::Center)
            .into(),
            AuthState::SignedOut => column![
                button(text("Sign in with SoundCloud").size(16))
                    .padding([12, 24])
                    .on_press(Message::AuthPage(Ma::LoginPressed)),
                text("Your browser will open to authorize Rustwave.")
                    .size(13)
                    .style(text::secondary),
            ]
            .spacing(12)
            .align_x(Alignment::Center)
            .into(),
            AuthState::WaitingForBrowser => column![
                spinner(32.0),
                text("Waiting for authorization in your browser…").size(14),
                text("Approve access there and you'll be signed in automatically.")
                    .size(13)
                    .style(text::secondary),
            ]
            .spacing(12)
            .align_x(Alignment::Center)
            .into(),
            AuthState::Failed(error) => column![
                text("Sign-in failed")
                    .size(18)
                    .font(bold)
                    .style(text::danger),
                text(error.clone()).size(13).style(text::secondary),
                button(text("Try again").size(16))
                    .padding([12, 24])
                    .on_press(Message::AuthPage(Ma::LoginPressed)),
            ]
            .spacing(12)
            .align_x(Alignment::Center)
            .into(),
        };

        container(
            column![brand, status]
                .spacing(40)
                .align_x(Alignment::Center),
        )
        .center(Length::Fill)
        .into()
    }
}
