use bollard::Docker;
use color_eyre::eyre::{Report, Result};
use container_list::ContainerList;
use crossterm::{cursor, event, execute, terminal};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{
    io::{self, Stdout},
    ops::ControlFlow,
    panic, thread,
};
use tokio::sync::mpsc;

mod container_list;
mod docker;
mod input;
mod input_state_machine;
mod viewport;

#[derive(Debug)]
pub enum Event {
    Input(Result<input::Event>),
    FromDocker(Result<docker::MessageOut>),
}

struct App {
    docker_sender: mpsc::UnboundedSender<docker::MessageIn>,
    receiver: mpsc::Receiver<Event>,
    container_list: ContainerList,
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl App {
    fn new(docker: Docker, terminal: Terminal<CrosstermBackend<Stdout>>) -> Result<Self> {
        const CHANNEL_SLOTS: usize = 10;
        let (docker_sender, docker_receiver) = mpsc::unbounded_channel();
        let (sender, receiver) = mpsc::channel(CHANNEL_SLOTS);

        // Spawn the docker thread.
        let sender_clone = sender.clone();
        thread::spawn(move || {
            docker::main(docker, docker_receiver, sender_clone, Event::FromDocker)
        });

        // Spawn input event thread.
        thread::spawn(move || input::main(sender));

        // Create the ContainerList.
        let (container_list, messages_out) = ContainerList::new();

        let mut result = Self {
            docker_sender,
            receiver,
            container_list,
            terminal,
        };

        // Handle any of the events the ContainerList generated.
        let control_flow = result.handle_container_list_events(messages_out)?;
        assert!(matches!(control_flow, ControlFlow::Continue(_)));

        Ok(result)
    }

    fn handle_container_list_events(
        &mut self,
        messages: impl IntoIterator<Item = container_list::MessageOut>,
    ) -> Result<ControlFlow<()>> {
        for message in messages {
            match message {
                container_list::MessageOut::Exit => {
                    return Ok(ControlFlow::Break(()));
                }
                container_list::MessageOut::Render => {
                    self.terminal.draw(|frame| {
                        frame.render_widget(&mut self.container_list, frame.area())
                    })?;
                }
                container_list::MessageOut::ToDocker(docker_message) => {
                    self.docker_sender.send(docker_message)?
                }
            }
        }
        Ok(ControlFlow::Continue(()))
    }

    fn main_loop(&mut self) -> Result<()> {
        while let Some(event) = self.receiver.blocking_recv() {
            let mut messages_out = vec![];
            match event {
                Event::Input(Ok(event)) => {
                    messages_out.extend(self.container_list.handle_input_event(event));
                }
                Event::FromDocker(Ok(message)) => {
                    messages_out.extend(self.container_list.handle_docker_response(message));
                }
                Event::Input(Err(err)) | Event::FromDocker(Err(err)) => {
                    return Err(err);
                }
            }
            if let ControlFlow::Break(_) = self.handle_container_list_events(messages_out)? {
                break;
            }
        }
        Ok(())
    }
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

    App::new(docker, Terminal::new(CrosstermBackend::new(stdout))?)?.main_loop()
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
