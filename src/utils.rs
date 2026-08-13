use std::path::PathBuf;

/// Returns the path to the ffmpeg executable.
/// Prioritizes the directory where the current executable is located,
/// falling back to "ffmpeg" (system PATH).
pub fn get_ffmpeg_path() -> PathBuf {
    find_binary("ffmpeg")
}

/// Returns the path to the ffprobe executable.
/// Prioritizes the directory where the current executable is located,
/// falling back to "ffprobe" (system PATH).
pub fn get_ffprobe_path() -> PathBuf {
    find_binary("ffprobe")
}

fn find_binary(name: &str) -> PathBuf {
    let binary_name = format!("{}{}", name, std::env::consts::EXE_SUFFIX);
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let local_path = exe_dir.join(&binary_name);
            if local_path.is_file() {
                return local_path;
            }
        }
    }
    PathBuf::from(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    #[test]
    fn test_find_binary_fallback() {
        let path = find_binary("non_existent_binary_xyz_123");
        assert_eq!(path, PathBuf::from("non_existent_binary_xyz_123"));
    }

    #[test]
    fn test_find_binary_local_priority() {
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let dummy_name = "test_dummy_binary_abc";
                let dummy_filename = format!("{}{}", dummy_name, std::env::consts::EXE_SUFFIX);
                let dummy_path = exe_dir.join(&dummy_filename);
                
                // Create a temporary dummy file in the executable directory
                let _ = File::create(&dummy_path);
                
                let found = find_binary(dummy_name);
                assert_eq!(found, dummy_path);
                
                // Cleanup
                let _ = std::fs::remove_file(dummy_path);
            }
        }
    }
}
