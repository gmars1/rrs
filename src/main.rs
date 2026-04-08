#![allow(clippy::print_literal)]

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "android"))]
mod tui_app {
    use audio_controller::{AudioController, DefaultController, Session};
    use crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use ratatui::{
        backend::CrosstermBackend,
        layout::{Constraint, Direction, Layout, Rect},
        style::{Color, Modifier, Style},
        text::{Line, Span},
        widgets::{Block, Borders, Clear, Paragraph},
        Frame, Terminal,
    };
    use std::io;

    #[derive(Clone, Copy, PartialEq)]
    enum AppMode {
        Normal,
        Editing,
        Help,
    }

    #[derive(Clone, Copy, PartialEq)]
    enum EditingField {
        None,
        Volume,
        BalanceLeft,
        BalanceRight,
    }

    struct App {
        sessions: Vec<Session>,
        selected: usize,
        mode: AppMode,
        editing: EditingField,
        input_buffer: String,
        message: String,
        message_timer: u32,
        running: bool,
    }

    impl App {
        fn set_message(&mut self, msg: String) {
            self.message = msg;
            self.message_timer = 8;
        }
    }

    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let mut controller =
            DefaultController::new().map_err(|e| format!("Failed to create controller: {}", e))?;
        controller.refresh_sessions().map_err(|e| e.to_string())?;

        let sessions = controller.list_sessions().map_err(|e| e.to_string())?;

        let mut app = App {
            sessions,
            selected: 0,
            mode: AppMode::Normal,
            editing: EditingField::None,
            input_buffer: String::new(),
            message: String::new(),
            message_timer: 0,
            running: true,
        };

        let res = run_app(&mut terminal, &mut controller, &mut app);

        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        if let Err(err) = res {
            eprintln!("Error: {err:?}");
        }

        Ok(())
    }

    fn run_app(
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        controller: &mut impl AudioController,
        app: &mut App,
    ) -> io::Result<()> {
        loop {
            terminal.draw(|f| ui(f, app))?;

            if !app.running {
                return Ok(());
            }

            if event::poll(std::time::Duration::from_millis(250))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }

                    match app.mode {
                        AppMode::Normal => match key.code {
                            KeyCode::Char('q') => {
                                app.running = false;
                            }
                            KeyCode::Char('h') | KeyCode::Char('?') => {
                                app.mode = AppMode::Help;
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                if !app.sessions.is_empty() {
                                    app.selected = (app.selected + 1) % app.sessions.len();
                                }
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                if !app.sessions.is_empty() {
                                    app.selected = app
                                        .selected
                                        .checked_sub(1)
                                        .unwrap_or(app.sessions.len() - 1);
                                }
                            }
                            KeyCode::Char('r') => {
                                if let Err(e) = controller.refresh_sessions() {
                                    app.set_message(format!("Refresh failed: {e}"));
                                } else {
                                    app.sessions = controller.list_sessions().unwrap_or_default();
                                    app.set_message("Sessions refreshed".into());
                                    if app.selected >= app.sessions.len() {
                                        app.selected = app.sessions.len().saturating_sub(1);
                                    }
                                }
                            }
                            KeyCode::Char('v') => {
                                if !app.sessions.is_empty() {
                                    app.editing = EditingField::Volume;
                                    app.mode = AppMode::Editing;
                                    app.input_buffer.clear();
                                }
                            }
                            KeyCode::Char('b') => {
                                if !app.sessions.is_empty() {
                                    app.editing = EditingField::BalanceLeft;
                                    app.mode = AppMode::Editing;
                                    app.input_buffer.clear();
                                }
                            }
                            KeyCode::Char('m') => {
                                if !app.sessions.is_empty() {
                                    let session = &app.sessions[app.selected];
                                    if let Err(e) = controller.set_mute(session.id, !session.mute) {
                                        app.set_message(format!("Mute failed: {e}"));
                                    } else {
                                        app.set_message("Mute toggled".into());
                                    }
                                    app.sessions = controller.list_sessions().unwrap_or_default();
                                }
                            }
                            _ => {}
                        },
                        AppMode::Help => {
                            app.mode = AppMode::Normal;
                        }
                        AppMode::Editing => match key.code {
                            KeyCode::Enter => {
                                apply_input(controller, app);
                            }
                            KeyCode::Esc => {
                                app.mode = AppMode::Normal;
                                app.editing = EditingField::None;
                                app.input_buffer.clear();
                            }
                            KeyCode::Char(c) if c.is_ascii_digit() => {
                                if app.input_buffer.len() < 3 {
                                    app.input_buffer.push(c);
                                }
                            }
                            KeyCode::Backspace => {
                                app.input_buffer.pop();
                            }
                            _ => {}
                        },
                    }
                }
            }

            if app.message_timer > 0 {
                app.message_timer -= 1;
                if app.message_timer == 0 {
                    app.message.clear();
                }
            }
        }
    }

    fn apply_input(controller: &mut impl AudioController, app: &mut App) {
        if app.sessions.is_empty() {
            app.mode = AppMode::Normal;
            app.editing = EditingField::None;
            app.input_buffer.clear();
            return;
        }

        if app.input_buffer.is_empty() {
            app.set_message("Enter a value".into());
            return;
        }

        let value: u32 = match app.input_buffer.parse() {
            Ok(v) => v,
            Err(_) => {
                app.set_message("Invalid number".into());
                app.mode = AppMode::Normal;
                app.editing = EditingField::None;
                return;
            }
        };

        let session = &app.sessions[app.selected];

        match app.editing {
            EditingField::Volume => {
                if value > 100 {
                    app.set_message("Value must be 0-100".into());
                    return;
                }
                let v = value as f32 / 100.0;
                if let Err(e) = controller.set_volume(session.id, v, v) {
                    app.set_message(format!("Volume failed: {e}"));
                } else {
                    app.set_message(format!("Volume set to {value}%"));
                }
            }
            EditingField::BalanceLeft => {
                if value > 100 {
                    app.set_message("Value must be 0-100".into());
                    return;
                }
                let left = value as f32 / 100.0;
                let right = session.right_volume;
                if let Err(e) = controller.set_volume(session.id, left, right) {
                    app.set_message(format!("Balance failed: {e}"));
                } else {
                    // Refresh sessions to get updated left_volume
                    app.sessions = controller.list_sessions().unwrap_or_default();
                    app.editing = EditingField::BalanceRight;
                    app.input_buffer.clear();
                    app.set_message("Enter Right (0-100)".into());
                    return;
                }
            }
            EditingField::BalanceRight => {
                if value > 100 {
                    app.set_message("Value must be 0-100".into());
                    return;
                }
                // Refresh sessions first to get the updated left_volume from BalanceLeft step
                app.sessions = controller.list_sessions().unwrap_or_default();
                let session = &app.sessions[app.selected];
                let left = session.left_volume;
                let right = value as f32 / 100.0;
                if let Err(e) = controller.set_volume(session.id, left, right) {
                    app.set_message(format!("Balance failed: {e}"));
                } else {
                    app.set_message(format!(
                        "Balance set: L={}{}% R={value}%",
                        (left * 100.0).round() as u32,
                        "%"
                    ));
                }
            }
            EditingField::None => {}
        }

        app.sessions = controller.list_sessions().unwrap_or_default();
        app.mode = AppMode::Normal;
        app.editing = EditingField::None;
        app.input_buffer.clear();
    }

    fn ui(f: &mut Frame, app: &App) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(3),
            ])
            .split(f.area());

        let header = Paragraph::new(vec![
            Line::from(Span::styled(
                " Audio Controller ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                " Up/Down: navigate | v: volume | b: balance | m: mute | r: refresh | h: help | q: quit",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .block(Block::default().borders(Borders::ALL).title(" Controls "));
        f.render_widget(header, chunks[0]);

        if app.sessions.is_empty() {
            let empty = Paragraph::new("No active audio sessions")
                .block(Block::default().borders(Borders::ALL).title(" Sessions "))
                .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(empty, chunks[1]);
        } else {
            let items: Vec<Line> = app
                .sessions
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let pid = if s.pid > 0 {
                        s.pid.to_string()
                    } else {
                        "system".to_string()
                    };
                    let l = (s.left_volume * 100.0).round() as u32;
                    let r = (s.right_volume * 100.0).round() as u32;
                    let ch = if s.channel_count <= 1 {
                        "mono"
                    } else {
                        "stereo"
                    };
                    let mute = if s.mute { " MUTED" } else { "" };

                    let style = if i == app.selected {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };

                    let arrow = if i == app.selected { "▸ " } else { "  " };
                    Line::from(vec![
                        Span::styled(arrow, style),
                        Span::styled(
                            format!(
                                "{:<4} {:<12} {} (L:{}% R:{}% {}{})",
                                i + 1,
                                pid,
                                s.name,
                                l,
                                r,
                                ch,
                                mute
                            ),
                            style,
                        ),
                    ])
                })
                .collect();

            let list = Paragraph::new(items)
                .block(Block::default().borders(Borders::ALL).title(" Sessions "));
            f.render_widget(list, chunks[1]);

            if app.selected < app.sessions.len() {
                let s = &app.sessions[app.selected];
                let bar_text = format!(
                    "  Volume: {}  L: {}{}  R: {}{}  Channels: {}",
                    format_bar(s.volume),
                    (s.left_volume * 100.0).round() as u32,
                    "%",
                    (s.right_volume * 100.0).round() as u32,
                    "%",
                    if s.channel_count <= 1 {
                        "mono"
                    } else {
                        "stereo"
                    }
                );
                let detail = Paragraph::new(Line::from(Span::styled(
                    bar_text,
                    Style::default().fg(Color::Green),
                )))
                .block(Block::default().borders(Borders::ALL).title(" Selected "));
                f.render_widget(detail, chunks[2]);
            }
        }

        if app.mode == AppMode::Editing {
            let popup_area = centered_rect(45, 5, f.area());
            f.render_widget(Clear, popup_area);

            let label = match app.editing {
                EditingField::BalanceLeft => "Left Volume (0-100)",
                EditingField::BalanceRight => "Right Volume (0-100)",
                EditingField::Volume => "Volume (0-100)",
                EditingField::None => "",
            };

            let session = &app.sessions[app.selected];
            let cur_l = (session.left_volume * 100.0).round() as u32;
            let cur_r = (session.right_volume * 100.0).round() as u32;

            let popup = Paragraph::new(vec![
                Line::from(Span::styled(label, Style::default().fg(Color::Cyan))),
                Line::from(Span::styled(
                    format!("Current: L={}% R={}%  |  New value:", cur_l, cur_r),
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(Span::styled(
                    format!("> {}_", app.input_buffer),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )),
            ])
            .block(Block::default().borders(Borders::ALL).title(" Input "))
            .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(popup, popup_area);
        }

        if app.mode == AppMode::Help {
            let popup_area = centered_rect(60, 55, f.area());
            f.render_widget(Clear, popup_area);

            let help_text = vec![
                Line::from(Span::styled(
                    " Keyboard Shortcuts ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  Up/Down or j/k    Navigate sessions",
                    Style::default().fg(Color::White),
                )),
                Line::from(Span::styled(
                    "  v                 Set volume (0-100)",
                    Style::default().fg(Color::White),
                )),
                Line::from(Span::styled(
                    "  b                 Set balance L/R",
                    Style::default().fg(Color::White),
                )),
                Line::from(Span::styled(
                    "  m                 Toggle mute",
                    Style::default().fg(Color::White),
                )),
                Line::from(Span::styled(
                    "  r                 Refresh sessions",
                    Style::default().fg(Color::White),
                )),
                Line::from(Span::styled(
                    "  h or ?            Show this help",
                    Style::default().fg(Color::White),
                )),
                Line::from(Span::styled(
                    "  q                 Quit",
                    Style::default().fg(Color::White),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  Enter             Confirm input",
                    Style::default().fg(Color::White),
                )),
                Line::from(Span::styled(
                    "  Esc               Cancel / Close help",
                    Style::default().fg(Color::White),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    " Press any key to close",
                    Style::default().fg(Color::DarkGray),
                )),
            ];

            let help = Paragraph::new(help_text)
                .block(Block::default().borders(Borders::ALL).title(" Help "))
                .alignment(ratatui::layout::Alignment::Left);
            f.render_widget(help, popup_area);
        }

        if !app.message.is_empty() {
            let msg_area = Rect {
                x: f.area().width.saturating_sub(app.message.len() as u16 + 4) / 2,
                y: f.area().height.saturating_sub(2),
                width: app.message.len() as u16 + 4,
                height: 1,
            };
            f.render_widget(Clear, msg_area);
            let msg = Paragraph::new(Line::from(Span::styled(
                &app.message,
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )))
            .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(msg, msg_area);
        }
    }

    fn format_bar(volume: f32) -> String {
        let filled = (volume * 20.0).round() as usize;
        let empty = 20 - filled;
        format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
    }

    fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
    }
}

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "android"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tui_app::run()
}

#[cfg(target_os = "android")]
fn main() {
    eprintln!("This is a library for Android. Use JNI to call from Java/Kotlin.");
}
