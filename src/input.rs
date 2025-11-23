use crate::AppEvent;
use color_eyre::eyre::Report;
use crossterm::event;
use tokio::sync::mpsc;

pub fn main(sender: mpsc::Sender<AppEvent>) {
    while sender
        .blocking_send(AppEvent::InputEvent(event::read().map_err(Report::from)))
        .is_ok()
    {}
}
