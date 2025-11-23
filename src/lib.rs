use color_eyre::eyre::Result;
use crossterm::event::Event;

pub mod docker;
pub mod input;

#[derive(Debug)]
pub enum AppEvent {
    InputEvent(Result<Event>),
    DockerEvent(Result<Vec<docker::Container>>),
}
