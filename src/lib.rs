use bollard::Docker;
use color_eyre::eyre::{Report, Result};
use container_list::ContainerList;
use crossterm::{
    cursor,
    event::KeyEventKind,
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use docker::Container;
use ratatui::prelude::*;
use std::{io, ops::ControlFlow, panic, thread};
use tokio::sync::mpsc;

mod container_list;
mod docker;
mod input;
mod viewport;

#[derive(Debug)]
pub enum Event {
    Input(Result<input::Event>),
    Docker(Result<Vec<Container>>),
}

fn main_loop(docker: Docker, mut terminal: Terminal<impl Backend>) -> Result<()> {
    const CHANNEL_SLOTS: usize = 10;
    let (sender, mut receiver) = mpsc::channel(CHANNEL_SLOTS);

    let sender_clone = sender.clone();
    thread::spawn(move || docker::main(docker, sender_clone));

    // Spawn input event thread.
    thread::spawn(move || input::main(sender));
    let mut container_list = ContainerList::default();

    terminal.draw(|frame| frame.render_widget(&mut container_list, frame.area()))?;
    while let Some(event) = receiver.blocking_recv() {
        match event {
            Event::Input(Ok(input::Event::Key(key))) if key.kind == KeyEventKind::Press => {
                if let ControlFlow::Break(_) = container_list.handle_key_event(key) {
                    break;
                }
            }
            Event::Input(Ok(_)) => {}
            Event::Docker(Ok(containers)) => {
                container_list.handle_containers(containers);
            }
            Event::Input(Err(err)) | Event::Docker(Err(err)) => {
                return Err(err);
            }
        }
        terminal.draw(|frame| frame.render_widget(&mut container_list, frame.area()))?;
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
