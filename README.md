# Serial Logging CLI

`serial-logging-cli` is a tiny cross-platform command-line tool for capturing raw serial output to timestamped text logs or simple line-based CSV logs.

Use it as a serial port logger for Arduino, ESP32, USB serial adapters, sensors, embedded device testing, and QA bench capture. It prints each incoming line to the terminal and can save the same line without trying to interpret or restructure your data.

## Install

### macOS and Linux

Download and inspect the installer, then run it:

```bash
curl -fsSLO https://raw.githubusercontent.com/DaqSense/serial-logging-cli/main/scripts/install.sh
sh install.sh
```

It detects your operating system and CPU, verifies the release checksum, and installs the executable to `~/.local/bin`. To choose a version or directory:

```bash
sh install.sh --version 0.1.1 --install-dir /your/preferred/bin
```

### Windows

In PowerShell, download and inspect the installer, then run it:

```powershell
Invoke-WebRequest https://raw.githubusercontent.com/DaqSense/serial-logging-cli/main/scripts/install.ps1 -OutFile install.ps1
powershell -ExecutionPolicy Bypass -File .\install.ps1
```

It verifies the release checksum, installs `serial-logging-cli.exe` under your local application data directory, and adds that directory to your user `PATH`. To choose a version or directory:

```powershell
.\install.ps1 -Version 0.1.1 -InstallDir C:\Tools\serial-logging-cli
```

You can also download the archive for Windows, macOS, or Linux from the [latest release](../../releases/latest), extract it, and put the executable somewhere on your `PATH` manually.

To build from source, install a current Rust toolchain and run:

```bash
cargo install --path .
```

## Quick start

List the serial ports visible to your computer:

```bash
serial-logging-cli --list
```

Print incoming serial lines in the terminal:

```bash
serial-logging-cli --port COM3 --baud 115200
```

Save a timestamped text log in `./logs`:

```bash
serial-logging-cli --port /dev/tty.usbmodem101 --baud 9600 --out ./logs
```

Wrap each raw line in a simple CSV row:

```bash
serial-logging-cli --port /dev/ttyUSB0 --baud 115200 --format csv --out ./logs
```

Press `Ctrl+C` to stop. The logger flushes the final buffered line and closes the output file cleanly.

Run `serial-logging-cli --help` for the full command reference.

## Output formats

Text logs contain a UTC timestamp followed by the complete raw line:

```text
2026-07-07T20:10:01.412Z\tTEMP: 22.4 HUM: 41
2026-07-07T20:10:02.418Z\tTEMP: 22.5 HUM: 40
```

CSV logs contain only metadata and the complete raw line. Quotes, commas, and line content are escaped correctly:

```csv
timestamp,port,raw_line
2026-07-07T20:10:01.412Z,COM3,"TEMP: 22.4 HUM: 41"
2026-07-07T20:10:02.418Z,COM3,"TEMP: 22.5 HUM: 40"
```

The CSV format does **not** turn `TEMP` and `HUM` into separate columns. It is intentionally a loss-minimizing wrapper around each incoming serial line.

## Supported platforms

- Windows 10 and 11 serial ports such as `COM3`
- macOS serial devices such as `/dev/tty.usbmodem101`
- Linux serial devices such as `/dev/ttyUSB0` and `/dev/ttyACM0`

Tagged releases publish ready-to-run binaries for Windows, macOS (Apple silicon and Intel), and Linux (x86-64).

## Common serial logging problems

- **Port busy:** Close Arduino Serial Monitor, PuTTY, screen, or any other program holding the port, then try again.
- **Garbage output:** Confirm that `--baud` exactly matches the device firmware. A baud mismatch is the most common cause.
- **Linux permission denied:** Add your user to the system's serial-port group (commonly `dialout`) or update the device rule, then sign in again.
- **Port missing:** Check the USB cable and the board's USB-to-serial driver. CH340-based boards may need a platform-specific driver.
- **Arduino restarts on connect:** Many Arduino boards reset when a serial application opens the port. Startup banners or a short delay are normal for those boards.

## Scope and limitations

This tool owns the raw capture layer:

- list available serial ports;
- connect by port name and baud rate;
- print raw incoming lines;
- write timestamped text logs;
- write simple `timestamp,port,raw_line` CSV logs; and
- stop cleanly with `Ctrl+C`.

It does not parse delimiters or key/value pairs, detect headers, ignore debug rows, coerce numeric values, draw charts, export Excel workbooks, or save device profiles.

Need live tables, charts, parsing profiles, and structured CSV or Excel export? [Use DaqSense](https://daqsense.com/).

## Development

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

Pull requests that keep the CLI focused on dependable raw serial capture are welcome. Features that turn it into a general parsing or visualization product are intentionally out of scope.

## License

MIT
