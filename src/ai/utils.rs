use anyhow::Result;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Get the display name of a folder for AI prompts
pub fn get_folder_display_name(path: &Path) -> String {
  path
    .file_name()
    .and_then(|name| name.to_str())
    .unwrap_or("unknown")
    .to_string()
}

/// Calculate directory size
pub fn calculate_folder_size(path: &Path) -> u64 {
  WalkDir::new(path)
    .into_iter()
    .filter_map(|entry| entry.ok())
    .filter_map(|entry| {
      if entry.file_type().is_file() {
        entry.metadata().ok().map(|metadata| metadata.len())
      } else {
        None
      }
    })
    .sum()
}

/// Format bytes into human readable format
pub fn format_size(bytes: u64) -> String {
  human_bytes::human_bytes(bytes as f64)
}

/// Resolve path argument, defaulting to current directory if None
pub fn resolve_target_path(path_arg: Option<String>) -> Result<PathBuf> {
  match path_arg {
    Some(path) => {
      let path = PathBuf::from(path);
      if !path.exists() {
        anyhow::bail!("Directory does not exist: {}", path.display());
      }
      if !path.is_dir() {
        anyhow::bail!("Not a directory: {}", path.display());
      }
      Ok(path.canonicalize()?)
    }
    None => Ok(std::env::current_dir()?),
  }
}

/// Check if Ollama installation exists on the system
pub fn check_ollama_installation() -> bool {
  use std::process::Command;

  // Try to run "ollama --version" to check if it's installed
  match Command::new("ollama").arg("--version").output() {
    Ok(output) => output.status.success(),
    Err(_) => false,
  }
}

/// Open URL in the default browser
pub fn open_url(url: &str) -> Result<()> {
  use std::process::Command;

  #[cfg(target_os = "windows")]
  {
    Command::new("cmd").args(["/c", "start", url]).spawn()?;
  }

  #[cfg(target_os = "macos")]
  {
    Command::new("open").arg(url).spawn()?;
  }

  #[cfg(target_os = "linux")]
  {
    Command::new("xdg-open").arg(url).spawn()?;
  }

  Ok(())
}
