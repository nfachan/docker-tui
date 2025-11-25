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
    widgets::{Block, List, ListItem, Paragraph},
};
use std::{cmp, io, ops::ControlFlow, panic, thread};
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
    selection: usize,
    viewport_top: usize,
    viewport_height: usize,
}

impl App {
    fn block() -> Block<'static> {
        Block::bordered().title("Containers")
    }

    fn validate_selection_and_viewport(&self) {
        let num_containers = self.containers.len();
        if num_containers == 0 {
            assert_eq!(self.selection, 0);
            assert_eq!(self.viewport_top, 0);
        } else if self.viewport_height == 0 {
            assert!(self.selection < num_containers);
            assert_eq!(self.viewport_top, self.selection);
        } else {
            assert!(self.selection < num_containers);
            assert!(self.viewport_top <= self.selection);
            assert!(self.selection < self.viewport_top + self.viewport_height);
            assert!(
                self.viewport_top + self.viewport_height <= num_containers
                    || self.viewport_top == 0
            );
        }
    }

    fn handle_up(&mut self) {
        self.selection = self.selection.saturating_sub(1);
        if self.viewport_top > self.selection {
            self.viewport_top -= 1;
        }
        self.validate_selection_and_viewport();
    }

    fn handle_down(&mut self) {
        let num_containers = self.containers.len();
        self.selection = cmp::min(num_containers.saturating_sub(1), self.selection + 1);
        if self.viewport_height == 0 {
            self.viewport_top = self.selection;
        } else if self.selection >= self.viewport_top + self.viewport_height {
            self.viewport_top += 1;
        }
        self.validate_selection_and_viewport();
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

    fn handle_resize(&mut self, height: u16) {
        let old_height = self.viewport_height;
        self.viewport_height = usize::from(Self::block().inner(Rect::new(0, 0, 1, height)).height);
        let num_containers = self.containers.len();
        if num_containers > 0 {
            if self.viewport_height < old_height {
                if self.viewport_height == 0 {
                    self.viewport_top = self.selection;
                } else {
                    self.viewport_top += self
                        .selection
                        .saturating_sub(self.viewport_top + self.viewport_height - 1);
                }
            } else if self.viewport_height > old_height {
                self.viewport_top = self.viewport_top.saturating_sub(
                    (self.viewport_top + self.viewport_height).saturating_sub(num_containers),
                );
            }
        }
        self.validate_selection_and_viewport();
    }

    fn handle_containers(&mut self, containers: Vec<Container>) {
        self.containers = containers;
        let num_containers = self.containers.len();

        if num_containers == 0 {
            self.selection = 0;
            self.viewport_top = 0;
        } else {
            if self.selection >= num_containers {
                self.selection = num_containers - 1;
            }
            self.viewport_top = self.viewport_top.saturating_sub(
                (self.viewport_top + self.viewport_height).saturating_sub(num_containers),
            );
        }
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let inner_area = App::block().inner(area);
        let num_containers = self.containers.len();
        if self.containers.is_empty() {
            Paragraph::new("no containers")
                .block(App::block())
                .render(area, buf);
        } else {
            if self.viewport_height != usize::from(inner_area.height) {
                self.handle_resize(area.height);
            }

            let empty_rows =
                (self.viewport_top + self.viewport_height).saturating_sub(num_containers);
            let container_rows = self.viewport_height - empty_rows;
            let selection_offset_in_viewport = self.selection - self.viewport_top;
            let items = self.containers[self.viewport_top..self.viewport_top + container_rows]
                .iter()
                .enumerate()
                .map(|(offset, container)| {
                    let prefix = if offset == selection_offset_in_viewport {
                        "> "
                    } else {
                        "  "
                    };
                    let mut item = ListItem::new(format!(
                        "{}{} - {}",
                        prefix, container.name, container.status
                    ));
                    if offset == selection_offset_in_viewport {
                        item = item.add_modifier(Modifier::REVERSED);
                    }
                    item
                });
            Widget::render(List::new(items).block(App::block()), area, buf);
        }
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
                app.handle_containers(containers);
            }
            Event::Input(Err(err)) | Event::Docker(Err(err)) => {
                return Err(err);
            }
        }
        terminal.draw(|frame| frame.render_widget(&mut app, frame.area()))?;
    }
    Ok(())
}

fn set_panic_hook() {
    let hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = main_clean_up(); // ignore any errors as we are already failing
        hook(panic_info);
    }));
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
    set_panic_hook();
    [main_start_up(docker), main_clean_up()]
        .into_iter()
        .collect()
}
