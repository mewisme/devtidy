use anyhow::{anyhow, Result};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;
use sysinfo::System;

/// Create an animated spinner with consistent styling
fn create_spinner(message: &str, color: &str) -> ProgressBar {
  let spinner = ProgressBar::new_spinner();
  spinner.set_style(
    ProgressStyle::default_spinner()
      .template(&format!("{{spinner:.{}}} {{msg}}", color))
      .unwrap(),
  );
  spinner.set_message(message.to_string());
  spinner.enable_steady_tick(Duration::from_millis(100));
  spinner
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaResponse {
  pub response: String,
  pub done: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaModel {
  pub name: String,
  pub size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaModelsResponse {
  pub models: Vec<OllamaModel>,
}

pub struct OllamaClient {
  client: Client,
  base_url: String,
  spawned_process: Option<std::process::Child>,
}

impl OllamaClient {
  pub fn new() -> Self {
    Self {
      client: Client::new(),
      base_url: "http://localhost:11434".to_string(),
      spawned_process: None,
    }
  }

  /// Check if Ollama is running and accessible
  pub async fn health_check(&self) -> Result<bool> {
    let response = self
      .client
      .get(&format!("{}/api/tags", self.base_url))
      .timeout(Duration::from_secs(5))
      .send()
      .await;

    match response {
      Ok(resp) => Ok(resp.status().is_success()),
      Err(_) => Ok(false),
    }
  }

  /// Extended health check with detailed diagnostics
  pub async fn detailed_health_check(&self) -> Result<String> {
    let spinner = create_spinner("Checking Ollama status...", "yellow");

    // Check if Ollama is responding
    match self.health_check().await {
      Ok(true) => {
        // Check available models
        match self.list_models().await {
          Ok(models) => {
            spinner.finish_with_message("Ollama is running");
            if models.is_empty() {
              Ok("Ollama is running but no models are installed.".to_string())
            } else {
              Ok(format!(
                "Ollama is running with {} models: {}",
                models.len(),
                models.join(", ")
              ))
            }
          }
          Err(e) => {
            spinner.finish_with_message("Ollama responding but API issues");
            Ok(format!("Ollama is running but API returned error: {}", e))
          }
        }
      }
      Ok(false) => {
        spinner.finish_with_message("ERROR: Ollama not responding");
        Ok("Ollama is not responding. Please check if 'ollama serve' is running.".to_string())
      }
      Err(e) => {
        spinner.finish_with_message("ERROR: Health check failed");
        Err(anyhow!("Health check failed: {}", e))
      }
    }
  }

  /// Start Ollama daemon in the background if not already running
  pub async fn ensure_running(&mut self) -> Result<()> {
    // First check if Ollama is already running
    if self.health_check().await? {
      return Ok(());
    }

    // Check if Ollama is installed
    if !crate::ai::utils::check_ollama_installation() {
      return Err(anyhow!("Ollama is not installed"));
    }

    // Start Ollama in the background
    let child = std::process::Command::new("ollama")
      .arg("serve")
      .stdout(std::process::Stdio::null())
      .stderr(std::process::Stdio::null())
      .spawn()?;

    self.spawned_process = Some(child);

    // Wait for Ollama to become available (up to 30 seconds)
    let spinner = create_spinner("Waiting for Ollama to start...", "green");

    for _ in 0..60 {
      // Try for 30 seconds (60 * 0.5s intervals)
      tokio::time::sleep(Duration::from_millis(500)).await;
      if self.health_check().await? {
        spinner.finish_and_clear();
        return Ok(());
      }
    }

    spinner.finish_with_message("Ollama failed to start");
    Err(anyhow!("Ollama failed to start within timeout"))
  }

  /// Stop the Ollama daemon if it was started by this client
  pub fn stop_if_spawned(&mut self) -> Result<()> {
    if let Some(mut child) = self.spawned_process.take() {
      let spinner = create_spinner("Stopping Ollama daemon...", "red");
      match child.kill() {
        Ok(_) => {
          // Wait for the process to actually terminate
          let _ = child.wait();
          spinner.finish_and_clear();
          println!("Ollama daemon stopped");
        }
        Err(e) => {
          spinner.finish_and_clear();
          eprintln!("Failed to stop Ollama daemon: {}", e);
        }
      }
    }
    Ok(())
  }

  /// Check if this client spawned an Ollama process
  pub fn has_spawned_process(&self) -> bool {
    self.spawned_process.is_some()
  }

  /// List available models
  pub async fn list_models(&self) -> Result<Vec<String>> {
    let response = self
      .client
      .get(&format!("{}/api/tags", self.base_url))
      .send()
      .await?;

    if !response.status().is_success() {
      return Err(anyhow!("Failed to list models"));
    }

    let models_response: OllamaModelsResponse = response.json().await?;
    Ok(models_response.models.into_iter().map(|m| m.name).collect())
  }

  /// Pull a model if not available locally
  pub async fn pull_model(&self, model: &str) -> Result<()> {
    use futures_util::StreamExt;

    println!("Pulling model '{}'...\nThis may take a few minutes.", model);

    let spinner = create_spinner(&format!("Downloading {}", model), "green");

    let response = self
      .client
      .post(&format!("{}/api/pull", self.base_url))
      .json(&json!({ "name": model }))
      .send()
      .await?;

    if !response.status().is_success() {
      spinner.finish_with_message("ERROR: Failed to pull model");
      return Err(anyhow!("Failed to pull model: {}", model));
    }

    // Handle streaming response for real-time progress
    let mut stream = response.bytes_stream();
    let mut buffer = Vec::new();

    while let Some(chunk) = stream.next().await {
      let chunk = chunk?;
      buffer.extend_from_slice(&chunk);

      // Process complete lines
      while let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
        let line_bytes = buffer.drain(..=newline_pos).collect::<Vec<u8>>();
        let line = String::from_utf8_lossy(&line_bytes[..line_bytes.len() - 1]); // Remove newline

        if let Ok(status) = serde_json::from_str::<serde_json::Value>(&line) {
          if let Some(status_msg) = status.get("status").and_then(|s| s.as_str()) {
            spinner.set_message(format!("{}: {}", model, status_msg));
          }

          // Check if download is complete
          if status.get("status").and_then(|s| s.as_str()) == Some("success") {
            break;
          }
        }
      }
    }

    spinner.finish_with_message(format!("Model '{}' ready", model));
    Ok(())
  }

  /// Generate text with optional formatting
  pub async fn generate_with_format(
    &self,
    model: &str,
    prompt: &str,
    show_header: bool,
  ) -> Result<String> {
    // Retry logic for timeout errors
    for attempt in 1..=2 {
      match self
        .try_generate_with_format(model, prompt, show_header, attempt)
        .await
      {
        Ok(result) => return Ok(result),
        Err(e) => {
          if attempt == 2 || !e.to_string().contains("timeout") {
            return Err(e);
          }
          println!("   Retrying... (attempt {}/2)", attempt + 1);
        }
      }
    }

    unreachable!()
  }

  /// Internal implementation with retry support
  async fn try_generate_with_format(
    &self,
    model: &str,
    prompt: &str,
    show_header: bool,
    attempt: u32,
  ) -> Result<String> {
    use futures_util::StreamExt;
    use std::io::{self, Write};

    // Initial spinner with attempt info
    let spinner_msg = if attempt == 1 {
      format!("Connecting to AI ({})", model)
    } else {
      format!("Retrying AI connection ({}) - attempt {}", model, attempt)
    };
    let spinner = create_spinner(&spinner_msg, "cyan");

    // Model-specific token limits for complete responses
    let max_tokens = match model {
      "tinyllama" => 1000,        // Small model, moderate limit
      "phi" => 1500,              // Balanced
      "gemma:2b" => 2000,         // Good capacity
      "gemma:7b" => 2500,         // Larger capacity
      "mistral:instruct" => 3000, // High capacity
      _ => 1500,                  // Safe default
    };

    let payload = json!({
        "model": model,
        "prompt": prompt,
        "stream": true,  // Enable streaming
        "options": {
            "temperature": 0.1,
            "top_p": 0.9,
            "num_predict": max_tokens
        }
    });

    // Model-specific timeouts for initial loading
    let timeout_secs = match model {
      "tinyllama" => 30,
      "phi" => 45,
      "gemma:2b" => 60,
      "mistral:instruct" => 120,
      _ => 60,
    };

    // Start the streaming request with model-appropriate timeout
    let response = self
      .client
      .post(&format!("{}/api/generate", self.base_url))
      .json(&payload)
      .timeout(Duration::from_secs(timeout_secs))
      .send()
      .await?;

    if !response.status().is_success() {
      spinner.finish_and_clear();
      println!("Failed to connect to AI");
      let status = response.status();
      let error_text = response.text().await.unwrap_or_default();
      return Err(anyhow!(
        "Ollama API error ({}): {}",
        status,
        if error_text.is_empty() {
          "Unknown error".to_string()
        } else {
          error_text
        }
      ));
    }

    // Stop spinner and start streaming output
    spinner.finish_and_clear();

    if show_header {
      print!("AI > ");
    }
    io::stdout().flush().ok();

    // Handle streaming response
    let mut stream = response.bytes_stream();
    let mut buffer = Vec::new();
    let mut full_response = String::new();
    let mut words_on_line = 0;
    const MAX_WORDS_PER_LINE: usize = 12;

    while let Some(chunk_result) = stream.next().await {
      let chunk = match chunk_result {
        Ok(chunk) => chunk,
        Err(e) => {
          eprintln!("\nERROR: Streaming interrupted: {}", e);
          break;
        }
      };

      buffer.extend_from_slice(&chunk);

      // Process complete lines
      while let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
        let line_bytes = buffer.drain(..=newline_pos).collect::<Vec<u8>>();
        let line = String::from_utf8_lossy(&line_bytes[..line_bytes.len() - 1]); // Remove newline

        // Skip empty lines
        if line.trim().is_empty() {
          continue;
        }

        match serde_json::from_str::<serde_json::Value>(&line) {
          Ok(chunk_data) => {
            // Check for error in response
            if let Some(error) = chunk_data.get("error") {
              eprintln!("\nERROR: AI model error: {}", error);
              break;
            }

            if let Some(response_text) = chunk_data.get("response").and_then(|r| r.as_str()) {
              if !response_text.is_empty() {
                // Print the streaming text
                print!("{}", response_text);
                io::stdout().flush().ok();

                full_response.push_str(response_text);

                // Add line breaks for readability (only if showing header)
                if show_header && response_text.contains(' ') {
                  words_on_line += response_text.split_whitespace().count();
                  if words_on_line >= MAX_WORDS_PER_LINE {
                    println!();
                    print!("> ");
                    io::stdout().flush().ok();
                    words_on_line = 0;
                  }
                }
              }
            }

            // Check if streaming is done
            if chunk_data
              .get("done")
              .and_then(|d| d.as_bool())
              .unwrap_or(false)
            {
              break;
            }
          }
          Err(e) => {
            // Skip malformed JSON lines (common in streaming responses)
            eprintln!("\nDEBUG: Skipping malformed JSON: {} (line: {})", e, line);
            continue;
          }
        }
      }
    }

    // Process any remaining buffer data
    if !buffer.is_empty() {
      let remaining = String::from_utf8_lossy(&buffer);
      if !remaining.trim().is_empty() {
        if let Ok(chunk_data) = serde_json::from_str::<serde_json::Value>(&remaining) {
          if let Some(response_text) = chunk_data.get("response").and_then(|r| r.as_str()) {
            if !response_text.is_empty() {
              print!("{}", response_text);
              full_response.push_str(response_text);
            }
          }
        }
      }
    }

    if full_response.trim().is_empty() {
      return Err(anyhow!("No response received from AI"));
    }

    Ok(full_response.trim().to_string())
  }
}

impl Drop for OllamaClient {
  fn drop(&mut self) {
    let _ = self.stop_if_spawned();
  }
}

/// Hardware capabilities detected from the system
#[derive(Debug)]
struct HardwareInfo {
  gpu_type: GpuType,
  gpu_memory_gb: f64,
  cpu_cores: usize,
  total_memory_gb: f64,
  available_memory_gb: f64,
}

#[derive(Debug, PartialEq)]
enum GpuType {
  NvidiaGpu(String), // Model name
  AmdGpu(String),    // Model name
  #[allow(dead_code)] // Only constructed on macOS
  AppleSilicon, // M1/M2/M3 etc
  IntelGpu,          // Integrated graphics
  None,              // CPU only
}

/// Detect GPU capabilities using system commands
fn detect_gpu() -> GpuType {
  // Check for NVIDIA GPU
  if let Ok(_) = which::which("nvidia-smi") {
    if let Ok(output) = std::process::Command::new("nvidia-smi")
      .args(&["--query-gpu=name", "--format=csv,noheader,nounits"])
      .output()
    {
      if output.status.success() {
        let gpu_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !gpu_name.is_empty() {
          return GpuType::NvidiaGpu(gpu_name);
        }
      }
    }
  }

  // Check for AMD GPU (ROCm)
  if let Ok(_) = which::which("rocm-smi") {
    if let Ok(output) = std::process::Command::new("rocm-smi")
      .args(&["--showproductname"])
      .output()
    {
      if output.status.success() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
          if line.contains("Card series:") || line.contains("Card model:") {
            let gpu_name = line.split(':').nth(1).unwrap_or("Unknown AMD GPU").trim();
            return GpuType::AmdGpu(gpu_name.to_string());
          }
        }
      }
    }
  }

  // Check for Apple Silicon on macOS
  #[cfg(target_os = "macos")]
  {
    if let Ok(output) = std::process::Command::new("system_profiler")
      .args(&["SPHardwareDataType"])
      .output()
    {
      if output.status.success() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        if output_str.contains("Apple M") || output_str.contains("Apple Silicon") {
          return GpuType::AppleSilicon;
        }
      }
    }
  }

  // Check for Intel integrated graphics (basic detection)
  if let Ok(output) = std::process::Command::new("lspci").arg("-v").output() {
    if output.status.success() {
      let output_str = String::from_utf8_lossy(&output.stdout);
      if output_str.contains("Intel") && output_str.contains("VGA") {
        return GpuType::IntelGpu;
      }
    }
  }

  GpuType::None
}

/// Get GPU memory in GB (estimates based on GPU type)
fn estimate_gpu_memory(gpu_type: &GpuType) -> f64 {
  match gpu_type {
    GpuType::NvidiaGpu(name) => {
      // Parse VRAM from common GPU names or use defaults
      if name.contains("RTX 4090") {
        24.0
      } else if name.contains("RTX 4080") {
        16.0
      } else if name.contains("RTX 4070") {
        12.0
      } else if name.contains("RTX 3090") {
        24.0
      } else if name.contains("RTX 3080") {
        10.0
      } else if name.contains("RTX 3070") {
        8.0
      } else if name.contains("RTX 3060") {
        12.0
      } else if name.contains("GTX 1660") {
        6.0
      } else if name.contains("GTX 1050") {
        4.0
      } else {
        8.0
      } // Conservative default
    }
    GpuType::AmdGpu(_) => 8.0,     // Conservative estimate
    GpuType::AppleSilicon => 16.0, // Unified memory, conservative estimate
    GpuType::IntelGpu => 2.0,      // Shared system memory
    GpuType::None => 0.0,
  }
}

/// Detect comprehensive hardware information
fn detect_hardware() -> HardwareInfo {
  let mut system = System::new_all();
  system.refresh_all();

  let gpu_type = detect_gpu();
  let gpu_memory_gb = estimate_gpu_memory(&gpu_type);
  let cpu_cores = system.cpus().len();
  let total_memory_gb = system.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0;
  let available_memory_gb = system.available_memory() as f64 / 1024.0 / 1024.0 / 1024.0;

  HardwareInfo {
    gpu_type,
    gpu_memory_gb,
    cpu_cores,
    total_memory_gb,
    available_memory_gb,
  }
}

/// Determine the best model based on comprehensive hardware analysis
pub fn select_model_by_ram() -> String {
  let hw = detect_hardware();

  let model = match &hw.gpu_type {
    GpuType::NvidiaGpu(_name) => {
      // NVIDIA GPUs can handle larger models efficiently
      if hw.gpu_memory_gb >= 16.0 {
        "mistral:instruct" // Large GPU can handle Mistral
      } else if hw.gpu_memory_gb >= 8.0 {
        "gemma:7b" // Medium GPU
      } else if hw.gpu_memory_gb >= 4.0 {
        "gemma:2b" // Smaller GPU
      } else {
        "phi" // Very limited GPU
      }
    }
    GpuType::AmdGpu(_) => {
      // AMD GPUs with ROCm support, be conservative
      if hw.gpu_memory_gb >= 12.0 && hw.available_memory_gb >= 8.0 {
        "gemma:7b"
      } else if hw.gpu_memory_gb >= 6.0 {
        "gemma:2b"
      } else {
        "phi"
      }
    }
    GpuType::AppleSilicon => {
      // Apple Silicon with unified memory is very efficient
      if hw.total_memory_gb >= 16.0 {
        "mistral:instruct" // Apple Silicon handles this well
      } else if hw.total_memory_gb >= 8.0 {
        "gemma:7b"
      } else {
        "gemma:2b"
      }
    }
    GpuType::IntelGpu | GpuType::None => {
      // CPU-only or integrated graphics - be very conservative
      if hw.available_memory_gb >= 8.0 && hw.cpu_cores >= 8 {
        "gemma:2b" // Only small models on CPU
      } else if hw.available_memory_gb >= 4.0 {
        "phi"
      } else {
        "tinyllama"
      }
    }
  };

  // Print comprehensive hardware info
  let gpu_info = match &hw.gpu_type {
    GpuType::NvidiaGpu(name) => format!("NVIDIA {} ({:.1}GB VRAM)", name, hw.gpu_memory_gb),
    GpuType::AmdGpu(name) => format!("AMD {} ({:.1}GB VRAM)", name, hw.gpu_memory_gb),
    GpuType::AppleSilicon => "Apple Silicon (Unified Memory)".to_string(),
    GpuType::IntelGpu => "Intel Integrated Graphics".to_string(),
    GpuType::None => "CPU Only".to_string(),
  };

  println!(
    "Hardware detected - GPU: {}, CPU: {} cores, RAM: {:.1}GB total, {:.1}GB available",
    gpu_info, hw.cpu_cores, hw.total_memory_gb, hw.available_memory_gb
  );
  println!("Selected model '{}' for optimal performance", model);

  model.to_string()
}

/// Ensure model is available, pull if necessary
pub async fn ensure_model_available(client: &OllamaClient, model: &str) -> Result<()> {
  let available_models = client.list_models().await?;

  if !available_models.iter().any(|m| m.starts_with(model)) {
    client.pull_model(model).await?;
  }

  Ok(())
}
