use bollard::{Docker, container::ListContainersOptions};
use color_eyre::eyre::{Error, Report, Result};
use tokio::{runtime::Builder, sync::mpsc, task};

pub use bollard::models::ContainerSummary as Container;

pub enum MessageIn {
    GetContainers,
}

#[derive(Debug)]
pub enum MessageOut {
    GetContainers(Vec<Container>),
}

async fn fetch_containers(docker: &Docker) -> Result<Vec<Container>> {
    let options = Some(ListContainersOptions::<String> {
        all: true,
        ..Default::default()
    });
    docker.list_containers(options).await.map_err(Error::from)
}

async fn main_inner<E>(
    docker: Docker,
    mut receiver: mpsc::UnboundedReceiver<MessageIn>,
    sender: mpsc::Sender<E>,
    sender_processor: impl Fn(Result<MessageOut>) -> E,
) {
    while let Some(message) = receiver.recv().await {
        match message {
            MessageIn::GetContainers => {
                let response = fetch_containers(&docker)
                    .await
                    .map(MessageOut::GetContainers);
                if sender.send(sender_processor(response)).await.is_err() {
                    break;
                }
            }
        }
    }
}

pub fn main<E: Send + 'static>(
    docker: Docker,
    receiver: mpsc::UnboundedReceiver<MessageIn>,
    sender: mpsc::Sender<E>,
    sender_processor: impl Fn(Result<MessageOut>) -> E + Send + Sync + 'static,
) -> Result<()> {
    Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            task::spawn(main_inner(docker, receiver, sender, sender_processor)).await
        })
        .map_err(Report::from)
}
