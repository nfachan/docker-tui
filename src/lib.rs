use bollard::Docker;
use color_eyre::eyre::{Report, Result};
use crossterm::{
    cursor,
    event::{KeyCode, KeyEventKind},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use docker::Container;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState},
};
use std::{cmp, io, ops::ControlFlow, thread};
use tokio::sync::mpsc;

mod docker;
mod input;

#[derive(Debug)]
pub enum Event {
    Input(Result<input::Event>),
    Docker(Result<Vec<Container>>),
}

#[derive(Default)]
struct App {
    containers: Vec<Container>,
    list_state: ListState,
}

impl App {
    fn handle_up(&mut self) {
        let selected = self.list_state.selected().unwrap_or(0);
        if !self.containers.is_empty() {
            self.list_state.select(Some(selected.saturating_sub(1)));
        }
    }

    fn handle_down(&mut self) {
        let selected = self.list_state.selected().unwrap_or(0);
        if !self.containers.is_empty() {
            self.list_state.select(Some(cmp::min(
                selected.saturating_add(1),
                self.containers.len() - 1,
            )));
        }
    }

    fn handle_key_event(&mut self, key: KeyCode) -> ControlFlow<()> {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => {
                return ControlFlow::Break(());
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.handle_up();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.handle_down();
            }
            _ => {}
        }
        ControlFlow::Continue(())
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let items = self
            .containers
            .iter()
            .map(|container| ListItem::new(format!("{} - {}", container.name, container.status)));
        StatefulWidget::render(
            List::new(items)
                .block(
                    Block::default()
                        .title("Docker Containers")
                        .borders(Borders::ALL),
                )
                .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                .highlight_symbol("> "),
            area,
            buf,
            &mut self.list_state,
        );
    }
}

fn main_loop(docker: Docker, mut terminal: Terminal<impl Backend>) -> Result<()> {
    const CHANNEL_SLOTS: usize = 10;
    let (sender, mut receiver) = mpsc::channel(CHANNEL_SLOTS);

    let sender_clone = sender.clone();
    thread::spawn(move || docker::main(docker, sender_clone));

    // Spawn input event thread.
    thread::spawn(move || input::main(sender));
    let mut app = App::default();

    terminal.draw(|frame| frame.render_widget(&mut app, frame.area()))?;
    while let Some(event) = receiver.blocking_recv() {
        match event {
            Event::Input(Ok(input::Event::Key(key))) if key.kind == KeyEventKind::Press => {
                if let ControlFlow::Break(_) = app.handle_key_event(key.code) {
                    break;
                }
            }
            Event::Input(Ok(_)) => {}
            Event::Docker(Ok(containers)) => {
                let current_selection = app.list_state.selected();
                app.containers = containers;

                // Maintain selection if possible, otherwise select first item
                if app.containers.is_empty() {
                    app.list_state.select(None);
                } else if current_selection.is_none()
                    || current_selection.unwrap() >= app.containers.len()
                {
                    app.list_state.select(Some(0));
                }
            }
            Event::Input(Err(err)) | Event::Docker(Err(err)) => {
                return Err(err);
            }
        }
        terminal.draw(|frame| frame.render_widget(&mut app, frame.area()))?;
    }
    Ok(())
}

fn main_start_up(docker: Docker) -> Result<()> {
    let mut stdout = io::stdout();

    terminal::enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen)?;
    execute!(stdout, cursor::Hide)?;

    main_loop(docker, Terminal::new(CrosstermBackend::new(stdout))?)
}

fn main_clean_up() -> Result<()> {
    let mut stdout = io::stdout();
    [
        execute!(stdout, cursor::Show),
        execute!(stdout, LeaveAlternateScreen),
        terminal::disable_raw_mode(),
    ]
    .into_iter()
    .collect::<Result<(), _>>()
    .map_err(Report::from)
}

pub fn main(docker: Docker) -> Result<()> {
    [main_start_up(docker), main_clean_up()]
        .into_iter()
        .collect()
}
