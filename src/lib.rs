use bollard::Docker;
use color_eyre::eyre::{Report, Result};
use container_list::ContainerList;
use crossterm::{cursor, event, execute, terminal};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{
    future,
    io::{self, Stdout},
    ops::ControlFlow,
    panic, thread,
};
use tokio::{runtime::Runtime, sync::mpsc};

mod container_list;
mod docker;
mod input;
mod input_state_machine;
mod timer;
mod viewport;

#[derive(Debug)]
pub enum Event {
    Input(Result<input::Event>),
    Docker(Result<docker::MessageOut>),
    Timer(timer::MessageOut),
}

struct App {
    docker_sender: mpsc::UnboundedSender<docker::MessageIn>,
    timer_sender: mpsc::UnboundedSender<timer::MessageIn>,
    receiver: mpsc::Receiver<Event>,
    container_list: ContainerList,
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl App {
    fn new(
        docker: Docker,
        runtime: Runtime,
        terminal: Terminal<CrosstermBackend<Stdout>>,
    ) -> (Self, Vec<container_list::MessageOut>) {
        const CHANNEL_SLOTS: usize = 10;
        let (docker_sender, docker_receiver) = mpsc::unbounded_channel();
        let (timer_sender, timer_receiver) = mpsc::unbounded_channel();
        let (sender, receiver) = mpsc::channel(CHANNEL_SLOTS);

        // Spawn the docker thread.
        runtime.spawn(docker::main(
            docker,
            docker_receiver,
            sender.clone(),
            Event::Docker,
        ));

        // Spawn the timer thread.
        runtime.spawn(timer::main(timer_receiver, sender.clone(), Event::Timer));

        // Spawn input event thread.
        runtime.spawn(input::main(sender, Event::Input));

        // Spawn a thread to wait on the runtime.
        thread::spawn(move || runtime.block_on(future::pending::<()>()));

        // Create the ContainerList.
        let (container_list, messages_out) = ContainerList::new();

        (
            Self {
                docker_sender,
                timer_sender,
                receiver,
                container_list,
                terminal,
            },
            messages_out,
        )
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
                container_list::MessageOut::ToDocker(message) => {
                    self.docker_sender.send(message)?
                }
                container_list::MessageOut::ToTimer(message) => self.timer_sender.send(message)?,
            }
        }
        Ok(ControlFlow::Continue(()))
    }

    fn main_loop(&mut self) -> Result<()> {
        while let Some(event) = self.receiver.blocking_recv() {
            let messages_out = match event {
                Event::Input(Ok(event)) => self.container_list.handle_input_event(event),
                Event::Docker(Ok(message)) => self.container_list.handle_docker_response(message),
                Event::Timer(event) => self.container_list.handle_timer_event(event),
                Event::Input(Err(err)) | Event::Docker(Err(err)) => {
                    return Err(err);
                }
            };
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

    let runtime = Runtime::new()?;
    let (mut app, initial_messages) = App::new(
        docker,
        runtime,
        Terminal::new(CrosstermBackend::new(stdout))?,
    );
    if let ControlFlow::Continue(_) = app.handle_container_list_events(initial_messages)? {
        app.main_loop()?;
    }

    Ok(())
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
