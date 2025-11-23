use clap::Parser;
use color_eyre::eyre::Result;

#[derive(Parser)]
#[command(name = "docker-tui")]
#[command(about = "A Terminal User Interface for Docker")]
struct Args {}

fn main() -> Result<()> {
    color_eyre::install()?;
    let _ = Args::parse();
    docker_tui::main()
}
