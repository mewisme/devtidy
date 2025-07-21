use crate::core::models::{App, AppState, CleanableItem};
use crate::services::cleaner::clean_selected_items;
use crate::services::scanner::{calculate_directory_sizes, scan_directory};
use crate::ui::ui as ui_module;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, MouseEventKind};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

pub async fn run_app(
  app: &mut App,
  terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
) -> Result<()> {
  app.state = AppState::Scanning;
  app.scanning = true;
  app.scan_start_time = Instant::now();

  terminal.draw(|f| ui_module::draw(f, app))?;

  let (scan_tx, mut scan_rx) = mpsc::channel::<ScanUpdate>(32);

  let _scan_task = tokio::spawn(scan_background(
    app.current_dir.clone(),
    app.use_gitignore,
    scan_tx.clone(),
    app.scan_start_time,
    app.max_depth,
  ));

  let mut last_key_time = Instant::now();
  let mut last_key_code = None;
  let key_debounce = Duration::from_millis(150);

  crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture)?;

  loop {
    if let Ok(update) = scan_rx.try_recv() {
      process_scan_update(app, update);
    }

    terminal.draw(|f| ui_module::draw(f, app))?;

    if event::poll(Duration::from_millis(16))? {
      match event::read()? {
        Event::Key(key) => {
          let now = Instant::now();
          let is_same_key = last_key_code == Some(key.code);
          if now.duration_since(last_key_time) >= key_debounce || !is_same_key {
            last_key_time = now;
            last_key_code = Some(key.code);
            if !handle_key_event(app, key).await? {
              break;
            }
          }
        }
        Event::Mouse(mouse) => {
          if app.state == AppState::Selecting {
            match mouse.kind {
              MouseEventKind::ScrollDown => {
                app.next();
              }
              MouseEventKind::ScrollUp => {
                app.previous();
              }
              _ => {}
            }
          } else if app.state == AppState::Help {
            match mouse.kind {
              MouseEventKind::ScrollDown => {
                app.help_scroll += 1;
              }
              MouseEventKind::ScrollUp => {
                if app.help_scroll > 0 {
                  app.help_scroll -= 1;
                }
              }
              _ => {}
            }
          }
        }
        _ => {}
      }
    }

    tokio::time::sleep(Duration::from_millis(10)).await;
  }

  let _ = crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture);

  Ok(())
}

enum ScanUpdate {
  ItemsFound(Vec<CleanableItem>),
  SizeUpdate(PathBuf, u64),
  SizeCalculationComplete,
  ScanComplete(Duration),
  ItemsScanned(usize),
}

async fn scan_background(
  dir: PathBuf,
  use_gitignore: bool,
  tx: mpsc::Sender<ScanUpdate>,
  start_time: Instant,
  max_depth: usize,
) -> Result<()> {
  let items = tokio::task::spawn_blocking(move || scan_directory(&dir, use_gitignore, max_depth))
    .await
    .unwrap();

  let _ = tx.send(ScanUpdate::ItemsFound(items.clone())).await;
  let _ = tx.send(ScanUpdate::ItemsScanned(items.len())).await;

  let _ = tx
    .send(ScanUpdate::ScanComplete(start_time.elapsed()))
    .await;

  let dir_items = items
    .iter()
    .filter(|item| item.size == 0 && item.path.is_dir())
    .count();

  if dir_items > 0 {
    let (size_tx, mut size_rx) = mpsc::channel(32);

    calculate_directory_sizes(&mut items.clone(), size_tx);

    let mut completed = 0;
    let total = dir_items;

    while let Some((path, size)) = size_rx.recv().await {
      let _ = tx.send(ScanUpdate::SizeUpdate(path, size)).await;

      completed += 1;

      if completed % 5 == 0 || completed >= total {
        let _ = tx.send(ScanUpdate::ItemsScanned(items.len())).await;
      }

      if completed >= total {
        break;
      }
    }

    let _ = tx.send(ScanUpdate::SizeCalculationComplete).await;
  } else {
    let _ = tx.send(ScanUpdate::SizeCalculationComplete).await;
  }

  let final_duration = start_time.elapsed();
  let _ = tx.send(ScanUpdate::ScanComplete(final_duration)).await;

  Ok(())
}

fn process_scan_update(app: &mut App, update: ScanUpdate) {
  match update {
    ScanUpdate::ItemsFound(items) => {
      app.items = items;
      app.scanned_items = app.items.len();
      app.total_size_jobs = app.items.len();
      app.completed_size_jobs = 0;
      app.pending_sizes.clear();

      if !app.items.is_empty() {
        app.calculating_sizes = true;
      }
    }
    ScanUpdate::ItemsScanned(count) => {
      app.scanned_items = count;
    }
    ScanUpdate::SizeUpdate(path, size) => {
      app.pending_sizes.insert(path.clone(), size);
      app.completed_size_jobs += 1;

      for item in &mut app.items {
        if item.path == path {
          item.size = size;
          break;
        }
      }
    }
    ScanUpdate::SizeCalculationComplete => {
      app.sort_by_size();

      let total_size: u64 = app.items.iter().map(|item| item.size).sum();
      app.total_size = total_size;

      app.state = AppState::Selecting;
      app.scanning = false;
      app.calculating_sizes = false;

      if !app.items.is_empty() {
        app.list_state.select(Some(0));
      }
    }
    ScanUpdate::ScanComplete(duration) => {
      app.scan_duration = duration;

      if !app.calculating_sizes {
        app.state = AppState::Selecting;
        app.scanning = false;

        if !app.items.is_empty() {
          app.list_state.select(Some(0));
        }
      }
    }
  }
}

async fn handle_key_event(app: &mut App, key: KeyEvent) -> Result<bool> {
  match app.state {
    AppState::Scanning => {
      if key.code == KeyCode::Char('q') || key.code == KeyCode::Esc {
        return Ok(false);
      } else if key.code == KeyCode::Char('h') {
        app.previous_state = Some(app.state);
        app.state = AppState::Help;
        app.help_scroll = 0;
      }
    }
    AppState::Selecting => match key.code {
      KeyCode::Char('q') | KeyCode::Esc => return Ok(false),
      KeyCode::Char('h') => {
        app.previous_state = Some(app.state);
        app.state = AppState::Help;
        app.help_scroll = 0;
      }
      KeyCode::Char('c') => {
        if app.selected_count() > 0 && !app.cleaning {
          start_cleaning(app).await;
        }
      }
      KeyCode::Char(' ') => {
        if !app.cleaning {
          app.toggle_selection();
          app.total_size = app.selected_size();
        }
      }
      KeyCode::Up | KeyCode::Char('k') => {
        app.previous();
      }
      KeyCode::Down | KeyCode::Char('j') => {
        app.next();
      }
      _ => {}
    },
    AppState::Cleaning => {
      if key.code == KeyCode::Char('q') || key.code == KeyCode::Esc {
        return Ok(false);
      } else if key.code == KeyCode::Char('h') {
        app.previous_state = Some(app.state);
        app.state = AppState::Help;
        app.help_scroll = 0;
      }
    }
    AppState::Complete => match key.code {
      KeyCode::Char('q') | KeyCode::Esc => return Ok(false),
      KeyCode::Char('h') => {
        app.previous_state = Some(app.state);
        app.state = AppState::Help;
        app.help_scroll = 0;
      }
      _ => {
        app.state = AppState::Selecting;
        if !app.items.is_empty() {
          app.list_state.select(Some(0));
        }
      }
    },
    AppState::Help => match key.code {
      KeyCode::Esc | KeyCode::Char('h') => {
        if let Some(previous_state) = app.previous_state {
          app.state = previous_state;
          app.previous_state = None;
        } else {
          app.state = AppState::Selecting;
        }
      }
      KeyCode::Up | KeyCode::Char('k') => {
        if app.help_scroll > 0 {
          app.help_scroll -= 1;
        }
      }
      KeyCode::Down | KeyCode::Char('j') => {
        app.help_scroll += 1;
      }
      KeyCode::PageUp => {
        if app.help_scroll >= 5 {
          app.help_scroll -= 5;
        } else {
          app.help_scroll = 0;
        }
      }
      KeyCode::PageDown => {
        app.help_scroll += 5;
      }
      _ => {}
    },
  }

  Ok(true)
}

async fn start_cleaning(app: &mut App) {
  app.state = AppState::Cleaning;
  app.cleaning = true;
  app.progress = 0.0;

  let total_to_clean = app.selected_size();
  app.total_size = total_to_clean;

  let (tx, mut rx) = mpsc::channel(32);

  let clean_handle = clean_selected_items(&app.items, tx);

  while let Some((done, total, item)) = rx.recv().await {
    app.progress = done as f32 / total as f32;
    app.processing_item = item;
  }

  let results = clean_handle.await.unwrap();

  app.cleaned_size = results.iter().map(|r| r.size).sum();

  app.items.retain(|item| {
    !results
      .iter()
      .any(|r| r.path == item.display_path() && r.success)
  });

  app.state = AppState::Complete;
  app.cleaning = false;
}

pub fn initialize_app(
  target_dir: Option<String>,
  use_gitignore: bool,
  max_depth: usize,
) -> Result<App> {
  // Determine the base directory using the following priority:
  // 1. User provided path (--path/-p argument)
  // 2. Current working directory
  let dir = match target_dir {
    Some(path) => {
      let path = PathBuf::from(path);
      if !path.exists() {
        anyhow::bail!("Directory does not exist: {}", path.display());
      }
      if !path.is_dir() {
        anyhow::bail!("Not a directory: {}", path.display());
      }
      path.canonicalize()?
    }
    None => std::env::current_dir()?,
  };

  if use_gitignore {
    let gitignore_path = dir.join(".gitignore");
    if !gitignore_path.exists() {
      anyhow::bail!("No .gitignore file found in {}", dir.display());
    }
  }

  Ok(App::new(dir, use_gitignore, max_depth))
}
