use lazy_static::lazy_static;
use std::env;

lazy_static! {
    pub static ref CLIENT_ID: String = {
        dotenv::dotenv().ok();
        env::var("CLIENT_ID").expect("CLIENT_ID must be set in .env file")
    };
    
    pub static ref CLIENT_SECRET: String = {
        dotenv::dotenv().ok();
        env::var("CLIENT_SECRET").expect("CLIENT_SECRET must be set in .env file")
    };
    
    pub static ref REDIRECT_URL: String = {
        dotenv::dotenv().ok();
        env::var("REDIRECT_URL").unwrap_or_else(|_| "http://localhost:5000/".to_string())
    };
}

pub const SOUNDCLOUD_AUTH_URL: &str = "https://secure.soundcloud.com/authorize";
pub const SOUNDCLOUD_TOKEN_URL: &str = "https://secure.soundcloud.com/oauth/token";
