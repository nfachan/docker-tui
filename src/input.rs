use color_eyre::eyre::Report;
use crossterm::event;
use tokio::sync::mpsc;

pub use crossterm::event::Event;

pub fn main(sender: mpsc::Sender<crate::Event>) {
    while sender
        .blocking_send(crate::Event::InputEvent(
            event::read().map_err(Report::from),
        ))
        .is_ok()
    {}
}
