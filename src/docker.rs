use crate::AppEvent;
use bollard::{Docker, container::ListContainersOptions};
use color_eyre::eyre::{Report, Result};
use std::time::Duration;
use tokio::{runtime::Builder, sync::mpsc, task, time};

#[derive(Debug, Clone)]
pub struct Container {
    pub name: String,
    pub status: String,
}

async fn fetch_containers() -> Result<Vec<Container>> {
    let docker = Docker::connect_with_socket_defaults()?;
    let options = Some(ListContainersOptions::<String> {
        all: true,
        ..Default::default()
    });

    let container_list = docker.list_containers(options).await?;
    let parsed_containers: Vec<Container> = container_list
        .iter()
        .map(|container| {
            let name = container
                .names
                .as_ref()
                .and_then(|names| names.first())
                .map(|n| n.trim_start_matches('/').to_string())
                .unwrap_or_else(|| "unnamed".to_string());
            let status = container.status.as_deref().unwrap_or("unknown").to_string();
            Container { name, status }
        })
        .collect();

    Ok(parsed_containers)
}

async fn docker_event_main_inner(sender: mpsc::Sender<AppEvent>) {
    while sender
        .send(AppEvent::DockerEvent(fetch_containers().await))
        .await
        .is_ok()
    {
        time::sleep(Duration::from_secs(1)).await;
    }
}

pub fn main(sender: mpsc::Sender<AppEvent>) -> Result<()> {
    Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async move { task::spawn(docker_event_main_inner(sender)).await })
        .map_err(Report::from)
}
