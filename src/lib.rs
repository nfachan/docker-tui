use bollard::Docker;
use color_eyre::eyre::{Report, Result};
use container_list::ContainerList;
use crossterm::{cursor, event, execute, terminal};
use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
    layout::Rect,
};
use std::{io, ops::ControlFlow, panic, thread};
use tokio::sync::mpsc;

mod container_list;
mod docker;
mod input;
mod input_state_machine;
mod viewport;

#[derive(Debug)]
pub enum Event {
    Input(Result<input::Event>),
    FromDocker(docker::MessageOut),
}

fn handle_container_list_events(
    messages: impl IntoIterator<Item = container_list::MessageOut>,
    docker_sender: &mut mpsc::UnboundedSender<docker::MessageIn>,
) -> Result<ControlFlow<()>> {
    for message in messages {
        match message {
            container_list::MessageOut::Exit => {
                return Ok(ControlFlow::Break(()));
            }
            container_list::MessageOut::ToDocker(docker_message) => {
                docker_sender.send(docker_message)?
            }
        }
    }
    Ok(ControlFlow::Continue(()))
}

fn main_loop(docker: Docker, mut terminal: Terminal<impl Backend>) -> Result<()> {
    const CHANNEL_SLOTS: usize = 10;
    let (mut docker_sender, docker_receiver) = mpsc::unbounded_channel();
    let (sender, mut receiver) = mpsc::channel(CHANNEL_SLOTS);

    // Spawn the docker thread.
    let sender_clone = sender.clone();
    thread::spawn(move || docker::main(docker, docker_receiver, sender_clone, Event::FromDocker));

    // Spawn input event thread.
    thread::spawn(move || input::main(sender));
    let mut container_list = ContainerList::default();

    // Send an initial message to the docker thread.
    docker_sender.send(docker::MessageIn::GetContainers)?;

    terminal.draw(|frame| frame.render_widget(&mut container_list, frame.area()))?;
    while let Some(event) = receiver.blocking_recv() {
        let mut messages_out = vec![];
        match event {
            Event::Input(Ok(input::Event::Key(event))) => {
                messages_out.extend(container_list.handle_key_event(event)?);
            }
            Event::Input(Ok(input::Event::Mouse(event))) => {
                container_list.handle_mouse_event(event);
            }
            Event::Input(Ok(input::Event::Resize(columns, rows))) => {
                container_list.handle_resize(Rect::new(0, 0, columns, rows));
            }
            Event::Input(Ok(
                input::Event::FocusGained | input::Event::FocusLost | input::Event::Paste(_),
            )) => {
                continue;
            }
            Event::Input(Err(err)) => {
                return Err(err);
            }
            Event::FromDocker(message) => {
                container_list.handle_docker_response(message)?;
            }
        }
        if let ControlFlow::Break(_) =
            handle_container_list_events(messages_out, &mut docker_sender)?
        {
            return Ok(());
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
    execute!(stdout, terminal::EnterAlternateScreen)?;
    execute!(stdout, cursor::Hide)?;
    execute!(stdout, event::EnableMouseCapture)?;

    main_loop(docker, Terminal::new(CrosstermBackend::new(stdout))?)
}

fn main_clean_up() -> Result<()> {
    let mut stdout = io::stdout();
    [
        execute!(stdout, event::DisableMouseCapture),
        execute!(stdout, cursor::Show),
        execute!(stdout, terminal::LeaveAlternateScreen),
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
