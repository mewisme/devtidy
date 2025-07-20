use crate::models::CleanableItem;
use std::fs;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct CleanResult {
  pub path: String,
  pub success: bool,
  pub size: u64,
}

pub fn clean_selected_items(
  items: &[CleanableItem],
  sender: tokio::sync::mpsc::Sender<(usize, usize, Option<String>)>,
) -> tokio::task::JoinHandle<Vec<CleanResult>> {
  let selected: Vec<CleanableItem> = items.iter().filter(|item| item.selected).cloned().collect();

  let total = selected.len();

  tokio::spawn(async move {
    if total == 0 {
      return Vec::new();
    }

    let results = Arc::new(Mutex::new(Vec::new()));
    let mut handles = Vec::new();

    for (index, item) in selected.iter().enumerate() {
      let results_clone = Arc::clone(&results);
      let sender_clone = sender.clone();
      let path_str = item.display_path();
      let size = item.size;
      let path = item.path.clone();

      let handle = tokio::spawn(async move {
        let _ = sender_clone
          .send((index, total, Some(path_str.clone())))
          .await;

        let success = if path.is_dir() {
          match fs::remove_dir_all(&path) {
            Ok(_) => true,
            Err(_) => false,
          }
        } else {
          match fs::remove_file(&path) {
            Ok(_) => true,
            Err(_) => false,
          }
        };

        let result = CleanResult {
          path: path_str,
          success,
          size: if success { size } else { 0 },
        };

        {
          let mut results = results_clone.lock().unwrap();
          results.push(result);
        }

        let _ = sender_clone.send((index + 1, total, None)).await;
      });

      handles.push(handle);

      tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    for handle in handles {
      let _ = handle.await;
    }

    Arc::try_unwrap(results).unwrap().into_inner().unwrap()
  })
}
