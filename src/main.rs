use bollard::Docker;
use clap::Parser;
use color_eyre::{
    Section as _,
    eyre::{Result, WrapErr as _},
};
use std::env;

#[derive(Parser)]
#[command(name = "docker-tui")]
#[command(about = "A Terminal User Interface for Docker")]
#[command(
    after_help = r#"This program must successfully connect to a Docker socket before starting.
If no command-line arguments are specified, the contents of the DOCKER_HOST environment variable
will be used as the socket address. The contents must start with "unix://", "npipe://", "https://",
"http://", or "tcp://" to specify the connection scheme.

If the DOCKER_HOST environment variable isn't specified, then a default of
"npipe:////./pipe/docker_engine" (Windows) or "unix:///var/run/docker.sock" (everything else) will
be used.

To specify the Docker socket explicitly, use --socket."#
)]
struct Args {
    /// Specify the Docker socket path. The path can be specified directly, like
    /// `/foo/bar/docker.sock`, or with a prefix of `npipe://` (Windows) or `unix://` (everything
    /// else), like `unix:///foo/bar/docker.sock`.
    #[arg(short, long, value_name = "PATH")]
    socket: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Args::parse();
    let docker = args
        .socket
        .as_ref()
        .map_or_else(Docker::connect_with_defaults, |socket| {
            Docker::connect_with_socket(socket, 120, bollard::API_DEFAULT_VERSION)
        })
        .wrap_err("Connecting to Docker")
        .suggestion(format!(
            "Make sure Docker is running and that the Docker socket is specified correctly. \
            Run `{} --help` for more information.",
            env::args().next().unwrap_or("docker-tui".to_string())
        ))?;
    docker_tui::main(docker).await?;
    Ok(())
}
