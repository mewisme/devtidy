use super::{
  context::DevTidyContext,
  ollama::{ensure_model_available, select_model_by_ram, OllamaClient},
  utils::*,
};
use crate::core::constants::CLEANABLE_PATTERNS;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use std::io::{self, Write};
use std::time::Duration;

/// Handle the ai-explain command
pub async fn handle_ai_explain(path_arg: Option<String>) -> Result<()> {
  // Check Ollama availability first
  if !check_ollama_installation() {
    handle_ollama_not_available().await?;
    return Ok(());
  }

  let mut client = OllamaClient::new();

  // Automatically start Ollama if not running
  client.ensure_running().await?;

  // Show status info
  if client.has_spawned_process() {
    println!("Started Ollama daemon for this session");
  }

  // Resolve target path
  let target_path = resolve_target_path(path_arg)?;
  let folder_name = get_folder_display_name(&target_path);

  // Select appropriate model
  let model = select_model_by_ram();
  ensure_model_available(&client, &model).await?;

  // Create context-aware prompt
  let context = DevTidyContext::new();
  let prompt = context.create_explain_prompt(&folder_name);

  // Generate response with streaming
  println!("\nAI Explanation for '{}':", folder_name);
  print!("   ");

  match client.generate_with_format(&model, &prompt, false).await {
    Ok(_) => {
      // Response was printed via streaming
      println!("\n");
    }
    Err(e) => {
      println!("ERROR: {}\n", e);
    }
  }

  // Client will be dropped here, automatically cleaning up spawned processes
  Ok(())
}

/// Handle the ai-suggest command
pub async fn handle_ai_suggest() -> Result<()> {
  // Check Ollama availability first
  if !check_ollama_installation() {
    handle_ollama_not_available().await?;
    return Ok(());
  }

  let mut client = OllamaClient::new();

  // Automatically start Ollama if not running
  client.ensure_running().await?;

  // Show status info
  if client.has_spawned_process() {
    println!("Started Ollama daemon for this session");
  }

  // Get current directory
  let current_dir = std::env::current_dir()?;

  // Find known cleanable folders
  let mut found_folders = Vec::new();

  for (pattern, description) in CLEANABLE_PATTERNS.iter() {
    if !pattern.contains('*') {
      // Only check exact folder names
      let potential_path = current_dir.join(pattern);
      if potential_path.exists() && potential_path.is_dir() {
        let size = calculate_folder_size(&potential_path);
        if size > 0 {
          found_folders.push((pattern.to_string(), size, description.to_string()));
        }
      }
    }
  }

  if found_folders.is_empty() {
    println!("No known cleanable folders found in the current directory.");
    return Ok(());
  }

  // Select appropriate model
  let model = select_model_by_ram();
  ensure_model_available(&client, &model).await?;

  println!("\nAI Suggestions for cleanable folders:\n");

  // Process each folder with context
  let context = DevTidyContext::new();

  for (folder, size, _description) in found_folders {
    let size_str = format_size(size);
    let prompt = context.create_suggest_prompt(&folder, &size_str);

    println!("Folder: {} ({}):", folder, size_str);
    print!("   ");

    match client.generate_with_format(&model, &prompt, false).await {
      Ok(_) => {
        // Response was printed via streaming
        println!("\n");
      }
      Err(e) => {
        println!("ERROR: {}\n", e);
      }
    }
  }

  Ok(())
}

/// Test context-aware AI functionality
pub async fn handle_ai_test_context() -> Result<()> {
  println!("Testing DevTidy context-aware AI...\n");

  let context = DevTidyContext::new();

  // Test pattern matching
  let test_folders = vec![
    "node_modules",
    "target",
    "__pycache__",
    ".next",
    "dist",
    "src",
    "package.json",
    "unknown_folder",
  ];

  for folder in test_folders {
    println!("Testing: {}", folder);
    let prompt = context.create_explain_prompt(folder);

    // Show what the AI would be told (first 200 chars)
    let prompt_preview = if prompt.len() > 200 {
      format!("{}...", &prompt[..200])
    } else {
      prompt
    };

    println!("  Context preview: {}\n", prompt_preview);
  }

  println!("Context test complete!");
  Ok(())
}

/// Handle AI diagnostics command
pub async fn handle_ai_diagnose() -> Result<()> {
  println!("Running AI system diagnostics...\n");

  // Check Ollama installation
  if !check_ollama_installation() {
    println!("ISSUE: Ollama is not installed");
    println!("SOLUTION: Install Ollama from https://ollama.com/download");
    return Ok(());
  }
  println!("Ollama is installed");

  // Check Ollama service
  let mut client = OllamaClient::new();
  match client.detailed_health_check().await {
    Ok(status) => println!("{}", status),
    Err(e) => {
      println!("ISSUE: {}", e);
      println!("SOLUTION: Try running 'ollama serve' in another terminal");
      return Ok(());
    }
  }

  // Check system resources
  let selected_model = select_model_by_ram();
  println!("Selected model: {}", selected_model);

  // Test basic connectivity
  match client.ensure_running().await {
    Ok(_) => println!("Ollama daemon is responsive"),
    Err(e) => {
      println!("ISSUE: Failed to start/connect to Ollama: {}", e);
      println!("SOLUTION: Check if port 11434 is available and Ollama has permissions");
      return Ok(());
    }
  }

  println!("\nAll diagnostics passed! AI features should work properly.");
  Ok(())
}

/// Handle the ai-chat command (interactive REPL)
pub async fn handle_ai_chat() -> Result<()> {
  // Check Ollama availability first
  if !check_ollama_installation() {
    handle_ollama_not_available().await?;
    return Ok(());
  }

  let mut client = OllamaClient::new();

  // Automatically start Ollama if not running
  client.ensure_running().await?;

  // Show status info
  if client.has_spawned_process() {
    println!("Started Ollama daemon for this session");
  }

  // Select appropriate model
  let model = select_model_by_ram();
  ensure_model_available(&client, &model).await?;

  println!("\nAI Chat Mode - Ask questions about folders and cleaning!");
  println!("Model: {}", model);
  println!("Type 'exit' or 'quit' to end the session\n");

  // Initialize conversation context
  let mut context = DevTidyContext::new();

  loop {
    print!("Dev > ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();

    if input.is_empty() {
      continue;
    }

    if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
      println!("Goodbye!");
      if client.has_spawned_process() {
        println!("Cleaning up Ollama daemon...");
      }
      break;
    }

    // Create context-aware prompt
    let prompt = context.create_chat_prompt(input);

    // Generate response with streaming
    print!("AI > ");
    match client.generate_with_format(&model, &prompt, false).await {
      Ok(response) => {
        // Add to conversation history
        context.add_exchange(input.to_string(), response.clone());
        println!("\n");
      }
      Err(e) => {
        println!("\nERROR: Error generating response: {}\n", e);
      }
    }
  }

  Ok(())
}

/// Handle case where Ollama is not installed
async fn handle_ollama_not_available() -> Result<()> {
  println!("ERROR: Ollama is not installed or not running.");
  print!("Do you want to install it now? (Y/n): ");

  // Try to get a single key press without requiring Enter
  let input = get_single_key_input().unwrap_or_else(|| {
    // Fallback to line input if single key doesn't work
    print!("Please type y or n and press Enter: ");
    io::stdout().flush().ok();
    let mut line = String::new();
    io::stdin().read_line(&mut line).ok();
    line.trim().to_lowercase()
  });

  if input.is_empty() || input == "y" || input == "yes" {
    println!("Opening Ollama download page...");
    if let Err(e) = open_url("https://ollama.com/download") {
      println!("ERROR: Failed to open browser: {}", e);
      println!("Please visit: https://ollama.com/download");
    }
    println!("After installing Ollama, run: ollama serve");
    println!("Then try your command again.");
  } else {
    println!("AI features require Ollama to be installed and running.");
    println!("Visit: https://ollama.com/download");
  }

  Ok(())
}

/// Get a single key press without requiring Enter using crossterm
fn get_single_key_input() -> Option<String> {
  use std::thread;

  // Give user time to read the prompt
  thread::sleep(Duration::from_millis(200));

  // Clear any existing input buffer first
  while let Ok(true) = event::poll(Duration::from_millis(1)) {
    let _ = event::read(); // Consume and discard buffered events
  }

  // Enable raw mode to capture single key presses
  if crossterm::terminal::enable_raw_mode().is_err() {
    return None;
  }

  // Give user visual indication that we're waiting for input
  io::stdout().flush().ok();

  let result = loop {
    // Poll for events with a longer timeout
    if let Ok(true) = event::poll(Duration::from_secs(30)) {
      if let Ok(event) = event::read() {
        match event {
          Event::Key(key_event) => {
            // Only respond to actual key press events, not releases
            if key_event.kind == KeyEventKind::Press {
              let response = match key_event.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => Some("y".to_string()),
                KeyCode::Char('n') | KeyCode::Char('N') => Some("n".to_string()),
                KeyCode::Enter => Some("y".to_string()), // Default to yes on Enter
                KeyCode::Esc => Some("n".to_string()),   // Escape means no
                _ => continue,                           // Ignore other keys
              };
              break response;
            }
          }
          _ => continue, // Ignore other events
        }
      }
    } else {
      // Timeout - default to no
      break Some("n".to_string());
    }
  };

  // Always restore terminal mode
  let _ = crossterm::terminal::disable_raw_mode();

  // Print the choice so user knows what was selected
  if let Some(ref choice) = result {
    println!("{}", choice.to_uppercase());
  }

  result
}
