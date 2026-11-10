use crate::state::{Channel, VersionState};
use crate::{docs, git};
use crossterm::event::{self, KeyCode, KeyEvent};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::prelude::*;
use ratatui::widgets::*;
use std::io::{self, stdout};
use std::time::Duration;

pub fn run_tui() -> Result<(), String> {
    let mut stdout = stdout();
    stdout.execute(EnterAlternateScreen).map_err(|e| e.to_string())?;
    terminal::enable_raw_mode().map_err(|e| e.to_string())?;

    let result = run_tui_inner(&mut stdout);

    terminal::disable_raw_mode().ok();
    stdout.execute(LeaveAlternateScreen).ok();
    result
}

struct App {
    state: VersionState,
    menu: Vec<&'static str>,
    selected: usize,
    status: String,
    error: Option<String>,

    // Sub-screens
    screen: Screen,
    commit_input: String,
    version_major: String,
    version_minor: String,
    version_patch: String,
    field_focus: usize,
    channel_index: usize,
}

#[derive(Clone, Copy, PartialEq)]
enum Screen {
    Main,
    SetVersion,
    ChannelSelect,
    CommitInput,
    ConfirmRelease,
    Done,
}

impl Default for App {
    fn default() -> Self {
        let state = VersionState::load().unwrap_or_else(|e| {
            eprintln!("failed to load version state: {}", e);
            VersionState::default()
        });
        Self {
            state,
            menu: vec![
                "View Version",
                "Edit Major/Minor/Patch",
                "Change Channel",
                "Bump Build Number",
                "Set Commit Message",
                "Release & Push to GitHub",
                "Quit",
            ],
            selected: 0,
            status: String::new(),
            error: None,
            screen: Screen::Main,
            commit_input: "Release v".to_string(),
            version_major: "1".to_string(),
            version_minor: "0".to_string(),
            version_patch: "1".to_string(),
            field_focus: 0,
            channel_index: 1,
        }
    }
}

impl App {
    fn sync_channel_index(&mut self) {
        self.channel_index = match self.state.channel {
            Channel::Alpha => 0,
            Channel::Beta => 1,
            Channel::Release => 2,
        };
    }

    fn apply_version_from_inputs(&mut self) {
        if let Ok(m) = self.version_major.parse::<u64>() {
            self.state.major = m;
        }
        if let Ok(m) = self.version_minor.parse::<u64>() {
            self.state.minor = m;
        }
        if let Ok(p) = self.version_patch.parse::<u64>() {
            self.state.patch = p;
        }
        self.state.channel = match self.channel_index {
            0 => Channel::Alpha,
            2 => Channel::Release,
            _ => Channel::Beta,
        };
        self.state.build = 0;
        let _ = self.state.save();
    }

    fn do_release(&mut self) -> Result<(), String> {
        // Save state first.
        self.state.save().map_err(|e| e.to_string())?;

        let tag = self.state.tag_name();
        let title = format!("Release {}", self.state.version_string());
        let body = format!(
            "## {}\n\nChannel: {}\nBuild: {}\n\n{}",
            self.state.version_string(),
            self.state.channel.as_str(),
            self.state.build,
            self.commit_input.trim()
        );

        // Update docs.
        docs::update_readme(&self.state)?;
        docs::add_changelog_entry(&self.state, &self.commit_input)?;

        // Git: stage version state + docs.
        git::add(&[".version.json", "readme.md", "CHANGELOG.md"])?;
        git::commit(&format!(
            "chore(release): {} {}",
            self.state.version_string(),
            self.commit_input.trim()
        ))?;
        git::tag(&tag, &title)?;
        git::push()?;

        let prerelease = self.state.channel != Channel::Release;
        git::gh_create_release(&tag, &title, &body, prerelease)?;

        self.status = format!(
            "Released {} (tag {}) pushed to GitHub",
            self.state.version_string(),
            tag
        );
        self.screen = Screen::Done;
        Ok(())
    }
}

fn draw(app: &App, frame: &mut Frame) {
    let size = frame.size();
    match app.screen {
        Screen::Main => draw_main(app, frame, size),
        Screen::SetVersion => draw_set_version(app, frame, size),
        Screen::ChannelSelect => draw_channel_select(app, frame, size),
        Screen::CommitInput => draw_commit_input(app, frame, size),
        Screen::ConfirmRelease => draw_confirm(app, frame, size),
        Screen::Done => draw_done(app, frame, size),
    }
}

fn draw_main(app: &App, frame: &mut Frame, size: Rect) {
    let block = Block::default()
        .title("LPPO Version Tracker")
        .borders(Borders::ALL);
    frame.render_widget(block, size);

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .margin(1)
        .split(size);

    // Version info panel.
    let info = format!(
        "Version: {}\nCore:     {}\nChannel:  {}\nBuild:    {}\nTag:      {}\n",
        app.state.version_string(),
        app.state.core_string(),
        app.state.channel.as_str(),
        app.state.build,
        app.state.tag_name()
    );
    frame.render_widget(
        Paragraph::new(info).block(Block::default().borders(Borders::ALL).title("Current")),
        inner[0],
    );

    // Menu.
    let items: Vec<ListItem> = app
        .menu
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let style = if i == app.selected {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(*m).style(style)
        })
        .collect();
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Actions"))
        .highlight_style(Style::default().bg(Color::DarkGray));
    let mut list_state = ListState::default();
    list_state.select(Some(app.selected));
    frame.render_stateful_widget(list, inner[1], &mut list_state);

    // Status line.
    let mut status_text = app.status.clone();
    if let Some(e) = &app.error {
        status_text = format!("ERROR: {}", e);
    }
    let hint = "↑/↓ navigate  Enter select  q quit";
    let status = format!("{}  {}", status_text, hint);
    frame.render_widget(
        Paragraph::new(status).block(Block::default().borders(Borders::NONE)),
        inner[2],
    );
}

fn draw_set_version(app: &App, frame: &mut Frame, size: Rect) {
    let block = Block::default()
        .title("Edit Version (manual input)")
        .borders(Borders::ALL);
    frame.render_widget(block, size);

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .margin(1)
        .split(size);

    let row = |label: &str, value: &str, focused: bool| -> Paragraph {
        let text = if focused {
            format!("{}: [{}]", label, value)
        } else {
            format!("{}: {}", label, value)
        };
        let style = if focused {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        Paragraph::new(text).style(style).block(
            Block::default().borders(Borders::LEFT),
        )
    };

    frame.render_widget(
        row("Major", &app.version_major, app.field_focus == 0),
        inner[0],
    );
    frame.render_widget(
        row("Minor", &app.version_minor, app.field_focus == 1),
        inner[1],
    );
    frame.render_widget(
        row("Patch", &app.version_patch, app.field_focus == 2),
        inner[2],
    );

    let channel_label = match app.state.channel {
        Channel::Alpha => "Alpha",
        Channel::Beta => "Beta",
        Channel::Release => "Release",
    };
    frame.render_widget(
        row("Channel", channel_label, false),
        inner[3],
    );

    let help = "Type digits to edit the focused field. Tab/↑↓ next field. Enter save. Esc cancel.";
    frame.render_widget(Paragraph::new(help), inner[4]);
}

fn draw_channel_select(app: &App, frame: &mut Frame, size: Rect) {
    let block = Block::default()
        .title("Select Release Channel")
        .borders(Borders::ALL);
    frame.render_widget(block, size);

    let items: Vec<ListItem> = Channel::all()
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let style = if i == app.channel_index {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let note = match c {
                Channel::Alpha => " (pre-release, unstable)",
                Channel::Beta => " (pre-release)",
                Channel::Release => " (stable)",
            };
            ListItem::new(format!("{}{}", c.as_str(), note)).style(style)
        })
        .collect();
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Channel"))
        .highlight_style(Style::default().bg(Color::DarkGray));
    let layout = Layout::default()
        .constraints([Constraint::Min(0)])
        .margin(1)
        .split(size);
    let mut channel_state = ListState::default();
    channel_state.select(Some(app.channel_index));
    frame.render_stateful_widget(list, layout[0], &mut channel_state);
}

fn draw_commit_input(app: &App, frame: &mut Frame, size: Rect) {
    let block = Block::default()
        .title("Commit Message")
        .borders(Borders::ALL);
    frame.render_widget(block, size);

    let inner = Layout::default()
        .constraints([Constraint::Length(5), Constraint::Min(0)])
        .margin(1)
        .split(size);

    let input = Paragraph::new(app.commit_input.as_str())
        .block(Block::default().borders(Borders::ALL).title("Message"));
    frame.render_widget(input, inner[0]);

    let help = "Enter the release commit message. Enter save. Esc cancel.";
    frame.render_widget(Paragraph::new(help), inner[1]);
}

fn draw_confirm(app: &App, frame: &mut Frame, size: Rect) {
    let block = Block::default()
        .title("Confirm Release")
        .borders(Borders::ALL);
    frame.render_widget(block, size);

    let text = format!(
        "Release {} as {} channel {}?\nCommit message: {}\n\nThis will commit, tag, push, and create a GitHub release.",
        app.state.version_string(),
        app.state.channel.as_str(),
        app.state.build,
        app.commit_input.trim()
    );
    let area = centered_rect(60, 40, size);
    frame.render_widget(
        Clear,
        area,
    );
    frame.render_widget(
        Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title("Confirm"))
            .alignment(Alignment::Center),
        area,
    );
}

fn draw_done(app: &App, frame: &mut Frame, size: Rect) {
    let text = format!(
        "Released {} successfully!\n\nPush to GitHub complete. Press any key to return.",
        app.state.version_string()
    );
    let area = centered_rect(50, 30, size);
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title("Done"))
            .alignment(Alignment::Center),
        area,
    );
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
    let popup_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1]);
    popup_layout[1]
}

fn run_tui_inner(stdout: &mut io::Stdout) -> Result<(), String> {
    let mut app = App::default();

    let backend = CrosstermBackend::new(stdout);
    let mut terminal =
        ratatui::Terminal::new(backend).map_err(|e| e.to_string())?;

    loop {
        if let Err(e) = app.state.save() {
            app.error = Some(format!("failed to save state: {}", e));
        }

        if crossterm::event::poll(Duration::from_millis(100)).map_err(|e| e.to_string())? {
            match crossterm::event::read().map_err(|e| e.to_string())? {
                event::Event::Key(key) => {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            if app.screen != Screen::Main {
                                app.screen = Screen::Main;
                                app.error = None;
                                continue;
                            }
                            return Ok(());
                        }
                        _ => {}
                    }
                    match app.screen {
                        Screen::Main => handle_main_input(&mut app, key),
                        Screen::SetVersion => handle_set_version_input(&mut app, key),
                        Screen::ChannelSelect => handle_channel_input(&mut app, key),
                        Screen::CommitInput => handle_commit_input(&mut app, key),
                        Screen::ConfirmRelease => handle_confirm_input(&mut app, key),
                        Screen::Done => {
                            app.screen = Screen::Main;
                            app.error = None;
                        }
                    }
                }
                event::Event::Resize(_, _) => {}
                _ => {}
            }
        }

        terminal
            .draw(|f| draw(&app, f))
            .map_err(|e| e.to_string())?;
    }
}

fn handle_main_input(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') => {
            // handled above
        }
        KeyCode::Up => {
            if app.selected > 0 {
                app.selected -= 1;
            }
        }
        KeyCode::Down => {
            if app.selected < app.menu.len() - 1 {
                app.selected += 1;
            }
        }
        KeyCode::Enter => match app.selected {
            0 => {
                app.status = format!("Current: {}", app.state.version_string());
            }
            1 => {
                app.version_major = app.state.major.to_string();
                app.version_minor = app.state.minor.to_string();
                app.version_patch = app.state.patch.to_string();
                app.screen = Screen::SetVersion;
            }
            2 => {
                app.sync_channel_index();
                app.screen = Screen::ChannelSelect;
            }
            3 => {
                app.state.bump_build();
                let _ = app.state.save();
                app.status = format!("Build bumped to {}", app.state.build);
            }
            4 => {
                app.commit_input = format!("Release v{}", app.state.version_string());
                app.screen = Screen::CommitInput;
            }
            5 => {
                app.screen = Screen::ConfirmRelease;
            }
            6 => {
                // Quit handled by q
            }
            _ => {}
        },
        _ => {}
    }
}

fn handle_set_version_input(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            app.apply_version_from_inputs();
            app.screen = Screen::Main;
            app.status = format!("Version set to {}", app.state.version_string());
        }
        KeyCode::Esc => {
            app.screen = Screen::Main;
        }
        KeyCode::Tab | KeyCode::Down => {
            app.field_focus = (app.field_focus + 1) % 3;
        }
        KeyCode::Backspace => {
            match app.field_focus {
                0 => { app.version_major.pop(); }
                1 => { app.version_minor.pop(); }
                _ => { app.version_patch.pop(); }
            }
        }
        KeyCode::Char(c) if c.is_ascii_digit() => {
            match app.field_focus {
                0 => { app.version_major.push(c); }
                1 => { app.version_minor.push(c); }
                _ => { app.version_patch.push(c); }
            }
        }
        KeyCode::Char(c) => {
            let _ = c;
        }
        _ => {}
    }
}

fn handle_channel_input(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Up => {
            if app.channel_index > 0 {
                app.channel_index -= 1;
            }
        }
        KeyCode::Down => {
            if app.channel_index < Channel::all().len() - 1 {
                app.channel_index += 1;
            }
        }
        KeyCode::Enter => {
            app.state.channel = match app.channel_index {
                0 => Channel::Alpha,
                2 => Channel::Release,
                _ => Channel::Beta,
            };
            app.state.build = 0;
            let _ = app.state.save();
            app.sync_channel_index();
            app.screen = Screen::Main;
            app.status = format!("Channel set to {}", app.state.channel.as_str());
        }
        KeyCode::Esc => {
            app.screen = Screen::Main;
        }
        _ => {}
    }
}

fn handle_commit_input(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            app.screen = Screen::ConfirmRelease;
        }
        KeyCode::Esc => {
            app.screen = Screen::Main;
        }
        KeyCode::Backspace => {
            app.commit_input.pop();
        }
        KeyCode::Char(c) => {
            app.commit_input.push(c);
        }
        _ => {}
    }
}

fn handle_confirm_input(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('y') | KeyCode::Enter => {
            app.error = None;
            match app.do_release() {
                Ok(()) => {}
                Err(e) => {
                    app.error = Some(e);
                    app.screen = Screen::Main;
                }
            }
        }
        KeyCode::Char('n') | KeyCode::Esc => {
            app.screen = Screen::Main;
        }
        _ => {}
    }
}
