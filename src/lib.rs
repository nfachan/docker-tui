use color_eyre::eyre::Result;

pub mod docker;
pub mod input;

#[derive(Debug)]
pub enum AppEvent {
    InputEvent(Result<input::Event>),
    DockerEvent(Result<Vec<docker::Container>>),
}
