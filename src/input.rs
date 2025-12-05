use color_eyre::eyre::{Report, Result};
use crossterm::event;
use tokio::sync::mpsc;

pub use crossterm::event::Event;

pub fn main<E>(sender: mpsc::Sender<E>, sender_processor: impl Fn(Result<Event>) -> E) {
    while sender
        .blocking_send(sender_processor(event::read().map_err(Report::from)))
        .is_ok()
    {}
}
