use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};
use std::{
    io::{self, stdout},
    time::Duration,
};

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut counter = 0;
    let mut should_quit = false;

    while !should_quit {
        terminal.draw(|frame| {
            let area = frame.area();

            let block = Block::default().title("Counter").borders(Borders::ALL);

            let paragraph = Paragraph::new(format!(
                "Counter: {}\n\nPress '+' to increment\nPress '-' to decrement\nPress 'q' to quit",
                counter,
            ))
            .block(block);

            frame.render_widget(paragraph, area);
        })?;

        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => should_quit = true,
                        KeyCode::Char('+') => counter += 1,
                        KeyCode::Char('-') => counter -= 1,
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
