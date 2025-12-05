use color_eyre::eyre::{Report, Result};
use crossterm::event::EventStream;
use futures::stream::StreamExt as _;
use tokio::sync::mpsc;

pub use crossterm::event::Event;

pub async fn main<E>(sender: mpsc::Sender<E>, sender_processor: impl Fn(Result<Event>) -> E) {
    let mut event_stream = EventStream::new();
    while let Some(event) = event_stream.next().await {
        if sender
            .send(sender_processor(event.map_err(Report::from)))
            .await
            .is_err()
        {
            break;
        }
    }
}
