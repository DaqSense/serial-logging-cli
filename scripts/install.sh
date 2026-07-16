#!/bin/sh

set -eu

repository="DaqSense/serial-logging-cli"
version="latest"
install_dir="${SERIAL_LOGGING_CLI_INSTALL_DIR:-${HOME}/.local/bin}"

usage() {
    cat <<'EOF'
Install serial-logging-cli for the current user.

Usage:
  sh install.sh [--version VERSION] [--install-dir DIRECTORY]

Options:
  --version VERSION        Release to install, such as 0.1.1 (default: latest)
  --install-dir DIRECTORY  Destination directory (default: ~/.local/bin)
  -h, --help               Show this help

Environment:
  SERIAL_LOGGING_CLI_INSTALL_DIR  Alternative default installation directory
EOF
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --version)
            [ "$#" -ge 2 ] || {
                echo "error: --version requires a value" >&2
                exit 2
            }
            version=$2
            shift 2
            ;;
        --install-dir)
            [ "$#" -ge 2 ] || {
                echo "error: --install-dir requires a value" >&2
                exit 2
            }
            install_dir=$2
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "error: unknown option: $1" >&2
            usage >&2
            exit 2
            ;;
    esac
done

for command in curl install tar; do
    command -v "$command" >/dev/null 2>&1 || {
        echo "error: required command '$command' was not found" >&2
        exit 1
    }
done

os=$(uname -s)
architecture=$(uname -m)

case "$os:$architecture" in
    Darwin:arm64|Darwin:aarch64)
        target="aarch64-apple-darwin"
        ;;
    Darwin:x86_64|Darwin:amd64)
        target="x86_64-apple-darwin"
        ;;
    Linux:x86_64|Linux:amd64)
        target="x86_64-unknown-linux-gnu"
        ;;
    *)
        echo "error: unsupported platform: $os $architecture" >&2
        echo "Download a compatible binary from https://github.com/$repository/releases" >&2
        exit 1
        ;;
esac

asset="serial-logging-cli-${version}-${target}.tar.gz"
if [ "$version" = "latest" ]; then
    asset="serial-logging-cli-LATEST-${target}.tar.gz"
    download_base="https://github.com/$repository/releases/latest/download"
else
    case "$version" in
        v*) tag=$version ;;
        *) tag="v$version" ;;
    esac
    asset="serial-logging-cli-${tag}-${target}.tar.gz"
    download_base="https://github.com/$repository/releases/download/$tag"
fi

temporary_dir=$(mktemp -d 2>/dev/null || mktemp -d -t serial-logging-cli)
cleanup() {
    rm -rf "$temporary_dir"
}
trap cleanup EXIT HUP INT TERM

archive="$temporary_dir/$asset"
checksums="$temporary_dir/SHA256SUMS"

echo "Downloading serial-logging-cli ($target)..."
if [ "$version" = "latest" ]; then
    release_url=$(curl -fsSIL -o /dev/null -w '%{url_effective}' "https://github.com/$repository/releases/latest")
    tag=${release_url##*/}
    asset="serial-logging-cli-${tag}-${target}.tar.gz"
    archive="$temporary_dir/$asset"
fi

curl -fsSL "$download_base/$asset" -o "$archive"
curl -fsSL "$download_base/SHA256SUMS" -o "$checksums"

expected_checksum=$(awk -v asset="$asset" '$2 == asset { print $1 }' "$checksums")
[ -n "$expected_checksum" ] || {
    echo "error: $asset is missing from SHA256SUMS" >&2
    exit 1
}

if command -v sha256sum >/dev/null 2>&1; then
    actual_checksum=$(sha256sum "$archive" | awk '{ print $1 }')
elif command -v shasum >/dev/null 2>&1; then
    actual_checksum=$(shasum -a 256 "$archive" | awk '{ print $1 }')
else
    echo "error: neither sha256sum nor shasum is available" >&2
    exit 1
fi

[ "$actual_checksum" = "$expected_checksum" ] || {
    echo "error: checksum verification failed for $asset" >&2
    exit 1
}

tar -xzf "$archive" -C "$temporary_dir"
mkdir -p "$install_dir"
install -m 755 "$temporary_dir/serial-logging-cli" "$install_dir/serial-logging-cli"

echo "Installed serial-logging-cli to $install_dir/serial-logging-cli"
case ":${PATH}:" in
    *":${install_dir}:"*) ;;
    *)
        echo "Add this directory to your PATH:"
        echo "  export PATH=\"$install_dir:\$PATH\""
        ;;
esac
echo "Try: serial-logging-cli --list"
