use color_eyre::eyre::Result;
use crossterm::{
    event::{KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use docker_tui::{AppEvent, docker::Container, input};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState},
};
use tokio::sync::mpsc;

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

fn run_app() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::default();

    const CHANNEL_SLOTS: usize = 10;
    let (sender, mut receiver) = mpsc::channel(CHANNEL_SLOTS);

    let sender_clone = sender.clone();
    std::thread::spawn(move || docker_tui::docker::main(sender_clone));

    // Spawn input event thread.
    std::thread::spawn(move || docker_tui::input::main(sender));

    // Main event loop
    terminal.draw(|frame| app.render(frame))?;
    while let Some(event) = receiver.blocking_recv() {
        match event {
            AppEvent::InputEvent(Ok(input::Event::Key(key))) if key.kind == KeyEventKind::Press => {
                app.handle_key_event(key.code);
            }
            AppEvent::InputEvent(Ok(_)) => {}
            AppEvent::DockerEvent(Ok(containers)) => {
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
            AppEvent::InputEvent(Err(err)) | AppEvent::DockerEvent(Err(err)) => {
                return Err(err);
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
