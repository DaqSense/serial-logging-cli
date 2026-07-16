use std::path::PathBuf;

use clap::{Parser, ValueEnum};

/// Capture raw serial output from a terminal.
#[derive(Debug, Parser)]
#[command(
    name = "serial-logging-cli",
    version,
    about = "Capture raw serial output to text or line-based CSV logs",
    long_about = "A tiny cross-platform serial logger for Arduino, ESP32, sensors, and embedded devices. Incoming lines are printed exactly as text and can also be saved with UTC timestamps."
)]
pub struct Cli {
    /// List available serial ports and exit.
    #[arg(long, conflicts_with = "port")]
    pub list: bool,

    /// Serial port to open, such as COM3 or /dev/ttyUSB0.
    #[arg(long, value_name = "PORT", required_unless_present = "list")]
    pub port: Option<String>,

    /// Serial baud rate.
    #[arg(
        long,
        value_name = "RATE",
        default_value_t = 115_200,
        value_parser = parse_baud
    )]
    pub baud: u32,

    /// Directory in which to create the timestamped log file.
    #[arg(long, value_name = "DIRECTORY", requires = "port")]
    pub out: Option<PathBuf>,

    /// Saved log format. The default is text when --out is set.
    #[arg(long, value_enum, value_name = "FORMAT", requires = "out")]
    pub format: Option<OutputFormat>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub enum OutputFormat {
    #[default]
    Text,
    Csv,
}

fn parse_baud(value: &str) -> Result<u32, String> {
    let baud = value
        .parse::<u32>()
        .map_err(|_| "baud rate must be a positive whole number".to_owned())?;

    if baud == 0 {
        return Err("baud rate must be greater than zero".to_owned());
    }

    Ok(baud)
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::{Cli, OutputFormat};

    #[test]
    fn parses_list_command() {
        let cli = Cli::try_parse_from(["serial-logging-cli", "--list"]).unwrap();

        assert!(cli.list);
        assert_eq!(cli.port, None);
    }

    #[test]
    fn parses_csv_capture_command() {
        let cli = Cli::try_parse_from([
            "serial-logging-cli",
            "--port",
            "COM3",
            "--baud",
            "9600",
            "--out",
            "logs",
            "--format",
            "csv",
        ])
        .unwrap();

        assert_eq!(cli.port.as_deref(), Some("COM3"));
        assert_eq!(cli.baud, 9600);
        assert_eq!(cli.format, Some(OutputFormat::Csv));
    }

    #[test]
    fn requires_a_port_for_capture() {
        assert!(Cli::try_parse_from(["serial-logging-cli", "--out", "logs"]).is_err());
    }

    #[test]
    fn rejects_zero_baud() {
        assert!(
            Cli::try_parse_from(["serial-logging-cli", "--port", "COM3", "--baud", "0"]).is_err()
        );
    }
}
