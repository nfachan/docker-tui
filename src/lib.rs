use bollard::Docker;
use color_eyre::eyre::{Report, Result};
use crossterm::{
    cursor,
    event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
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
use viewport::Viewport;

mod docker;
mod input;
mod viewport;

#[derive(Debug)]
pub enum Event {
    Input(Result<input::Event>),
    Docker(Result<Vec<Container>>),
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

    fn handle_key_event(&mut self, key_event: KeyEvent) -> ControlFlow<()> {
        match (key_event.code, key_event.modifiers) {
            (KeyCode::Char('q') | KeyCode::Esc, _) => {
                return ControlFlow::Break(());
            }
            (KeyCode::Char('k') | KeyCode::Up, _) => {
                self.viewport.move_selection_up_one_line();
            }
            (KeyCode::Char('j') | KeyCode::Down, _) => {
                self.viewport.move_selection_down_one_line();
            }
            (KeyCode::Char('y'), KeyModifiers::CONTROL) => {
                self.viewport.scroll_up_n_lines(1);
            }
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                self.viewport
                    .scroll_up_n_lines(cmp::max(1, self.viewport.height() / 2));
            }
            (KeyCode::Char('b'), KeyModifiers::CONTROL) => {
                self.viewport
                    .scroll_up_n_lines(cmp::max(1, self.viewport.height().saturating_sub(2)));
            }
            (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
                self.viewport.scroll_down_n_lines(1);
            }
            (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                self.viewport
                    .scroll_down_n_lines(cmp::max(1, self.viewport.height() / 2));
            }
            (KeyCode::Char('f'), KeyModifiers::CONTROL) => {
                self.viewport
                    .scroll_down_n_lines(cmp::max(1, self.viewport.height().saturating_sub(2)));
            }
            _ => {}
        }
        ControlFlow::Continue(())
    }

    fn handle_resize(&mut self, height: u16) {
        let viewport_height = Self::block().inner(Rect::new(0, 0, 1, height)).height;
        self.viewport.change_viewport_height(viewport_height.into());
    }

    fn handle_containers(&mut self, containers: Vec<Container>) {
        self.containers = containers;
        self.viewport.change_num_containers(self.containers.len());
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
            if self.viewport.height() != usize::from(inner_area.height) {
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
                if let ControlFlow::Break(_) = app.handle_key_event(key) {
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
