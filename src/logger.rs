use std::{
    fs::{self, File},
    io::{self, BufWriter, Read, Write},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use anyhow::{Context, Result, bail};
use chrono::{SecondsFormat, Utc};
use serialport::{SerialPortInfo, SerialPortType};

use crate::cli::OutputFormat;

const READ_TIMEOUT: Duration = Duration::from_millis(250);

pub struct CaptureOptions<'a> {
    pub port: &'a str,
    pub baud: u32,
    pub out: Option<&'a Path>,
    pub format: OutputFormat,
}

#[derive(Debug)]
pub struct CaptureSummary {
    pub lines: u64,
    pub output_path: Option<PathBuf>,
}

pub fn print_available_ports(mut output: impl Write) -> Result<()> {
    let mut ports = serialport::available_ports().context("could not enumerate serial ports")?;
    ports.sort_by(|left, right| left.port_name.cmp(&right.port_name));

    if ports.is_empty() {
        writeln!(output, "No serial ports found.")?;
        return Ok(());
    }

    writeln!(output, "PORT\tTYPE\tDETAILS")?;
    for port in ports {
        let (kind, details) = describe_port(&port);
        writeln!(output, "{}\t{kind}\t{details}", port.port_name)?;
    }

    Ok(())
}

pub fn capture(options: CaptureOptions<'_>) -> Result<CaptureSummary> {
    let running = Arc::new(AtomicBool::new(true));
    let signal_flag = Arc::clone(&running);
    ctrlc::set_handler(move || signal_flag.store(false, Ordering::SeqCst))
        .context("could not install the Ctrl+C handler")?;

    let mut port = serialport::new(options.port, options.baud)
        .timeout(READ_TIMEOUT)
        .open()
        .with_context(|| {
            format!(
                "could not open serial port '{}' at {} baud; check the port name, permissions, and whether another serial monitor is using it",
                options.port, options.baud
            )
        })?;

    let mut log = match options.out {
        Some(directory) => Some(LogWriter::create(directory, options.port, options.format)?),
        None => None,
    };
    let output_path = log.as_ref().map(|writer| writer.path().to_owned());

    eprintln!(
        "Connected to {} at {} baud. Press Ctrl+C to stop.",
        options.port, options.baud
    );
    if let Some(path) = &output_path {
        eprintln!("Saving log to {}", path.display());
    }

    let stdout = io::stdout();
    let mut terminal = stdout.lock();
    let mut buffer = [0_u8; 4096];
    let mut lines = LineBuffer::default();
    let mut line_count = 0_u64;

    while running.load(Ordering::SeqCst) {
        match port.read(&mut buffer) {
            Ok(0) => {}
            Ok(bytes_read) => {
                for line in lines.push(&buffer[..bytes_read]) {
                    record_line(
                        &line,
                        options.port,
                        &mut terminal,
                        log.as_mut(),
                        &mut line_count,
                    )?;
                }
            }
            Err(error) if error.kind() == io::ErrorKind::TimedOut => {}
            Err(error) => {
                bail!(
                    "failed while reading from serial port '{}': {error}",
                    options.port
                );
            }
        }
    }

    if let Some(line) = lines.finish() {
        record_line(
            &line,
            options.port,
            &mut terminal,
            log.as_mut(),
            &mut line_count,
        )?;
    }

    if let Some(writer) = log.as_mut() {
        writer.flush()?;
    }
    terminal.flush()?;

    Ok(CaptureSummary {
        lines: line_count,
        output_path,
    })
}

fn record_line(
    line: &[u8],
    port: &str,
    terminal: &mut impl Write,
    log: Option<&mut LogWriter>,
    line_count: &mut u64,
) -> Result<()> {
    let raw_line = String::from_utf8_lossy(line);
    writeln!(terminal, "{raw_line}").context("could not write serial output to the terminal")?;
    terminal.flush()?;

    if let Some(writer) = log {
        let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        writer.write_line(&timestamp, port, &raw_line)?;
        writer.flush()?;
    }

    *line_count += 1;
    Ok(())
}

fn describe_port(port: &SerialPortInfo) -> (&'static str, String) {
    match &port.port_type {
        SerialPortType::UsbPort(info) => {
            let mut details = vec![format!("VID:PID={:04X}:{:04X}", info.vid, info.pid)];
            if let Some(product) = &info.product {
                details.push(format!("product={product}"));
            }
            if let Some(manufacturer) = &info.manufacturer {
                details.push(format!("manufacturer={manufacturer}"));
            }
            if let Some(serial_number) = &info.serial_number {
                details.push(format!("serial={serial_number}"));
            }
            ("USB", details.join(" "))
        }
        SerialPortType::BluetoothPort => ("Bluetooth", String::new()),
        SerialPortType::PciPort => ("PCI", String::new()),
        SerialPortType::Unknown => ("Unknown", String::new()),
    }
}

#[derive(Default)]
struct LineBuffer {
    pending: Vec<u8>,
}

impl LineBuffer {
    fn push(&mut self, bytes: &[u8]) -> Vec<Vec<u8>> {
        self.pending.extend_from_slice(bytes);
        let mut complete = Vec::new();
        let mut line_start = 0;

        for (index, byte) in self.pending.iter().enumerate() {
            if *byte == b'\n' {
                complete.push(clean_line(&self.pending[line_start..index]));
                line_start = index + 1;
            }
        }

        if line_start > 0 {
            self.pending.drain(..line_start);
        }

        complete
    }

    fn finish(&mut self) -> Option<Vec<u8>> {
        if self.pending.is_empty() {
            None
        } else {
            Some(clean_line(std::mem::take(&mut self.pending).as_slice()))
        }
    }
}

fn clean_line(line: &[u8]) -> Vec<u8> {
    line.strip_suffix(b"\r").unwrap_or(line).to_vec()
}

enum LogWriter {
    Text {
        path: PathBuf,
        writer: BufWriter<File>,
    },
    Csv {
        path: PathBuf,
        writer: Box<csv::Writer<BufWriter<File>>>,
    },
}

impl LogWriter {
    fn create(directory: &Path, port: &str, format: OutputFormat) -> Result<Self> {
        fs::create_dir_all(directory).with_context(|| {
            format!("could not create output directory {}", directory.display())
        })?;

        let extension = match format {
            OutputFormat::Text => "log",
            OutputFormat::Csv => "csv",
        };
        let timestamp = Utc::now().format("%Y%m%dT%H%M%S%.3fZ");
        let filename = format!("serial-{}-{timestamp}.{extension}", sanitize_filename(port));
        let path = directory.join(filename);
        let file = File::options()
            .write(true)
            .create_new(true)
            .open(&path)
            .with_context(|| format!("could not create log file {}", path.display()))?;
        let writer = BufWriter::new(file);

        match format {
            OutputFormat::Text => Ok(Self::Text { path, writer }),
            OutputFormat::Csv => {
                let mut writer = csv::WriterBuilder::new()
                    .has_headers(false)
                    .from_writer(writer);
                writer.write_record(["timestamp", "port", "raw_line"])?;
                writer.flush()?;
                Ok(Self::Csv {
                    path,
                    writer: Box::new(writer),
                })
            }
        }
    }

    fn path(&self) -> &Path {
        match self {
            Self::Text { path, .. } | Self::Csv { path, .. } => path,
        }
    }

    fn write_line(&mut self, timestamp: &str, port: &str, raw_line: &str) -> Result<()> {
        match self {
            Self::Text { writer, .. } => {
                writeln!(writer, "{timestamp}\t{raw_line}")?;
            }
            Self::Csv { writer, .. } => {
                writer.write_record([timestamp, port, raw_line])?;
            }
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        match self {
            Self::Text { writer, .. } => writer.flush()?,
            Self::Csv { writer, .. } => writer.flush()?,
        }
        Ok(())
    }
}

fn sanitize_filename(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '-'
            }
        })
        .collect();

    let trimmed = sanitized.trim_matches(['-', '.']);
    if trimmed.is_empty() {
        "serial-port".to_owned()
    } else {
        trimmed.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{LineBuffer, LogWriter, sanitize_filename};
    use crate::cli::OutputFormat;

    #[test]
    fn assembles_lines_across_reads_and_removes_crlf() {
        let mut buffer = LineBuffer::default();

        assert!(buffer.push(b"TEMP: 22").is_empty());
        assert_eq!(
            buffer.push(b".4\r\nHUM: 41\npartial"),
            vec![b"TEMP: 22.4".to_vec(), b"HUM: 41".to_vec()]
        );
        assert_eq!(buffer.finish(), Some(b"partial".to_vec()));
    }

    #[test]
    fn preserves_empty_serial_lines() {
        let mut buffer = LineBuffer::default();

        assert_eq!(buffer.push(b"\n\r\n"), vec![Vec::new(), Vec::new()]);
    }

    #[test]
    fn creates_valid_csv_with_escaped_raw_lines() {
        let directory = tempdir().unwrap();
        let mut writer = LogWriter::create(directory.path(), "COM3", OutputFormat::Csv).unwrap();
        let path = writer.path().to_owned();

        writer
            .write_line("2026-07-07T20:10:01.412Z", "COM3", "TEMP: 22.4, HUM: 41")
            .unwrap();
        writer.flush().unwrap();
        drop(writer);

        let contents = fs::read_to_string(path).unwrap();
        assert_eq!(
            contents,
            "timestamp,port,raw_line\n2026-07-07T20:10:01.412Z,COM3,\"TEMP: 22.4, HUM: 41\"\n"
        );
    }

    #[test]
    fn makes_port_names_safe_for_filenames() {
        assert_eq!(
            sanitize_filename("/dev/tty.usbmodem101"),
            "dev-tty.usbmodem101"
        );
        assert_eq!(sanitize_filename("COM3"), "COM3");
    }
}
