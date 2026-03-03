# Installation

## Recommended: uv

[uv](https://docs.astral.sh/uv/) is the recommended way to install ascend-tools. It handles Python and dependencies automatically.

Install uv (if you don't have it):

```bash
curl -LsSf https://astral.sh/uv/install.sh | sh
```

Install ascend-tools as a CLI tool:

```bash
uv tool install ascend-tools
```

Upgrade to the latest version:

```bash
uv tool install --upgrade ascend-tools
```

You can also run without installing:

```bash
uvx ascend-tools runtime list
```

### Python SDK

Add ascend-tools as a dependency in your Python project:

```bash
uv add ascend-tools
```

## GitHub releases (pre-built binaries)

Pre-built binaries are available for Linux and macOS. No Python or Rust toolchain required.

### Available platforms

| Platform | Archive |
|----------|---------|
| macOS (Apple Silicon) | `ascend-tools-<VERSION>-aarch64-apple-darwin.tar.gz` |
| macOS (Intel) | `ascend-tools-<VERSION>-x86_64-apple-darwin.tar.gz` |
| Linux (ARM64) | `ascend-tools-<VERSION>-aarch64-unknown-linux-gnu.tar.gz` |
| Linux (x86_64) | `ascend-tools-<VERSION>-x86_64-unknown-linux-gnu.tar.gz` |

### Install with gh CLI

```bash
# Download the latest release for your platform
gh release download --repo ascend-io/ascend-tools --pattern "*aarch64-apple-darwin*"

# Extract
tar xzf ascend-tools-*.tar.gz

# Move to a directory on your PATH
mv ascend-tools /usr/local/bin/
```

### Install with curl

```bash
# Set version and platform
VERSION="v0.4.0"
PLATFORM="aarch64-apple-darwin"

# Download and extract
curl -L "https://github.com/ascend-io/ascend-tools/releases/download/${VERSION}/ascend-tools-${VERSION}-${PLATFORM}.tar.gz" | tar xz

# Move to a directory on your PATH
mv ascend-tools /usr/local/bin/
```

### Verify

```bash
ascend-tools --version
```

## Cargo (Rust)

> Not yet published on crates.io. Contact your Ascend representative if you're interested in Cargo installation.

```bash
cargo install ascend-tools-cli     # CLI binary
cargo add ascend-tools-core        # Rust SDK (library dependency)
```

## Verify installation

Regardless of install method, verify everything is working:

```bash
ascend-tools --version
```

Then set up authentication and test connectivity:

```bash
export ASCEND_SERVICE_ACCOUNT_ID="<YOUR_SERVICE_ACCOUNT_ID>"
export ASCEND_SERVICE_ACCOUNT_KEY="<YOUR_SERVICE_ACCOUNT_KEY>"
export ASCEND_INSTANCE_API_URL="<YOUR_INSTANCE_API_URL>"

ascend-tools runtime list
```

See the [Quickstart](QUICKSTART.md) for the full setup walkthrough.
