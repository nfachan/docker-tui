use crate::Event;
use bollard::{Docker, container::ListContainersOptions};
use color_eyre::eyre::{Error, Report, Result};
use std::time::Duration;
use tokio::{runtime::Builder, sync::mpsc, task, time};

pub use bollard::models::ContainerSummary as Container;

async fn fetch_containers(docker: &Docker) -> Result<Vec<Container>> {
    let options = Some(ListContainersOptions::<String> {
        all: true,
        ..Default::default()
    });
    docker.list_containers(options).await.map_err(Error::from)
}

async fn docker_event_main_inner(docker: Docker, sender: mpsc::Sender<Event>) {
    while sender
        .send(Event::Docker(fetch_containers(&docker).await))
        .await
        .is_ok()
    {
        time::sleep(Duration::from_secs(10)).await;
    }
}

pub fn main(docker: Docker, sender: mpsc::Sender<Event>) -> Result<()> {
    Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async move { task::spawn(docker_event_main_inner(docker, sender)).await })
        .map_err(Report::from)
}
