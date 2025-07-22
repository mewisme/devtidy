use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

/// Create an animated spinner with consistent styling
#[allow(dead_code)]
pub fn create_spinner(message: &str, color: &str) -> ProgressBar {
  let spinner = ProgressBar::new_spinner();
  spinner.set_style(
    ProgressStyle::default_spinner()
      .template(&format!("{{spinner:.{}}} {{msg}}", color))
      .unwrap()
      .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
  );
  spinner.set_message(message.to_string());
  spinner.enable_steady_tick(Duration::from_millis(100));
  spinner
}

/// Finish spinner with optional icon and color
#[allow(dead_code)]
pub fn finish_spinner(pb: &ProgressBar, final_msg: &str, icon: Option<&str>, color: Option<&str>) {
  let resolved_icon = match icon {
    Some("success") => Some("✔"),
    Some("error") => Some("✖"),
    Some("warn") | Some("warning") => Some("⚠"),
    Some("info") => Some("ℹ"),
    _ => None,
  };

  let mut msg = String::new();

  if let Some(symbol) = resolved_icon {
    msg.push_str(symbol);
    msg.push(' ');
  }

  msg.push_str(final_msg);

  let colored_msg = if let Some(c) = color {
    format!(
      "\x1b[{}m{}\x1b[0m",
      match c {
        "black" => "30",
        "red" => "31",
        "green" => "32",
        "yellow" => "33",
        "blue" => "34",
        "magenta" => "35",
        "cyan" => "36",
        "white" => "37",
        _ => "0",
      },
      msg
    )
  } else {
    msg
  };

  pb.set_style(ProgressStyle::with_template("{msg}").unwrap());
  pb.finish_with_message(colored_msg);
}
