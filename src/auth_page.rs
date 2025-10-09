use crate::soundcloud::auth;
use crate::{Message, Page, page_b::PageB};
use iced::Task;
use iced::widget::{button, column, text};

#[derive(Debug, Clone)]
pub enum AuthPageMessage {
    LoginPressed,
}

type Ma = AuthPageMessage;

pub struct AuthPage {
    jwt: String,
    token: String,
}

impl AuthPage {
    pub fn new() -> Self {
        Self {
            jwt: String::new(),
            token: String::new(),
        }
    }
}

impl Page for AuthPage {
    fn update(&mut self, message: Message) -> (Option<Box<dyn Page>>, Task<Message>) {
        if let Message::AuthPage(msg) = message {
            match msg {
                AuthPageMessage::LoginPressed => {
                    // Store the authentication state and handle it in the view
                    self.jwt = "authenticating".to_string();

                    // Run the authentication synchronously to avoid borrowing `self` across threads
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    match rt.block_on(auth::authenticate()) {
                        Ok(tm) => {
                            let access_token = tm.get_access_token();
                            self.token = access_token.secret().to_string();
                            self.jwt = "authenticated".to_string();
                            println!("Authentication successful!");
                            return (Some(Box::new(PageB::new(tm))), Task::none());
                        }
                        Err(e) => {
                            self.jwt = format!("error: {}", e);
                            eprintln!("Authentication failed: {}", e);
                        }
                    }

                    (None, Task::none())
                }
            }
        } else {
            (None, Task::none())
        }
    }

    fn view(&self) -> iced::Element<'_, Message> {
        let status = match self.jwt.as_str() {
            "" => "Not logged in".to_string(),
            "authenticating" => "Authenticating...".to_string(),
            "authenticated" => "Authenticated!".to_string(),
            s if s.starts_with("error: ") => s.to_string(),
            _ => "Unknown state".to_string(),
        };

        column![
            text(status).size(20),
            text(self.token.clone()).size(20),
            button("Log in").on_press(Message::AuthPage(Ma::LoginPressed))
        ]
        .padding(20)
        .spacing(10)
        .into()
    }
}
