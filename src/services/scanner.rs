use crate::core::constants::CLEANABLE_PATTERNS;
use crate::core::models::CleanableItem;
use ignore::WalkBuilder;
use std::collections::HashSet;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use walkdir::WalkDir;

pub fn scan_directory(dir: &Path, use_gitignore: bool, max_depth: usize) -> Vec<CleanableItem> {
  if use_gitignore {
    scan_gitignore_items(dir, max_depth)
  } else {
    scan_cleanable_items(dir, max_depth)
  }
}

fn scan_cleanable_items(dir: &Path, max_depth: usize) -> Vec<CleanableItem> {
  let items = Arc::new(Mutex::new(Vec::new()));
  let thread_count = num_cpus::get().max(2);

  let walker = WalkDir::new(dir)
    .min_depth(1)
    .max_depth(max_depth)
    .into_iter()
    .filter_entry(|e| {
      let name = e.file_name().to_string_lossy();
      !(name.starts_with(".") && name != ".git")
    });

  let entries: Vec<_> = walker.filter_map(Result::ok).collect();

  let chunks = split_into_chunks(entries, thread_count);
  let mut handles = vec![];

  for chunk in chunks {
    let items_clone = Arc::clone(&items);
    let handle = thread::spawn(move || {
      let mut local_items = Vec::new();

      for entry in chunk {
        let name = entry.file_name().to_string_lossy();
        let path = entry.path().to_path_buf();

        for (pattern, description) in CLEANABLE_PATTERNS.iter() {
          let pattern_matches = if pattern.contains('*') {
            match_glob_pattern(pattern, &name)
          } else {
            &name == pattern
          };

          if pattern_matches {
            let size = if entry.file_type().is_file() {
              entry.metadata().map(|m| m.len()).unwrap_or(0)
            } else {
              0
            };

            local_items.push(CleanableItem::new(
              path.clone(),
              description.to_string(),
              size,
              description.to_string(),
            ));
            break;
          }
        }
      }

      let mut items = items_clone.lock().unwrap();
      items.extend(local_items);
    });

    handles.push(handle);
  }

  for handle in handles {
    handle.join().unwrap();
  }

  let items = items.lock().unwrap().clone();
  items
}

fn scan_gitignore_items(dir: &Path, max_depth: usize) -> Vec<CleanableItem> {
  let gitignore_path = dir.join(".gitignore");
  if !gitignore_path.exists() {
    return Vec::new();
  }

  let patterns = match read_gitignore(&gitignore_path) {
    Ok(p) => p,
    Err(_) => return Vec::new(),
  };

  if patterns.is_empty() {
    return Vec::new();
  }

  let items = Arc::new(Mutex::new(Vec::new()));
  let seen_paths = Arc::new(Mutex::new(HashSet::new()));

  let walker = WalkBuilder::new(dir)
    .hidden(false)
    .ignore(false)
    .git_ignore(false)
    .max_depth(Some(max_depth))
    .build();

  let thread_count = num_cpus::get().max(2);
  let entries: Vec<_> = walker.filter_map(Result::ok).collect();
  let chunks = split_into_chunks_ignore(entries, thread_count);

  let mut handles = vec![];

  for chunk in chunks {
    let items_clone = Arc::clone(&items);
    let seen_paths_clone = Arc::clone(&seen_paths);
    let patterns_clone = patterns.clone();
    let dir = dir.to_path_buf();

    let handle = thread::spawn(move || {
      let mut local_items = Vec::new();

      for entry in chunk {
        let path = entry.path().to_path_buf();
        let rel_path = path.strip_prefix(&dir).unwrap_or(&path);
        let rel_path_str = rel_path.to_string_lossy().to_string();

        for pattern in &patterns_clone {
          if matches_gitignore_pattern(pattern, &rel_path_str) {
            let mut seen = seen_paths_clone.lock().unwrap();
            if !seen.contains(&path) {
              seen.insert(path.clone());

              let size = if entry.path().is_file() {
                match std::fs::metadata(entry.path()) {
                  Ok(metadata) => metadata.len(),
                  Err(_) => 0,
                }
              } else {
                0
              };

              local_items.push(CleanableItem::new(
                path.clone(),
                format!("Gitignore pattern: {}", pattern),
                size,
                "Matches .gitignore pattern".to_string(),
              ));
            }
            break;
          }
        }
      }

      let mut items = items_clone.lock().unwrap();
      items.extend(local_items);
    });

    handles.push(handle);
  }

  for handle in handles {
    handle.join().unwrap();
  }

  let items = items.lock().unwrap().clone();
  items
}

pub fn calculate_directory_sizes(
  items: &mut [CleanableItem],
  sender: tokio::sync::mpsc::Sender<(PathBuf, u64)>,
) {
  let dir_items: Vec<CleanableItem> = items
    .iter()
    .filter(|item| item.size == 0 && item.path.is_dir())
    .cloned()
    .collect();

  if dir_items.is_empty() {
    return;
  }

  let items_arc = Arc::new(dir_items);
  let thread_count = num_cpus::get().max(2);
  let chunk_size = (items_arc.len() / thread_count) + 1;

  for chunk_start in (0..items_arc.len()).step_by(chunk_size) {
    let chunk_end = (chunk_start + chunk_size).min(items_arc.len());
    let items_chunk = items_arc.clone();
    let sender = sender.clone();

    tokio::spawn(async move {
      for i in chunk_start..chunk_end {
        let item = &items_chunk[i];
        let size = get_directory_size(&item.path);
        if let Err(_) = sender.send((item.path.clone(), size)).await {
          break;
        }
      }
    });
  }
}

fn get_directory_size(path: &Path) -> u64 {
  WalkDir::new(path)
    .into_iter()
    .filter_map(|e| e.ok())
    .filter_map(|e| {
      if e.file_type().is_file() {
        e.metadata().ok().map(|m| m.len())
      } else {
        None
      }
    })
    .sum()
}

fn match_glob_pattern(pattern: &str, name: &str) -> bool {
  if let Ok(glob) = glob::Pattern::new(pattern) {
    glob.matches(name)
  } else {
    false
  }
}

fn matches_gitignore_pattern(pattern: &str, path: &str) -> bool {
  if pattern.ends_with('/') {
    let pattern = pattern.trim_end_matches('/');
    path == pattern || path.starts_with(&format!("{}/", pattern))
  } else if pattern.contains('*') {
    let base_name = Path::new(path)
      .file_name()
      .map(|s| s.to_string_lossy().to_string())
      .unwrap_or_default();

    match_glob_pattern(pattern, &base_name) || match_glob_pattern(pattern, path)
  } else {
    path == pattern || path.contains(pattern) || path.ends_with(&format!("/{}", pattern))
  }
}

fn read_gitignore(path: &Path) -> io::Result<Vec<String>> {
  let file = File::open(path)?;
  let reader = BufReader::new(file);
  let mut patterns = Vec::new();

  for line in reader.lines() {
    let line = line?;
    let trimmed = line.trim();
    if !trimmed.is_empty() && !trimmed.starts_with('#') && !trimmed.starts_with('!') {
      patterns.push(trimmed.to_string());
    }
  }

  Ok(patterns)
}

fn split_into_chunks<T: Clone>(items: Vec<T>, chunk_count: usize) -> Vec<Vec<T>> {
  let mut chunks = Vec::new();
  let chunk_size = if items.is_empty() {
    0
  } else {
    (items.len() / chunk_count) + 1
  };

  for i in 0..chunk_count {
    let start = i * chunk_size;
    if start >= items.len() {
      break;
    }
    let end = (start + chunk_size).min(items.len());
    chunks.push(items[start..end].to_vec());
  }

  chunks
}

fn split_into_chunks_ignore<T: Clone>(items: Vec<T>, chunk_count: usize) -> Vec<Vec<T>> {
  split_into_chunks(items, chunk_count)
}
