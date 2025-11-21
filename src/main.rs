use std::time::Duration;

use bollard::{Docker, container::ListContainersOptions};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState},
};
use tokio::{sync::mpsc, time::interval};

#[derive(Debug, Clone)]
struct Container {
    name: String,
    status: String,
}

struct App {
    should_quit: bool,
    containers: Vec<Container>,
    list_state: ListState,
}

impl App {
    fn new(containers: Vec<Container>) -> Self {
        let mut list_state = ListState::default();
        if !containers.is_empty() {
            list_state.select(Some(0));
        }
        Self {
            should_quit: false,
            containers,
            list_state,
        }
    }

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

async fn fetch_containers() -> Result<Vec<Container>, Box<dyn std::error::Error + Send + Sync>> {
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

async fn run_app() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Initial fetch
    let initial_containers = fetch_containers().await.unwrap_or_else(|_| Vec::new());
    let mut app = App::new(initial_containers);

    // Create channels for communication
    let (tx, mut rx) = mpsc::unbounded_channel::<Vec<Container>>();

    // Spawn background task to fetch containers periodically
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(2));
        loop {
            interval.tick().await;
            match fetch_containers().await {
                Ok(containers) => {
                    let _ = tx.send(containers);
                }
                Err(e) => {
                    eprintln!("Error fetching containers: {}", e);
                }
            }
        }
    });

    // Main event loop
    let mut last_tick = std::time::Instant::now();
    let tick_rate = Duration::from_millis(50);

    loop {
        let timeout = tick_rate.saturating_sub(last_tick.elapsed());

        tokio::select! {
            // Handle terminal events
            result = tokio::time::timeout(timeout, async {
                if event::poll(Duration::from_millis(0))? {
                    event::read()
                } else {
                    Ok(Event::FocusGained)  // Dummy event
                }
            }) => {
                if let Ok(Ok(event)) = result {
                    if let Event::Key(key) = event {
                        if key.kind == KeyEventKind::Press {
                            app.handle_key_event(key.code);
                        }
                    }
                }
            }

            // Handle container updates
            Some(new_containers) = rx.recv() => {
                let current_selection = app.list_state.selected();
                app.containers = new_containers;

                // Maintain selection if possible, otherwise select first item
                if app.containers.is_empty() {
                    app.list_state.select(None);
                } else if current_selection.is_none() || current_selection.unwrap() >= app.containers.len() {
                    app.list_state.select(Some(0));
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            terminal.draw(|frame| app.render(frame))?;
            last_tick = std::time::Instant::now();
        }

        if app.should_quit {
            break;
        }
    }

    // Cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    run_app().await
}
