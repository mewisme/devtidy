mod app;
mod cleaner;
mod constants;
mod models;
mod scanner;
mod ui;

use anyhow::Result;
use clap::Parser;
use crossterm::{
  event, execute,
  terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;

/// DevTidy - Clean development artifacts from your projects
#[derive(Parser, Debug)]
#[clap(version = constants::VERSION, about, long_about = None)]
#[clap(disable_version_flag = true)]
struct Args {
  /// Target directory to scan (defaults to current directory)
  #[clap(value_parser)]
  directory: Option<String>,

  /// Scan files matching .gitignore patterns
  #[clap(long)]
  gitignore: bool,

  /// Maximum depth for directory scanning (default: 6)
  #[clap(short, long, default_value = "6")]
  depth: usize,

  /// Show version information
  #[clap(short, long = "version")]
  version: bool,

  /// Install devtidy globally
  #[clap(short, long)]
  install: bool,
}

async fn run() -> Result<()> {
  use std::env;
  use std::fs;
  use std::io::Write;

  let args = Args::parse();

  if args.version {
    println!("devtidy {}", constants::VERSION);
    println!(
      "Built with Rust {} ({}/{})",
      rustc_version_runtime::version(),
      std::env::consts::OS,
      std::env::consts::ARCH
    );
    return Ok(());
  }

  if args.install {
    let home_dir =
      dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?;
    let install_dir = home_dir.join(".devtidy");

    fs::create_dir_all(&install_dir)?;

    let current_exe = env::current_exe()?;
    let exe_name = current_exe
      .file_name()
      .ok_or_else(|| anyhow::anyhow!("Executable has no filename"))?;
    let target_path = install_dir.join(exe_name);
    fs::copy(&current_exe, &target_path)?;

    println!("Installed to: {}", target_path.display());

    if cfg!(target_os = "windows") {
      use std::process::Command;

      let output = Command::new("powershell")
        .args([
          "-NoProfile",
          "-Command",
          "[System.Environment]::GetEnvironmentVariable('Path', 'User')",
        ])
        .output()?;

      let mut path_user = String::from_utf8_lossy(&output.stdout).to_string();

      if !path_user
        .split(';')
        .any(|p| p.trim_end_matches('\\') == install_dir.to_string_lossy())
      {
        path_user.push(';');
        path_user.push_str(&install_dir.to_string_lossy());

        Command::new("powershell")
          .args([
            "-NoProfile",
            "-Command",
            &format!(
              "[System.Environment]::SetEnvironmentVariable('Path', '{}', 'User')",
              path_user
            ),
          ])
          .status()?;

        println!("Added to user PATH: {}", install_dir.display());
      } else {
        println!("Already in PATH: {}", install_dir.display());
      }
    } else {
      let shell = env::var("SHELL").unwrap_or_default();
      let profile_file = if shell.contains("zsh") {
        home_dir.join(".zshrc")
      } else {
        home_dir.join(".bashrc")
      };

      let path_env = env::var("PATH").unwrap_or_default();
      if !path_env
        .split(':')
        .any(|p| p == install_dir.to_string_lossy())
      {
        let export_line = format!("export PATH=\"$HOME/.devtidy:$PATH\"\n");

        let mut needs_append = true;
        if let Ok(existing) = fs::read_to_string(&profile_file) {
          if existing.contains(&export_line.trim()) {
            needs_append = false;
          }
        }

        if needs_append {
          let mut file = fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&profile_file)?;
          file.write_all(export_line.as_bytes())?;

          println!("Added PATH export to: {}", profile_file.display());
          println!("Run this command to apply immediately:");
          println!("  source {}", profile_file.display());
        } else {
          println!("PATH export already exists in: {}", profile_file.display());
        }
      } else {
        println!("Already in PATH: {}", install_dir.display());
      }
    }

    return Ok(());
  }

  let mut app = match app::initialize_app(args.directory, args.gitignore, args.depth) {
    Ok(app) => app,
    Err(err) => {
      eprintln!("Error: {}", err);
      return Err(err);
    }
  };

  enable_raw_mode()?;
  let mut stdout = io::stdout();
  execute!(stdout, EnterAlternateScreen, event::EnableMouseCapture)?;
  let backend = CrosstermBackend::new(stdout);
  let mut terminal = Terminal::new(backend)?;
  terminal.clear()?;

  let app_result = app::run_app(&mut app, &mut terminal).await;

  disable_raw_mode()?;
  execute!(
    terminal.backend_mut(),
    LeaveAlternateScreen,
    event::DisableMouseCapture
  )?;
  terminal.show_cursor()?;

  if let Err(err) = app_result {
    eprintln!("Error: {}", err);
    return Err(err);
  }

  Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
  env_logger::init();

  if let Err(err) = run().await {
    eprintln!("Error: {}", err);
    std::process::exit(1);
  }

  Ok(())
}
