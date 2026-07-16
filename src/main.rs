mod cli;
mod logger;

use anyhow::Result;
use clap::Parser;

use cli::{Cli, OutputFormat};
use logger::{CaptureOptions, capture, print_available_ports};

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.list {
        return print_available_ports(std::io::stdout().lock());
    }

    let port = cli
        .port
        .as_deref()
        .expect("clap requires --port unless --list is used");
    let summary = capture(CaptureOptions {
        port,
        baud: cli.baud,
        out: cli.out.as_deref(),
        format: cli.format.unwrap_or(OutputFormat::Text),
    })?;

    match summary.output_path {
        Some(path) => eprintln!(
            "Stopped cleanly after {} lines. Log saved to {}",
            summary.lines,
            path.display()
        ),
        None => eprintln!("Stopped cleanly after {} lines.", summary.lines),
    }

    Ok(())
}
