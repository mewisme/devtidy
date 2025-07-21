mod ai;
mod core;
mod services;
mod ui;

use anyhow::Result;
use clap::{Parser, Subcommand};
use crossterm::{
  event, execute,
  terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;

/// DevTidy - Clean development artifacts from your projects
#[derive(Parser, Debug)]
#[clap(version = core::constants::VERSION, about, long_about = None)]
#[clap(disable_version_flag = true)]
struct Args {
  #[clap(subcommand)]
  command: Option<Commands>,

  /// Target directory to scan (defaults to current working directory)
  #[clap(short = 'p', long, value_parser, global = true)]
  path: Option<String>,

  /// Scan files matching .gitignore patterns
  #[clap(long, global = true)]
  gitignore: bool,

  /// Maximum depth for directory scanning (default: 6)
  #[clap(short, long, default_value = "6", global = true)]
  depth: usize,

  /// Show version information
  #[clap(short, long = "version")]
  version: bool,

  /// Install devtidy globally
  #[clap(short, long)]
  install: bool,

  /// Show help information
  #[clap(short, long)]
  help: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
  /// Explain what a folder is used for using AI
  AiExplain {
    /// Path to the folder to explain (defaults to current directory)
    path: Option<String>,
  },
  /// Get AI suggestions for cleaning the current project
  AiSuggest,
  /// Start an interactive AI chat for cleaning advice
  AiChat,
  /// Run AI system diagnostics
  AiDiagnose,
  /// Test AI context functionality (debug)
  AiTestContext,
}

async fn run() -> Result<()> {
  use std::env;
  use std::fs;
  use std::io::Write;

  let args = Args::parse();

  if args.version {
    println!("DevTidy v{}", core::constants::VERSION);
    println!(
      "Built with Rust {} ({}/{})",
      rustc_version_runtime::version(),
      std::env::consts::OS,
      std::env::consts::ARCH
    );
    return Ok(());
  }

  if args.help {
    println!("DevTidy - Clean development artifacts from your projects");
    println!();
    println!("USAGE:");
    println!("  dd [OPTIONS] [COMMAND]");
    println!();
    println!("OPTIONS:");
    println!("  -p, --path <PATH>          Target directory to scan (defaults to current working directory)");
    println!("  --gitignore                Scan files matching .gitignore patterns");
    println!("  -d, --depth <DEPTH>        Maximum depth for directory scanning (default: 6)");
    println!("  -v, --version              Show version information");
    println!("  -i, --install              Install devtidy globally");
    println!("  -h, --help                 Show help information");
    println!();
    println!("COMMANDS:");
    println!("  ai-explain <PATH>          Explain what a folder is used for using AI");
    println!("  ai-suggest                 Get AI suggestions for cleaning the current project");
    println!("  ai-chat                    Start an interactive AI chat for cleaning advice");
    println!();
    println!("EXAMPLES:");
    println!("  dd                         Scan current directory");
    println!("  dd -p /path/to/project     Scan specific directory");
    println!("  dd --gitignore             Scan with .gitignore patterns");
    println!("  dd ai-explain              Explain current directory with AI");
    println!("  dd ai-suggest              Get AI cleaning suggestions");
    return Ok(());
  }

  // Handle AI subcommands
  if let Some(command) = args.command {
    match command {
      Commands::AiExplain { path } => {
        return ai::handle_ai_explain(path).await;
      }
      Commands::AiSuggest => {
        return ai::handle_ai_suggest().await;
      }
      Commands::AiChat => {
        return ai::handle_ai_chat().await;
      }
      Commands::AiDiagnose => {
        return ai::handle_ai_diagnose().await;
      }
      Commands::AiTestContext => {
        return ai::handle_ai_test_context().await;
      }
    }
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

    // Remove existing installation if it exists
    if target_path.exists() {
      fs::remove_file(&target_path)?;
      println!("Removed existing installation: {}", target_path.display());
    }

    // Copy new version
    fs::copy(&current_exe, &target_path)?;

    // Make executable on Unix-like systems
    #[cfg(unix)]
    {
      use std::os::unix::fs::PermissionsExt;
      let mut perms = fs::metadata(&target_path)?.permissions();
      perms.set_mode(0o755);
      fs::set_permissions(&target_path, perms)?;
    }

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

  let mut app = match core::app::initialize_app(args.path, args.gitignore, args.depth) {
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

  let app_result = core::app::run_app(&mut app, &mut terminal).await;

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
