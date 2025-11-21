use bollard::{Docker, container::ListContainersOptions};
use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem},
};
use std::{
    io::{self, stdout},
    time::Duration,
};
use tokio::runtime::Runtime;

async fn get_containers() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let docker = Docker::connect_with_socket_defaults()?;
    let options = Some(ListContainersOptions::<String> {
        all: true,
        ..Default::default()
    });

    let containers = docker.list_containers(options).await?;
    let container_list = containers
        .iter()
        .map(|container| {
            let name = container
                .names
                .as_ref()
                .and_then(|names| names.first())
                .map(|n| n.trim_start_matches('/'))
                .unwrap_or("unnamed");
            let status = container.status.as_deref().unwrap_or("unknown");
            format!("{} - {}", name, status)
        })
        .collect();

    Ok(container_list)
}

fn main() -> io::Result<()> {
    let rt = Runtime::new()?;
    let containers = rt.block_on(async {
        get_containers()
            .await
            .unwrap_or_else(|_| vec!["Failed to connect to Docker".to_string()])
    });

    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut should_quit = false;

    while !should_quit {
        terminal.draw(|frame| {
            let area = frame.area();

            let block = Block::default()
                .title("Docker Containers")
                .borders(Borders::ALL);

            let items: Vec<ListItem> = containers
                .iter()
                .map(|container| ListItem::new(container.as_str()))
                .collect();

            let list = List::new(items).block(block);

            frame.render_widget(list, area);
        })?;

        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => should_quit = true,
                        _ => {}
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
