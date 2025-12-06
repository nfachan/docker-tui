use bollard::{Docker, container::ListContainersOptions};
use color_eyre::eyre::{Report, Result};
use tokio::sync::mpsc;

pub use bollard::models::ContainerSummary as Container;

pub enum MessageIn {
    GetContainers { all: bool },
}

#[derive(Debug)]
pub enum MessageOut {
    GetContainers(Vec<Container>),
}

pub async fn main<E>(
    docker: Docker,
    mut receiver: mpsc::UnboundedReceiver<MessageIn>,
    sender: mpsc::Sender<E>,
    sender_processor: impl Fn(Result<MessageOut>) -> E,
) {
    while let Some(message) = receiver.recv().await {
        match message {
            MessageIn::GetContainers { all } => {
                let response = docker
                    .list_containers(Some(ListContainersOptions::<String> {
                        all,
                        ..Default::default()
                    }))
                    .await
                    .map(MessageOut::GetContainers)
                    .map_err(Report::from);
                if sender.send(sender_processor(response)).await.is_err() {
                    break;
                }
            }
        }
    }
}
