use crate::models::{App, AppState};
use ratatui::prelude::*;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap};

// Define a harmonious color scheme
mod colors {
  use ratatui::style::Color;

  pub const BACKGROUND: Color = Color::Rgb(13, 17, 23); // #0D1117
  pub const PRIMARY: Color = Color::Rgb(139, 92, 246); // #8B5CF6
  pub const SECONDARY: Color = Color::Rgb(124, 58, 237); // #7C3AED
  pub const SUCCESS: Color = Color::Rgb(63, 185, 80); // #3FB950
  pub const WARNING: Color = Color::Rgb(255, 196, 66); // #FFC442
  pub const ERROR: Color = Color::Rgb(255, 85, 85); // #FF5555
  pub const INFO: Color = Color::Rgb(186, 104, 200); // #BA68C8
  pub const TEXT: Color = Color::Rgb(201, 209, 217); // #C9D1D9
  pub const TEXT_DIM: Color = Color::Rgb(139, 148, 158); // #8B949E
  pub const HIGHLIGHT_BG: Color = Color::Rgb(27, 34, 39); // #1B2227
}

const SPINNER_FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub fn draw(f: &mut Frame, app: &App) {
  let area = f.size();
  f.render_widget(
    Block::default().style(Style::default().bg(colors::BACKGROUND)),
    area,
  );

  let chunks = Layout::default()
    .direction(Direction::Vertical)
    .margin(1)
    .constraints(
      [
        Constraint::Length(3),
        Constraint::Min(5),
        Constraint::Length(3),
      ]
      .as_ref(),
    )
    .split(area);

  draw_header(f, app, chunks[0]);

  match app.state {
    AppState::Scanning => draw_scanning_view(f, app, chunks[1]),
    AppState::Selecting => draw_selecting_view(f, app, chunks[1]),
    AppState::Cleaning => draw_cleaning_view(f, app, chunks[1]),
    AppState::Complete => draw_complete_view(f, app, chunks[1]),
    AppState::Help => draw_help_view(f, app, chunks[1]),
  }

  draw_footer(f, app, chunks[2]);
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
  let title = match app.state {
    AppState::Scanning => "DevTidy - Scanning",
    AppState::Selecting => "DevTidy - Select Items to Clean",
    AppState::Cleaning => "DevTidy - Cleaning",
    AppState::Complete => "DevTidy - Complete",
    AppState::Help => "DevTidy - Help",
  };

  let header_color = match app.state {
    AppState::Scanning => colors::PRIMARY,
    AppState::Selecting => colors::PRIMARY,
    AppState::Cleaning => colors::SECONDARY,
    AppState::Complete => colors::SUCCESS,
    AppState::Help => colors::INFO,
  };

  let header = Paragraph::new(title)
    .style(Style::default().fg(colors::TEXT).bg(header_color))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL));

  f.render_widget(header, area);
}

fn draw_scanning_view(f: &mut Frame, app: &App, area: Rect) {
  let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
      Constraint::Length(1),
      Constraint::Length(3),
      Constraint::Length(3),
      Constraint::Length(1),
    ])
    .split(area);

  let elapsed_millis = app.scan_start_time.elapsed().as_millis();
  let spinner_frame = SPINNER_FRAMES[(elapsed_millis / 80) as usize % SPINNER_FRAMES.len()];

  let spinner_text = if app.calculating_sizes {
    format!(
      "{} Calculating sizes for {} items...",
      spinner_frame, app.scanned_items
    )
  } else {
    format!("{} Scanning for cleanable items...", spinner_frame)
  };

  let spinner = Paragraph::new(spinner_text)
    .style(Style::default().fg(colors::INFO))
    .alignment(Alignment::Left);
  f.render_widget(spinner, chunks[0]);

  let current_dir = Paragraph::new(format!("Directory: {}", app.current_dir.display()))
    .style(Style::default().fg(colors::TEXT))
    .alignment(Alignment::Left);
  f.render_widget(current_dir, chunks[1]);

  let mut info_text = String::new();
  if app.calculating_sizes {
    let percent = if app.total_size_jobs > 0 {
      (app.completed_size_jobs as f32 / app.total_size_jobs as f32) * 100.0
    } else {
      0.0
    };

    info_text.push_str(&format!(
      "Scan time: {:?}\nItems found: {}\nSizes calculated: {}/{} ({:.1}%)",
      app.scan_duration, app.scanned_items, app.completed_size_jobs, app.total_size_jobs, percent
    ));

    let gauge = Gauge::default()
      .block(
        Block::default()
          .title("Calculating")
          .border_style(Style::default().fg(colors::PRIMARY)),
      )
      .gauge_style(Style::default().fg(colors::INFO).bg(Color::Black))
      .percent((percent as u16).min(100));
    f.render_widget(gauge, chunks[3]);
  } else {
    let elapsed = app.scan_start_time.elapsed();
    info_text.push_str(&format!(
      "Elapsed: {:?}\nItems found: {}",
      elapsed, app.scanned_items
    ));

    if elapsed.as_secs() > 5 && app.scanned_items == 0 {
      let warning =
        Paragraph::new("No cleanable items found yet. If this persists, check the directory path.")
          .style(Style::default().fg(colors::ERROR))
          .alignment(Alignment::Center);
      f.render_widget(warning, chunks[3]);
    }
  }

  let info = Paragraph::new(info_text)
    .style(Style::default().fg(colors::TEXT_DIM))
    .alignment(Alignment::Left);
  f.render_widget(info, chunks[2]);
}

fn draw_selecting_view(f: &mut Frame, app: &App, area: Rect) {
  let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Min(5), Constraint::Length(3)])
    .split(area);

  let items: Vec<ListItem> = app
    .items
    .iter()
    .enumerate()
    .map(|(_index, item)| {
      let prefix = if item.selected { "✓ " } else { "" };
      let first_line = format!(
        "{}{} ({})",
        prefix,
        item.path.display(),
        item.display_size()
      );

      let second_line = format!("└── {}", item.display_info());

      let display_text = format!("{}\n{}", first_line, second_line);

      let style = if item.selected {
        Style::default().fg(colors::SUCCESS)
      } else {
        Style::default().fg(colors::TEXT)
      };

      ListItem::new(display_text).style(style)
    })
    .collect();

  let list_area = chunks[0];

  let current_selection_index = app.list_state.selected();
  let highlight_style = if current_selection_index
    .and_then(|i| app.items.get(i))
    .map(|item| item.selected)
    .unwrap_or(false)
  {
    Style::default()
      .bg(colors::HIGHLIGHT_BG)
      .fg(colors::SUCCESS)
      .add_modifier(Modifier::BOLD)
  } else {
    Style::default()
      .bg(colors::HIGHLIGHT_BG)
      .fg(colors::TEXT)
      .add_modifier(Modifier::BOLD)
  };

  let list = List::new(items)
    .block(
      Block::default()
        .borders(Borders::ALL)
        .title("Cleanable Items")
        .border_style(Style::default().fg(colors::PRIMARY)),
    )
    .highlight_style(highlight_style)
    .highlight_symbol(">> ");

  let mut list_state = app.list_state.clone();
  f.render_stateful_widget(list, list_area, &mut list_state);

  if app.cleaning {
    let gauge = Gauge::default()
      .block(Block::default().title("Progress").borders(Borders::ALL))
      .gauge_style(Style::default().fg(colors::SUCCESS).bg(Color::Black))
      .ratio(app.progress as f64);
    f.render_widget(gauge, chunks[1]);
  } else {
    let selected_count = app.selected_count();
    let selected_size = app.selected_size();

    let status_text = if selected_count > 0 {
      format!(
        "{} | Total: {} ({})",
        app.get_selected_info(),
        app.scanned_items,
        human_bytes::human_bytes(selected_size as f64)
      )
    } else {
      format!(
        "Scan time: {:?} | Found: {} items | No selection",
        app.scan_duration, app.scanned_items,
      )
    };

    let border_color = if selected_count > 0 {
      colors::SECONDARY
    } else {
      colors::PRIMARY
    };

    let status = Paragraph::new(status_text)
      .block(
        Block::default()
          .borders(Borders::ALL)
          .border_style(Style::default().fg(border_color)),
      )
      .style(Style::default().fg(colors::TEXT))
      .alignment(Alignment::Left);
    f.render_widget(status, chunks[1]);
  }
}

fn draw_cleaning_view(f: &mut Frame, app: &App, area: Rect) {
  let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Length(1), Constraint::Length(3)])
    .split(area);

  let status = Paragraph::new("Cleaning selected items...")
    .style(Style::default().fg(colors::WARNING))
    .alignment(Alignment::Center);
  f.render_widget(status, chunks[0]);

  let gauge = Gauge::default()
    .block(
      Block::default()
        .title("Progress")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(colors::PRIMARY)),
    )
    .gauge_style(Style::default().fg(colors::SUCCESS).bg(Color::Black))
    .ratio(app.progress as f64);
  f.render_widget(gauge, chunks[1]);

  if let Some(item) = &app.processing_item {
    let status = Paragraph::new(format!("Processing: {}", item))
      .style(Style::default().fg(colors::TEXT_DIM))
      .alignment(Alignment::Left);
    let status_area = chunks[1].inner(&Margin {
      vertical: 2,
      horizontal: 0,
    });
    f.render_widget(status, status_area);
  }
}

fn draw_complete_view(f: &mut Frame, app: &App, area: Rect) {
  let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
      Constraint::Length(1),
      Constraint::Min(5),
      Constraint::Length(1),
    ])
    .split(area);

  let title = Paragraph::new("✓ Cleaning complete!")
    .style(Style::default().fg(colors::SUCCESS))
    .alignment(Alignment::Center);
  f.render_widget(title, chunks[0]);

  let stats_chunks = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
    .split(chunks[1]);

  let cleaned_info = Paragraph::new(format!(
    "\nCleaned: {}\n\nOriginal items: {}\nRemaining items: {}",
    human_bytes::human_bytes(app.cleaned_size as f64),
    app.scanned_items,
    app.items.len()
  ))
  .style(Style::default().fg(colors::TEXT))
  .alignment(Alignment::Left)
  .block(
    Block::default()
      .borders(Borders::ALL)
      .title("Results")
      .border_style(Style::default().fg(colors::SUCCESS)),
  );
  f.render_widget(cleaned_info, stats_chunks[0]);

  let info_text = "\nAll selected items have been cleaned.\n\nPress any key to return to the list view\nor 'q' to quit the application.";
  let info = Paragraph::new(info_text)
    .style(Style::default().fg(colors::TEXT))
    .alignment(Alignment::Left)
    .block(
      Block::default()
        .borders(Borders::ALL)
        .title("Next Steps")
        .border_style(Style::default().fg(colors::SECONDARY)),
    );
  f.render_widget(info, stats_chunks[1]);
}

fn draw_help_view(f: &mut Frame, app: &App, area: Rect) {
  let help_block = Block::default()
    .title(" Help ")
    .borders(Borders::ALL)
    .border_style(Style::default().fg(colors::PRIMARY))
    .style(Style::default().bg(colors::BACKGROUND));

  let inner_area = help_block.inner(area);
  f.render_widget(help_block, area);

  let help_lines = vec![
    Line::from(Span::styled(
      "DevTidy Help",
      Style::default()
        .fg(colors::PRIMARY)
        .add_modifier(Modifier::BOLD),
    )),
    Line::from(""),
    Line::from(Span::styled(
      "TUI Keyboard Shortcuts:",
      Style::default()
        .fg(colors::INFO)
        .add_modifier(Modifier::BOLD),
    )),
    Line::from(vec![
      Span::styled("  h     ", Style::default().fg(colors::PRIMARY)),
      Span::raw("Show/hide this help screen"),
    ]),
    Line::from(vec![
      Span::styled("  q/Esc ", Style::default().fg(colors::PRIMARY)),
      Span::raw("Quit the application"),
    ]),
    Line::from(vec![
      Span::styled("  Space ", Style::default().fg(colors::PRIMARY)),
      Span::raw("Select/deselect item"),
    ]),
    Line::from(vec![
      Span::styled("  c     ", Style::default().fg(colors::PRIMARY)),
      Span::raw("Clean selected items"),
    ]),
    Line::from(vec![
      Span::styled("  j/↓   ", Style::default().fg(colors::PRIMARY)),
      Span::raw("Move down"),
    ]),
    Line::from(vec![
      Span::styled("  k/↑   ", Style::default().fg(colors::PRIMARY)),
      Span::raw("Move up"),
    ]),
    Line::from(""),
    Line::from(Span::styled(
      "Help Navigation:",
      Style::default()
        .fg(colors::INFO)
        .add_modifier(Modifier::BOLD),
    )),
    Line::from(vec![
      Span::styled("  ↑/↓/mouse wheel ", Style::default().fg(colors::PRIMARY)),
      Span::raw("Scroll help content"),
    ]),
    Line::from(vec![
      Span::styled("  Page Up/Down    ", Style::default().fg(colors::PRIMARY)),
      Span::raw("Scroll faster"),
    ]),
    Line::from(vec![
      Span::styled("  h/Esc           ", Style::default().fg(colors::PRIMARY)),
      Span::raw("Return to previous screen"),
    ]),
    Line::from(""),
    Line::from(Span::styled(
      "Application States:",
      Style::default()
        .fg(colors::INFO)
        .add_modifier(Modifier::BOLD),
    )),
    Line::from(vec![
      Span::styled(
        "  Scanning: ",
        Style::default()
          .fg(colors::PRIMARY)
          .add_modifier(Modifier::BOLD),
      ),
      Span::raw("Looking for cleanable items"),
    ]),
    Line::from(vec![
      Span::styled(
        "  Selecting: ",
        Style::default()
          .fg(colors::PRIMARY)
          .add_modifier(Modifier::BOLD),
      ),
      Span::raw("Select items to clean"),
    ]),
    Line::from(vec![
      Span::styled(
        "  Cleaning: ",
        Style::default()
          .fg(colors::PRIMARY)
          .add_modifier(Modifier::BOLD),
      ),
      Span::raw("Removing selected items"),
    ]),
    Line::from(vec![
      Span::styled(
        "  Complete: ",
        Style::default()
          .fg(colors::PRIMARY)
          .add_modifier(Modifier::BOLD),
      ),
      Span::raw("Cleaning finished"),
    ]),
    Line::from(""),
    Line::from(Span::styled(
      "CLI Commands:",
      Style::default()
        .fg(colors::INFO)
        .add_modifier(Modifier::BOLD),
    )),
    Line::from(vec![
      Span::styled("  dd [DIRECTORY]", Style::default().fg(colors::PRIMARY)),
      Span::raw(" - Scan the specified directory (default: current directory)"),
    ]),
    Line::from(vec![
      Span::styled("  dd --gitignore", Style::default().fg(colors::PRIMARY)),
      Span::raw(" - Scan files matching .gitignore patterns"),
    ]),
    Line::from(vec![
      Span::styled(
        "  dd -d, --depth <DEPTH>",
        Style::default().fg(colors::PRIMARY),
      ),
      Span::raw(" - Set maximum scan depth (default: 6)"),
    ]),
    Line::from(vec![
      Span::styled("  dd -v, --version", Style::default().fg(colors::PRIMARY)),
      Span::raw(" - Show version information"),
    ]),
    Line::from(vec![
      Span::styled("  dd -i, --install", Style::default().fg(colors::PRIMARY)),
      Span::raw(" - Install devtidy globally"),
    ]),
  ];

  let visible_height = inner_area.height as usize;
  let total_lines = help_lines.len();
  let max_scroll = if total_lines > visible_height {
    total_lines - visible_height
  } else {
    0
  };

  let scroll = app.help_scroll.min(max_scroll);

  let visible_lines = help_lines
    .iter()
    .skip(scroll)
    .take(visible_height)
    .cloned()
    .collect::<Vec<Line>>();

  let text = Text::from(visible_lines);
  let help_paragraph = Paragraph::new(text)
    .alignment(Alignment::Left)
    .wrap(Wrap { trim: true })
    .scroll((0, 0));

  f.render_widget(help_paragraph, inner_area);

  if total_lines > visible_height {
    let scrollbar_area = Rect {
      x: inner_area.x + inner_area.width - 1,
      y: inner_area.y,
      width: 1,
      height: inner_area.height,
    };

    let percent_scrolled = scroll as f32 / max_scroll as f32;
    let scrollbar_height =
      (visible_height as f32 / total_lines as f32 * inner_area.height as f32).max(1.0) as u16;
    let scrollbar_top =
      inner_area.y + (percent_scrolled * (inner_area.height - scrollbar_height) as f32) as u16;

    let scrollbar = Block::default().style(Style::default().bg(colors::TEXT_DIM));

    f.render_widget(
      scrollbar,
      Rect {
        x: scrollbar_area.x,
        y: scrollbar_top,
        width: 1,
        height: scrollbar_height,
      },
    );
  }
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
  let footer_block = Block::default().style(Style::default().bg(colors::BACKGROUND));
  f.render_widget(footer_block, area);

  let footer_chunks = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([Constraint::Percentage(60), Constraint::Percentage(40)].as_ref())
    .split(area);

  let footer_text = match app.state {
    AppState::Scanning => "",
    AppState::Selecting => "↑/↓: navigate | Space: select | c: clean | q: quit | h: help",
    AppState::Cleaning => "q: quit",
    AppState::Complete => "any key: return | q: quit",
    AppState::Help => "↑/↓/Mouse: scroll | PageUp/Down: fast scroll | h/Esc: back",
  };

  let footer = Paragraph::new(footer_text)
    .style(Style::default().fg(colors::TEXT_DIM))
    .alignment(Alignment::Left);

  f.render_widget(footer, footer_chunks[0]);

  let status_text = match app.state {
    AppState::Scanning => "Scanning for items...".to_string(),
    AppState::Selecting => "".to_string(),
    AppState::Cleaning => "Cleaning selected items...".to_string(),
    AppState::Complete => format!("Cleaned {} items", app.cleaned_size),
    AppState::Help => {
      if app.help_scroll > 0 {
        "Scroll to see more ↓".to_string()
      } else {
        "Help".to_string()
      }
    }
  };

  let footer_help = Paragraph::new(status_text)
    .style(Style::default().fg(colors::TEXT_DIM))
    .alignment(Alignment::Right);
  f.render_widget(footer_help, footer_chunks[1]);
}
