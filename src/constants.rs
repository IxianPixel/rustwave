use lazy_static::lazy_static;
use std::env;
use std::path::PathBuf;

fn load_dotenv() {
    // Try to load .env from current directory first
    if dotenv::dotenv().is_ok() {
        return;
    }
    
    // If that fails, try to load from the executable's directory (for app bundles)
    if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let env_path = exe_dir.join(".env");
            if env_path.exists() {
                dotenv::from_path(env_path).ok();
                return;
            }
        }
    }
    
    // As a last resort, try common locations
    let possible_paths = vec![
        PathBuf::from(".env"),
        PathBuf::from("../.env"),
        PathBuf::from("../../.env"),
    ];
    
    for path in possible_paths {
        if path.exists() {
            dotenv::from_path(path).ok();
            break;
        }
    }
}

lazy_static! {
    pub static ref CLIENT_ID: String = {
        load_dotenv();
        env::var("CLIENT_ID").expect("CLIENT_ID must be set in .env file")
    };
    
    pub static ref CLIENT_SECRET: String = {
        load_dotenv();
        env::var("CLIENT_SECRET").expect("CLIENT_SECRET must be set in .env file")
    };
    
    pub static ref REDIRECT_URL: String = {
        load_dotenv();
        env::var("REDIRECT_URL").unwrap_or_else(|_| "http://localhost:5000/".to_string())
    };
}

pub const SOUNDCLOUD_AUTH_URL: &str = "https://secure.soundcloud.com/authorize";
pub const SOUNDCLOUD_TOKEN_URL: &str = "https://secure.soundcloud.com/oauth/token";
