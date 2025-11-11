use lazy_static::lazy_static;
use std::env;
use std::path::PathBuf;

fn load_dotenv() {
    // Try to load .env from current directory first
    if dotenv::dotenv().is_ok() {
        return;
    }

    // If that fails, try to load from the executable's directory
    if let Ok(exe_path) = env::current_exe()
        && let Some(exe_dir) = exe_path.parent()
    {
        // For macOS app bundles: Contents/MacOS/exe -> Contents/Resources/.env
        #[cfg(target_os = "macos")]
        if let Some(contents_dir) = exe_dir.parent() {
            let resources_path = contents_dir.join("Resources").join(".env");
            if resources_path.exists() {
                dotenv::from_path(resources_path).ok();
                return;
            }
        }

        // For other platforms or non-bundled macOS builds
        let env_path = exe_dir.join(".env");
        if env_path.exists() {
            dotenv::from_path(env_path).ok();
            return;
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
        env::var("REDIRECT_URL").unwrap_or_else(|_| "http://localhost:32857/".to_string())
    };
}

pub const SOUNDCLOUD_AUTH_URL: &str = "https://secure.soundcloud.com/authorize";
pub const SOUNDCLOUD_TOKEN_URL: &str = "https://secure.soundcloud.com/oauth/token";
