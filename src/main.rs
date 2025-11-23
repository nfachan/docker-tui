use std::time::Duration;

use bollard::{Docker, container::ListContainersOptions};
use color_eyre::eyre::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState},
};
use tokio::{runtime::Runtime, sync::mpsc, time};

#[derive(Debug, Clone)]
struct Container {
    name: String,
    status: String,
}

#[derive(Debug)]
enum AppEvent {
    CrosstermEvent(Event),
    ContainerUpdate(Result<Vec<Container>>),
}

#[derive(Default)]
struct App {
    should_quit: bool,
    containers: Vec<Container>,
    list_state: ListState,
}

impl App {
    fn handle_key_event(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
            }
            KeyCode::Up => {
                let containers_len = self.containers.len();
                if containers_len > 0 {
                    let selected = self.list_state.selected().unwrap_or(0);
                    let new_selected = if selected == 0 {
                        containers_len - 1
                    } else {
                        selected - 1
                    };
                    self.list_state.select(Some(new_selected));
                }
            }
            KeyCode::Down => {
                let containers_len = self.containers.len();
                if containers_len > 0 {
                    let selected = self.list_state.selected().unwrap_or(0);
                    let new_selected = (selected + 1) % containers_len;
                    self.list_state.select(Some(new_selected));
                }
            }
            _ => {}
        }
    }

    fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();

        let block = Block::default()
            .title("Docker Containers")
            .borders(Borders::ALL);

        let items: Vec<ListItem> = self
            .containers
            .iter()
            .map(|container| ListItem::new(format!("{} - {}", container.name, container.status)))
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");

        frame.render_stateful_widget(list, area, &mut self.list_state);
    }
}

async fn fetch_containers() -> Result<Vec<Container>> {
    let docker = Docker::connect_with_socket_defaults()?;
    let options = Some(ListContainersOptions::<String> {
        all: true,
        ..Default::default()
    });

    let container_list = docker.list_containers(options).await?;
    let parsed_containers: Vec<Container> = container_list
        .iter()
        .map(|container| {
            let name = container
                .names
                .as_ref()
                .and_then(|names| names.first())
                .map(|n| n.trim_start_matches('/').to_string())
                .unwrap_or_else(|| "unnamed".to_string());
            let status = container.status.as_deref().unwrap_or("unknown").to_string();
            Container { name, status }
        })
        .collect();

    Ok(parsed_containers)
}

fn run_app() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::default();

    let (tx, mut rx) = mpsc::unbounded_channel();

    // Create tokio Runtime, and put a task on it that periodically gets containers.
    let rt = Runtime::new()?;
    let tx_clone = tx.clone();
    rt.spawn(async move {
        loop {
            let _ = tx_clone.send(AppEvent::ContainerUpdate(fetch_containers().await));
            time::sleep(Duration::from_secs(1)).await;
        }
    });

    // Spawn keyboard event thread
    let tx_clone = tx.clone();
    std::thread::spawn(move || {
        loop {
            if let Ok(event) = event::read() {
                let _ = tx_clone.send(AppEvent::CrosstermEvent(event));
            }
        }
    });

    // Main event loop
    terminal.draw(|frame| app.render(frame))?;
    while let Some(event) = rx.blocking_recv() {
        match event {
            AppEvent::CrosstermEvent(Event::Key(key)) if key.kind == KeyEventKind::Press => {
                app.handle_key_event(key.code);
            }
            AppEvent::CrosstermEvent(_) => {}
            AppEvent::ContainerUpdate(Ok(containers)) => {
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
            AppEvent::ContainerUpdate(Err(err)) => {
                return Err(err)
            }
        }

        if app.should_quit {
            break;
        }
        terminal.draw(|frame| app.render(frame))?;
    }

    // Cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

fn main() -> Result<()> {
    color_eyre::install()?;
    run_app()
}
