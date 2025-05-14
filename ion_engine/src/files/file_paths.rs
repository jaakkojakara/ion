use std::path::PathBuf;

pub fn user_dir() -> PathBuf {
    std::env::home_dir().expect("User home directory must exist")
}

pub fn game_data_dir(app_name: &str) -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        let storage_dir = PathBuf::from(format!("Library/Application Support/{}/", app_name));
        user_dir().join(storage_dir)
    }

    #[cfg(target_os = "windows")]
    {
        let storage_dir = PathBuf::from(format!("/AppData/Local/{}/", app_name));
        user_dir().join(storage_dir)
    }

    #[cfg(target_os = "linux")]
    {
        let storage_dir = PathBuf::from(format!("/.local/share/{}/", app_name));
        user_dir().join(storage_dir)
    }

    #[cfg(target_arch = "wasm32")]
    {
        panic!("WASM does not support direct file system access");
    }
}

pub fn log_dir(app_name: &str) -> PathBuf {
    let base = game_data_dir(app_name);
    let config_path = PathBuf::from("logs/");
    base.join(config_path)
}

pub fn config_dir(app_name: &str) -> PathBuf {
    let base = game_data_dir(app_name);
    let config_path = PathBuf::from("configs/");
    base.join(config_path)
}

pub fn cache_dir(app_name: &str) -> PathBuf {
    let base = game_data_dir(app_name);
    let cache_path = PathBuf::from("cache/");
    base.join(cache_path)
}

pub fn save_dir(app_name: &str, save_name: Option<&str>) -> PathBuf {
    let base = game_data_dir(app_name);
    let save_path = if let Some(save_name) = save_name {
        PathBuf::from(format!("saves/{}/", save_name))
    } else {
        PathBuf::from("saves/")
    };
    base.join(save_path)
}
