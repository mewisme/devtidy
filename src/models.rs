use ratatui::widgets::ListState;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct CleanableItem {
  pub path: PathBuf,
  pub item_type: String,
  pub size: u64,
  pub info: String,
  pub selected: bool,
}

impl CleanableItem {
  pub fn new(path: PathBuf, item_type: String, size: u64, info: String) -> Self {
    Self {
      path,
      item_type,
      size,
      info,
      selected: false,
    }
  }

  pub fn display_path(&self) -> String {
    self.path.to_string_lossy().to_string()
  }

  pub fn display_size(&self) -> String {
    human_bytes::human_bytes(self.size as f64)
  }

  pub fn display_info(&self) -> String {
    if !self.info.is_empty() {
      self.info.clone()
    } else {
      self.item_type.clone()
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppState {
  Scanning,
  Selecting,
  Cleaning,
  Complete,
  Help,
}

pub struct App {
  pub state: AppState,
  pub previous_state: Option<AppState>,
  pub items: Vec<CleanableItem>,
  pub list_state: ListState,
  pub scanning: bool,
  pub cleaning: bool,
  pub total_size: u64,
  pub cleaned_size: u64,
  pub current_dir: PathBuf,
  pub use_gitignore: bool,
  pub scan_start_time: Instant,
  pub scan_duration: Duration,
  pub scanned_items: usize,
  pub calculating_sizes: bool,
  pub pending_sizes: std::collections::HashMap<PathBuf, u64>,
  pub total_size_jobs: usize,
  pub completed_size_jobs: usize,
  pub progress: f32,
  pub processing_item: Option<String>,
  pub max_depth: usize,
  pub help_scroll: usize,
}

impl Default for App {
  fn default() -> Self {
    Self {
      state: AppState::Scanning,
      previous_state: None,
      items: Vec::new(),
      list_state: ListState::default(),
      scanning: true,
      cleaning: false,
      total_size: 0,
      cleaned_size: 0,
      current_dir: std::env::current_dir().unwrap_or_default(),
      use_gitignore: false,
      scan_start_time: Instant::now(),
      scan_duration: Duration::from_secs(0),
      scanned_items: 0,
      calculating_sizes: false,
      pending_sizes: std::collections::HashMap::new(),
      total_size_jobs: 0,
      completed_size_jobs: 0,
      progress: 0.0,
      processing_item: None,
      max_depth: 10,
      help_scroll: 0,
    }
  }
}

impl App {
  pub fn new(target_dir: PathBuf, use_gitignore: bool, max_depth: usize) -> Self {
    Self {
      current_dir: target_dir,
      use_gitignore,
      max_depth,
      ..Default::default()
    }
  }

  pub fn next(&mut self) {
    if self.items.is_empty() {
      return;
    }

    let i = match self.list_state.selected() {
      Some(i) => {
        if i >= self.items.len() - 1 {
          0
        } else {
          i + 1
        }
      }
      None => 0,
    };
    self.list_state.select(Some(i));
  }

  pub fn previous(&mut self) {
    if self.items.is_empty() {
      return;
    }

    let i = match self.list_state.selected() {
      Some(i) => {
        if i == 0 {
          self.items.len() - 1
        } else {
          i - 1
        }
      }
      None => 0,
    };
    self.list_state.select(Some(i));
  }

  pub fn toggle_selection(&mut self) {
    if let Some(i) = self.list_state.selected() {
      if i < self.items.len() {
        self.items[i].selected = !self.items[i].selected;
      }
    }
  }

  pub fn selected_count(&self) -> usize {
    self.items.iter().filter(|item| item.selected).count()
  }

  pub fn selected_size(&self) -> u64 {
    self
      .items
      .iter()
      .filter(|item| item.selected)
      .map(|item| item.size)
      .sum()
  }

  pub fn sort_by_size(&mut self) {
    self.items.sort_by(|a, b| b.size.cmp(&a.size));
  }

  pub fn get_selected_info(&self) -> String {
    let selected = self
      .items
      .iter()
      .filter(|item| item.selected)
      .collect::<Vec<_>>();
    match selected.len() {
      0 => "No items selected".to_string(),
      1 => format!(
        "Selected: {} ({})",
        selected[0].display_path(),
        selected[0].display_info()
      ),
      n => format!("Selected: {} items of various types", n),
    }
  }
}
