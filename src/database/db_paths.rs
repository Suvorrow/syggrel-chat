use std::path::PathBuf;

/// Get the appropriate database path for the current platform
/// 
/// This function determines the correct location to store the application database
/// based on the operating system, following platform-specific conventions for
/// configuration file storage.
pub fn get_database_path() -> Result<PathBuf, std::io::Error> {
    let mut path = get_config_dir()?;
    path.push("syggrel-chat.db");
    Ok(path)
}

/// Get the platform-specific configuration directory
/// 
/// Returns the appropriate directory for storing application configuration
/// and data files according to each platform's conventions:
/// - Linux: ~/.config/syggrel-chat/
/// - Android: /data/data/[package]/files/
fn get_config_dir() -> Result<PathBuf, std::io::Error> {
    #[cfg(target_os = "android")]
    {
        // Primarily: use app cache directory (always accessible)
        let cache_dir = std::env::var("CACHE_DIR")
            .unwrap_or_else(|_| "/tmp".to_string());
        let mut path = PathBuf::from(cache_dir);
        path.push("syggrel-chat");

        // Try to create directory in cache first
        match std::fs::create_dir_all(&path) {
            Ok(_) => Ok(path),
            Err(primary_err) => {
                // Fallback: try external files directory if cache fails
                // Note: In a real Android app, should be used ndk to get proper paths
                let external_dir = std::env::var("EXTERNAL_STORAGE")
                    .unwrap_or_else(|_| "/sdcard".to_string());
                let mut fallback_path = PathBuf::from(external_dir);
                fallback_path.push("Android");
                fallback_path.push("data");
                fallback_path.push("syggrel-chat"); // Should be replaced with actual package name in real app
                fallback_path.push("files");

                // Try fallback directory
                match std::fs::create_dir_all(&fallback_path) {
                    Ok(_) => Ok(fallback_path),
                    Err(fallback_err) => {
                        // Return the error from the fallback attempt as it's more informative
                        Err(fallback_err)
                    }, 
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        // Linux: use XDG config directory
        let home = std::env::var("HOME").map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "HOME environment variable not set"
            )
        })?;
        let mut path = PathBuf::from(home);
        path.push(".config");
        path.push("syggrel-chat");
        std::fs::create_dir_all(&path)?;
        Ok(path)
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        // Fallback for other platforms
        let current_dir = std::env::current_dir()?;
        let mut path = current_dir;
        path.push(".syggrel-chat");
        std::fs::create_dir_all(&path)?;
        Ok(path)                                                                   
    }
}

/// Ensure the database directory exists and return the database path
/// 
/// This function creates the necessary directory structure for the database
/// and returns the full path to the database file.
pub fn ensure_database_path() -> Result<PathBuf, std::io::Error> {
    let db_path = get_database_path()?;

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    Ok(db_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_config_dir_exists() {
        let result = get_config_dir();
        assert!(result.is_ok());

        let path = result.unwrap();
        assert!(path.exists());
        assert!(path.is_dir());
    }

    #[test]
    fn test_database_path_extension() {
        let result = get_database_path();
        assert!(result.is_ok());

        let path = result.unwrap();
        assert_eq!(path.extension(), Some(std::ffi::OsStr::new("db")));
    }
}