use std::io::{self, Stdout};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Cell, Clear, List, ListItem, ListState, Paragraph, Row, Table, TableState,
};
use ratatui::Terminal;

use crate::config::{self, ProjectConfig};
use crate::vault::Vault;

// ---------------------------------------------------------------------------
// Drop guard: ensures the terminal is restored even on panic
// ---------------------------------------------------------------------------

struct TerminalGuard;

impl TerminalGuard {
    fn init() -> Result<Terminal<CrosstermBackend<Stdout>>> {
        enable_raw_mode().context("failed to enable raw mode")?;
        let mut stdout = io::stdout();
        stdout
            .execute(EnterAlternateScreen)
            .context("failed to enter alternate screen")?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(terminal)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = io::stdout().execute(LeaveAlternateScreen);
    }
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputMode {
    Normal,
    Editing,
    Adding,
    Searching,
    Confirming,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FocusPanel {
    Environments,
    Variables,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AddField {
    Key,
    Value,
    Secret,
}

struct App {
    vault: Vault,
    config: ProjectConfig,
    environments: Vec<String>,
    selected_env: usize,
    variables: Vec<(String, String, bool)>, // key, value, is_secret
    selected_var: usize,
    focus: FocusPanel,
    input_mode: InputMode,
    input_buffer: String,
    input_field: Option<String>, // which field is being edited
    search_query: String,
    show_values: bool,
    message: Option<String>,
    message_time: Option<Instant>,
    should_quit: bool,

    // List / table widget state
    env_list_state: ListState,
    var_table_state: TableState,

    // Adding flow: key, value, secret
    add_key: String,
    add_value: String,
    add_secret: bool,
    add_field: AddField,
}

impl App {
    fn new(vault: Vault, config: ProjectConfig) -> Self {
        let environments = vault.environments();
        let environments = if environments.is_empty() {
            vec!["dev".to_string(), "staging".to_string(), "prod".to_string()]
        } else {
            environments
        };

        let variables = vault.list(&environments[0]).unwrap_or_default();

        let mut env_list_state = ListState::default();
        env_list_state.select(Some(0));

        let mut var_table_state = TableState::default();
        if !variables.is_empty() {
            var_table_state.select(Some(0));
        }

        Self {
            vault,
            config,
            environments,
            selected_env: 0,
            variables,
            selected_var: 0,
            focus: FocusPanel::Variables,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            input_field: None,
            search_query: String::new(),
            show_values: false,
            message: None,
            message_time: None,
            should_quit: false,
            env_list_state,
            var_table_state,
            add_key: String::new(),
            add_value: String::new(),
            add_secret: false,
            add_field: AddField::Key,
        }
    }

    // ------------------------------------------------------------------
    // Data helpers
    // ------------------------------------------------------------------

    fn current_env(&self) -> &str {
        &self.environments[self.selected_env]
    }

    fn refresh_variables(&mut self) {
        let env = self.current_env().to_string();
        let all = self.vault.list(&env).unwrap_or_default();
        if self.search_query.is_empty() {
            self.variables = all;
        } else {
            let q = self.search_query.to_lowercase();
            self.variables = all
                .into_iter()
                .filter(|(k, v, _)| k.to_lowercase().contains(&q) || v.to_lowercase().contains(&q))
                .collect();
        }
        // Clamp selected_var
        if self.variables.is_empty() {
            self.selected_var = 0;
            self.var_table_state.select(None);
        } else if self.selected_var >= self.variables.len() {
            self.selected_var = self.variables.len() - 1;
            self.var_table_state.select(Some(self.selected_var));
        } else {
            self.var_table_state.select(Some(self.selected_var));
        }
    }

    fn set_message(&mut self, msg: impl Into<String>) {
        self.message = Some(msg.into());
        self.message_time = Some(Instant::now());
    }

    fn tick_message(&mut self) {
        if let Some(t) = self.message_time {
            if t.elapsed() > Duration::from_secs(3) {
                self.message = None;
                self.message_time = None;
            }
        }
    }

    fn select_env(&mut self, idx: usize) {
        if idx < self.environments.len() {
            self.selected_env = idx;
            self.env_list_state.select(Some(idx));
            self.selected_var = 0;
            self.refresh_variables();
        }
    }

    // ------------------------------------------------------------------
    // Event handling
    // ------------------------------------------------------------------

    fn handle_event(&mut self, key: KeyEvent) {
        match self.input_mode {
            InputMode::Normal => self.handle_normal(key),
            InputMode::Editing => self.handle_editing(key),
            InputMode::Adding => self.handle_adding(key),
            InputMode::Searching => self.handle_searching(key),
            InputMode::Confirming => self.handle_confirming(key),
        }
    }

    fn handle_normal(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
            }
            KeyCode::Tab => {
                self.focus = match self.focus {
                    FocusPanel::Environments => FocusPanel::Variables,
                    FocusPanel::Variables => FocusPanel::Environments,
                };
            }
            KeyCode::Char('j') | KeyCode::Down => self.move_down(),
            KeyCode::Char('k') | KeyCode::Up => self.move_up(),
            KeyCode::Char('a') => {
                self.input_mode = InputMode::Adding;
                self.add_key.clear();
                self.add_value.clear();
                self.add_secret = false;
                self.add_field = AddField::Key;
                self.input_buffer.clear();
            }
            KeyCode::Char('d') => {
                if !self.variables.is_empty() {
                    self.input_mode = InputMode::Confirming;
                }
            }
            KeyCode::Char('e') => {
                if !self.variables.is_empty() && self.focus == FocusPanel::Variables {
                    let (_, val, _) = &self.variables[self.selected_var];
                    self.input_buffer = val.clone();
                    self.input_field = Some("value".to_string());
                    self.input_mode = InputMode::Editing;
                }
            }
            KeyCode::Char('s') => {
                if !self.variables.is_empty() && self.focus == FocusPanel::Variables {
                    let (key, value, secret) = &self.variables[self.selected_var];
                    let env = self.current_env().to_string();
                    let new_secret = !secret;
                    let _ = self.vault.set(&env, key, value, new_secret);
                    self.set_message(if new_secret {
                        format!("Marked '{}' as secret", key)
                    } else {
                        format!("Unmarked '{}' as secret", key)
                    });
                    self.refresh_variables();
                }
            }
            KeyCode::Char('/') => {
                self.input_mode = InputMode::Searching;
                self.input_buffer = self.search_query.clone();
            }
            KeyCode::Enter => {
                self.show_values = !self.show_values;
            }
            KeyCode::Char('r') => {
                // Reload vault from disk
                if let Ok(v) = Vault::load(&self.config) {
                    self.vault = v;
                    self.environments = self.vault.environments();
                    if self.environments.is_empty() {
                        self.environments =
                            vec!["dev".to_string(), "staging".to_string(), "prod".to_string()];
                    }
                    if self.selected_env >= self.environments.len() {
                        self.selected_env = 0;
                        self.env_list_state.select(Some(0));
                    }
                    self.refresh_variables();
                    self.set_message("Refreshed from vault");
                }
            }
            // Quick-switch environments: 1-9
            KeyCode::Char(c @ '1'..='9') => {
                let idx = (c as usize) - ('1' as usize);
                self.select_env(idx);
            }
            _ => {}
        }
    }

    fn handle_editing(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.input_buffer.clear();
                self.input_field = None;
            }
            KeyCode::Enter => {
                let env = self.current_env().to_string();
                let (k, _, secret) = &self.variables[self.selected_var];
                let k = k.clone();
                let secret = *secret;
                let new_value = self.input_buffer.clone();
                if let Err(e) = self.vault.set(&env, &k, &new_value, secret) {
                    self.set_message(format!("Error: {e}"));
                } else {
                    self.set_message(format!("Updated '{}'", k));
                }
                self.input_mode = InputMode::Normal;
                self.input_buffer.clear();
                self.input_field = None;
                self.refresh_variables();
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
            }
            KeyCode::Char(c) => {
                self.input_buffer.push(c);
            }
            _ => {}
        }
    }

    fn handle_adding(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.input_buffer.clear();
            }
            KeyCode::Tab => {
                // Cycle through add fields
                match self.add_field {
                    AddField::Key => {
                        self.add_key = self.input_buffer.clone();
                        self.input_buffer = self.add_value.clone();
                        self.add_field = AddField::Value;
                    }
                    AddField::Value => {
                        self.add_value = self.input_buffer.clone();
                        self.input_buffer.clear();
                        self.add_field = AddField::Secret;
                    }
                    AddField::Secret => {
                        self.input_buffer = self.add_key.clone();
                        self.add_field = AddField::Key;
                    }
                }
            }
            KeyCode::Enter => {
                // Save current field buffer
                match self.add_field {
                    AddField::Key => self.add_key = self.input_buffer.clone(),
                    AddField::Value => self.add_value = self.input_buffer.clone(),
                    AddField::Secret => {}
                }

                if self.add_key.is_empty() {
                    self.set_message("Key cannot be empty");
                    return;
                }

                let env = self.current_env().to_string();
                let key = self.add_key.clone();
                let value = self.add_value.clone();
                let secret = self.add_secret;
                if let Err(e) = self.vault.set(&env, &key, &value, secret) {
                    self.set_message(format!("Error: {e}"));
                } else {
                    self.set_message(format!("Added '{}'", key));
                }
                self.input_mode = InputMode::Normal;
                self.input_buffer.clear();
                self.refresh_variables();
            }
            KeyCode::Char(' ') if self.add_field == AddField::Secret => {
                self.add_secret = !self.add_secret;
            }
            KeyCode::Backspace => {
                if self.add_field != AddField::Secret {
                    self.input_buffer.pop();
                }
            }
            KeyCode::Char(c) => {
                if self.add_field != AddField::Secret {
                    self.input_buffer.push(c);
                }
            }
            _ => {}
        }
    }

    fn handle_searching(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.search_query.clear();
                self.input_buffer.clear();
                self.refresh_variables();
            }
            KeyCode::Enter => {
                self.search_query = self.input_buffer.clone();
                self.input_mode = InputMode::Normal;
                self.refresh_variables();
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
                self.search_query = self.input_buffer.clone();
                self.refresh_variables();
            }
            KeyCode::Char(c) => {
                self.input_buffer.push(c);
                self.search_query = self.input_buffer.clone();
                self.refresh_variables();
            }
            _ => {}
        }
    }

    fn handle_confirming(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                if !self.variables.is_empty() {
                    let env = self.current_env().to_string();
                    let (k, _, _) = &self.variables[self.selected_var];
                    let k = k.clone();
                    if let Err(e) = self.vault.remove(&env, &k) {
                        self.set_message(format!("Error: {e}"));
                    } else {
                        self.set_message(format!("Deleted '{}'", k));
                    }
                    self.refresh_variables();
                }
                self.input_mode = InputMode::Normal;
            }
            _ => {
                self.input_mode = InputMode::Normal;
            }
        }
    }

    fn move_down(&mut self) {
        match self.focus {
            FocusPanel::Environments => {
                if !self.environments.is_empty() {
                    let next = (self.selected_env + 1) % self.environments.len();
                    self.select_env(next);
                }
            }
            FocusPanel::Variables => {
                if !self.variables.is_empty() {
                    self.selected_var = (self.selected_var + 1) % self.variables.len();
                    self.var_table_state.select(Some(self.selected_var));
                }
            }
        }
    }

    fn move_up(&mut self) {
        match self.focus {
            FocusPanel::Environments => {
                if !self.environments.is_empty() {
                    let next = if self.selected_env == 0 {
                        self.environments.len() - 1
                    } else {
                        self.selected_env - 1
                    };
                    self.select_env(next);
                }
            }
            FocusPanel::Variables => {
                if !self.variables.is_empty() {
                    self.selected_var = if self.selected_var == 0 {
                        self.variables.len() - 1
                    } else {
                        self.selected_var - 1
                    };
                    self.var_table_state.select(Some(self.selected_var));
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Rendering
    // ------------------------------------------------------------------

    fn draw(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        terminal.draw(|frame| {
            let size = frame.area();

            // Main vertical layout: top bar | body | status | bottom bar
            let outer = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // top bar
                    Constraint::Min(5),    // body
                    Constraint::Length(1), // status message
                    Constraint::Length(3), // help bar
                ])
                .split(size);

            // -- Top bar --
            let project_name = self
                .config
                .project_root
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "envsafe".to_string());
            let env_name = self.current_env().to_string();
            let top_text = Line::from(vec![
                Span::styled(
                    format!(" {} ", project_name),
                    Style::default()
                        .fg(Color::White)
                        .bg(Color::Blue)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled(
                    format!(" {} ", env_name),
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled(
                    format!("{} vars", self.variables.len()),
                    Style::default().fg(Color::DarkGray),
                ),
                if !self.search_query.is_empty() {
                    Span::styled(
                        format!("  filter: \"{}\"", self.search_query),
                        Style::default().fg(Color::Yellow),
                    )
                } else {
                    Span::raw("")
                },
            ]);
            let top_bar = Paragraph::new(top_text).block(
                Block::default()
                    .borders(Borders::BOTTOM)
                    .title(" envsafe ")
                    .title_style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
            );
            frame.render_widget(top_bar, outer[0]);

            // -- Body: env list | variable table --
            let body = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(20), Constraint::Min(40)])
                .split(outer[1]);

            self.draw_env_list(frame, body[0]);
            self.draw_var_table(frame, body[1]);

            // -- Status message --
            let status_msg = if let Some(ref msg) = self.message {
                Span::styled(
                    format!(" {} ", msg),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::raw("")
            };
            let status = Paragraph::new(Line::from(status_msg));
            frame.render_widget(status, outer[2]);

            // -- Bottom help bar --
            let help = self.help_text();
            let help_bar = Paragraph::new(help).block(
                Block::default()
                    .borders(Borders::TOP)
                    .title(" Keybindings ")
                    .title_style(Style::default().fg(Color::DarkGray)),
            );
            frame.render_widget(help_bar, outer[3]);

            // -- Overlays --
            match self.input_mode {
                InputMode::Confirming => {
                    let var_name = if !self.variables.is_empty() {
                        self.variables[self.selected_var].0.clone()
                    } else {
                        String::new()
                    };
                    self.draw_confirm_popup(frame, size, &var_name);
                }
                InputMode::Adding => {
                    self.draw_add_popup(frame, size);
                }
                InputMode::Editing => {
                    self.draw_edit_popup(frame, size);
                }
                InputMode::Searching => {
                    self.draw_search_bar(frame, size);
                }
                InputMode::Normal => {}
            }
        })?;
        Ok(())
    }

    fn draw_env_list(&mut self, frame: &mut ratatui::Frame, area: Rect) {
        let env_style = if self.focus == FocusPanel::Environments {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let items: Vec<ListItem> = self
            .environments
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let prefix = if i == self.selected_env { "> " } else { "  " };
                let style = if i == self.selected_env {
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                ListItem::new(format!("{}{}", prefix, name)).style(style)
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Environments ")
                .title_style(env_style)
                .border_style(env_style),
        );
        frame.render_stateful_widget(list, area, &mut self.env_list_state);
    }

    fn draw_var_table(&mut self, frame: &mut ratatui::Frame, area: Rect) {
        let var_style = if self.focus == FocusPanel::Variables {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let header = Row::new(vec![
            Cell::from("KEY").style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::White),
            ),
            Cell::from("VALUE").style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::White),
            ),
            Cell::from("SECRET?").style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::White),
            ),
        ])
        .height(1)
        .bottom_margin(1);

        let rows: Vec<Row> = self
            .variables
            .iter()
            .enumerate()
            .map(|(i, (key, value, secret))| {
                let display_value = if *secret && !self.show_values {
                    "********".to_string()
                } else {
                    value.clone()
                };

                let secret_marker = if *secret { "Yes" } else { "No" };

                let style = if i == self.selected_var && self.focus == FocusPanel::Variables {
                    Style::default().bg(Color::DarkGray).fg(Color::White)
                } else {
                    Style::default()
                };

                Row::new(vec![
                    Cell::from(key.clone()).style(Style::default().fg(Color::Yellow)),
                    Cell::from(display_value),
                    Cell::from(secret_marker).style(if *secret {
                        Style::default().fg(Color::Red)
                    } else {
                        Style::default().fg(Color::Green)
                    }),
                ])
                .style(style)
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Percentage(30),
                Constraint::Percentage(50),
                Constraint::Percentage(20),
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    " Variables [{}] ",
                    if self.show_values {
                        "revealed"
                    } else {
                        "masked"
                    }
                ))
                .title_style(var_style)
                .border_style(var_style),
        )
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

        frame.render_stateful_widget(table, area, &mut self.var_table_state);
    }

    fn draw_confirm_popup(&self, frame: &mut ratatui::Frame, area: Rect, var_name: &str) {
        let popup_area = centered_rect(50, 7, area);
        frame.render_widget(Clear, popup_area);
        let popup = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("Delete '{}'?", var_name),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Press 'y' to confirm, any other key to cancel",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Confirm Delete ")
                .title_style(Style::default().fg(Color::Red))
                .border_style(Style::default().fg(Color::Red)),
        )
        .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(popup, popup_area);
    }

    fn draw_add_popup(&self, frame: &mut ratatui::Frame, area: Rect) {
        let popup_area = centered_rect(60, 12, area);
        frame.render_widget(Clear, popup_area);

        let key_style = if self.add_field == AddField::Key {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let val_style = if self.add_field == AddField::Value {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let sec_style = if self.add_field == AddField::Secret {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let key_display = if self.add_field == AddField::Key {
            &self.input_buffer
        } else {
            &self.add_key
        };
        let val_display = if self.add_field == AddField::Value {
            &self.input_buffer
        } else {
            &self.add_value
        };
        let sec_display = if self.add_secret { "[x]" } else { "[ ]" };

        let text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("  Key:    ", key_style),
                Span::styled(
                    format!("{}_", key_display),
                    if self.add_field == AddField::Key {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    },
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Value:  ", val_style),
                Span::styled(
                    format!("{}_", val_display),
                    if self.add_field == AddField::Value {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    },
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Secret: ", sec_style),
                Span::styled(
                    sec_display,
                    if self.add_field == AddField::Secret {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    },
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "  Tab: next field | Space: toggle secret | Enter: save | Esc: cancel",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        let popup = Paragraph::new(text).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Add Variable ")
                .title_style(Style::default().fg(Color::Green))
                .border_style(Style::default().fg(Color::Green)),
        );
        frame.render_widget(popup, popup_area);
    }

    fn draw_edit_popup(&self, frame: &mut ratatui::Frame, area: Rect) {
        let popup_area = centered_rect(60, 7, area);
        frame.render_widget(Clear, popup_area);

        let var_name = if !self.variables.is_empty() {
            self.variables[self.selected_var].0.clone()
        } else {
            String::new()
        };

        let text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("  Editing: ", Style::default().fg(Color::DarkGray)),
                Span::styled(var_name, Style::default().fg(Color::Yellow)),
            ]),
            Line::from(vec![
                Span::styled("  Value:   ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    format!("{}_", self.input_buffer),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(Span::styled(
                "  Enter: save | Esc: cancel",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        let popup = Paragraph::new(text).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Edit Variable ")
                .title_style(Style::default().fg(Color::Yellow))
                .border_style(Style::default().fg(Color::Yellow)),
        );
        frame.render_widget(popup, popup_area);
    }

    fn draw_search_bar(&self, frame: &mut ratatui::Frame, area: Rect) {
        // Search bar at the bottom of the body, just above status
        let search_area = Rect {
            x: area.x + 1,
            y: area.y + area.height.saturating_sub(7),
            width: area.width.saturating_sub(2),
            height: 3,
        };
        frame.render_widget(Clear, search_area);

        let text = Line::from(vec![
            Span::styled(
                " / ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{}_", self.input_buffer),
                Style::default().fg(Color::White),
            ),
        ]);

        let bar = Paragraph::new(text).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Search ")
                .title_style(Style::default().fg(Color::Cyan))
                .border_style(Style::default().fg(Color::Cyan)),
        );
        frame.render_widget(bar, search_area);
    }

    fn help_text(&self) -> Line<'static> {
        let bindings: Vec<(&str, &str)> = match self.input_mode {
            InputMode::Normal => vec![
                ("q/Esc", "Quit"),
                ("Tab", "Switch panel"),
                ("j/k", "Navigate"),
                ("a", "Add"),
                ("e", "Edit"),
                ("d", "Delete"),
                ("s", "Toggle secret"),
                ("/", "Search"),
                ("Enter", "Reveal/mask"),
                ("r", "Refresh"),
                ("1-9", "Switch env"),
            ],
            InputMode::Editing => vec![("Enter", "Save"), ("Esc", "Cancel")],
            InputMode::Adding => vec![
                ("Tab", "Next field"),
                ("Space", "Toggle secret"),
                ("Enter", "Save"),
                ("Esc", "Cancel"),
            ],
            InputMode::Searching => vec![("Enter", "Apply"), ("Esc", "Clear & close")],
            InputMode::Confirming => vec![("y", "Confirm"), ("any", "Cancel")],
        };

        let spans: Vec<Span<'static>> = bindings
            .iter()
            .flat_map(|(key, desc)| {
                vec![
                    Span::styled(
                        format!(" {} ", key),
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::DarkGray)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(format!(" {} ", desc), Style::default().fg(Color::White)),
                    Span::raw(" "),
                ]
            })
            .collect();

        Line::from(spans)
    }
}

// ---------------------------------------------------------------------------
// Utility: centered rectangle
// ---------------------------------------------------------------------------

fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let width = area.width * percent_x / 100;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect {
        x,
        y,
        width,
        height: height.min(area.height),
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn run_tui() -> Result<()> {
    let config = config::find_project_root()?;
    let vault = Vault::load(&config)?;

    // Install a panic hook that restores the terminal
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = io::stdout().execute(LeaveAlternateScreen);
        original_hook(info);
    }));

    let _guard = TerminalGuard;
    let mut terminal = TerminalGuard::init()?;
    let mut app = App::new(vault, config);

    loop {
        app.tick_message();
        app.draw(&mut terminal)?;

        // Poll for events with a timeout so the status message can expire
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                // Ignore key release events on Windows (crossterm sends both press and release)
                if key.kind == crossterm::event::KeyEventKind::Press {
                    app.handle_event(key);
                }
            }
            // Resize is handled automatically by ratatui on next draw
        }

        if app.should_quit {
            break;
        }
    }

    // _guard Drop will restore terminal
    Ok(())
}
