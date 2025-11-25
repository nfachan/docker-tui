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
struct Viewport {
    num_containers: usize,
    selection: usize,
    top: usize,
    height: usize,
}

impl Viewport {
    fn validate(&self) {
        if self.num_containers == 0 {
            assert_eq!(self.selection, 0);
            assert_eq!(self.top, 0);
        } else if self.height == 0 {
            assert!(self.selection < self.num_containers);
            assert_eq!(self.top, self.selection);
        } else {
            assert!(self.selection < self.num_containers);
            assert!(self.top <= self.selection);
            assert!(self.selection < self.top + self.height);
            assert!(self.top + self.height <= self.num_containers || self.top == 0);
        }
    }

    fn handle_up(&mut self) {
        self.selection = self.selection.saturating_sub(1);
        if self.top > self.selection {
            self.top -= 1;
        }
        self.validate();
    }

    fn handle_down(&mut self) {
        self.selection = cmp::min(self.num_containers.saturating_sub(1), self.selection + 1);
        if self.height == 0 {
            self.top = self.selection;
        } else if self.selection >= self.top + self.height {
            self.top += 1;
        }
        self.validate();
    }

    fn handle_resize(&mut self, height: usize) {
        let old_height = self.height;
        self.height = height;
        if self.num_containers > 0 {
            if self.height < old_height {
                if self.height == 0 {
                    self.top = self.selection;
                } else {
                    self.top += self.selection.saturating_sub(self.top + self.height - 1);
                }
            } else if self.height > old_height {
                self.top = self
                    .top
                    .saturating_sub((self.top + self.height).saturating_sub(self.num_containers));
            }
        }
        self.validate();
    }

    fn handle_num_containers(&mut self, num_containers: usize) {
        self.num_containers = num_containers;

        if num_containers == 0 {
            self.selection = 0;
            self.top = 0;
        } else {
            if self.selection >= num_containers {
                self.selection = num_containers - 1;
            }
            self.top = self
                .top
                .saturating_sub((self.top + self.height).saturating_sub(num_containers));
        }
        self.validate();
    }

    fn select_for_render<C, LI, F, G>(
        &self,
        containers: &[C],
        f: F,
        g: G,
    ) -> impl Iterator<Item = LI>
    where
        F: Fn(&C) -> LI,
        G: Fn(&C) -> LI,
    {
        assert_eq!(containers.len(), self.num_containers);
        let empty_rows = (self.top + self.height).saturating_sub(self.num_containers);
        let container_rows = self.height - empty_rows;
        let selection_offset_in_viewport = self.selection - self.top;
        containers[self.top..self.top + container_rows]
            .iter()
            .enumerate()
            .map(move |(offset, container)| {
                if offset == selection_offset_in_viewport {
                    g(container)
                } else {
                    f(container)
                }
            })
    }
}

#[derive(Default)]
struct App {
    containers: Vec<Container>,
    viewport: Viewport,
}

impl App {
    fn block() -> Block<'static> {
        Block::bordered().title("Containers")
    }

    fn handle_key_event(&mut self, key: KeyCode) -> ControlFlow<()> {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => {
                return ControlFlow::Break(());
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.viewport.handle_up();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.viewport.handle_down();
            }
            _ => {}
        }
        ControlFlow::Continue(())
    }

    fn handle_resize(&mut self, height: u16) {
        let viewport_height = Self::block().inner(Rect::new(0, 0, 1, height)).height;
        self.viewport.handle_resize(viewport_height.into());
    }

    fn handle_containers(&mut self, containers: Vec<Container>) {
        self.containers = containers;
        self.viewport.handle_num_containers(self.containers.len());
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let inner_area = App::block().inner(area);
        if self.containers.is_empty() {
            Paragraph::new("no containers")
                .block(App::block())
                .render(area, buf);
        } else {
            // We may not always get the resize event before some other event that causes a redraw.
            if self.viewport.height != usize::from(inner_area.height) {
                self.handle_resize(area.height);
            }

            let items = self.viewport.select_for_render(
                &self.containers[..],
                |container| ListItem::new(format!("  {} - {}", container.name, container.status)),
                |container| {
                    ListItem::new(format!("> {} - {}", container.name, container.status))
                        .add_modifier(Modifier::REVERSED)
                },
            );
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
