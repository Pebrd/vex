# Installation

This guide covers how to install and build vex on your system.

## Prerequisites

Before installing vex, make sure you have the following installed:

- **Rust**: Version 1.56 or later
- **Git**: For cloning the repository
- **C++ Build Tools**: Required for some dependencies

### Installing Rust

If you don't have Rust installed, you can install it using rustup:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

After installation, source your environment:

```bash
source $HOME/.cargo/env
```

Verify the installation:

```bash
rustc --version
# Should show version 1.56 or later
```

## Building from Source

### Step 1: Clone the Repository

```bash
git clone https://github.com/Pebrd/vex.git
cd vex
```

### Step 2: Install Dependencies

On Ubuntu/Debian:
```bash
sudo apt-get install build-essential libssl-dev pkg-config
```

On Fedora:
```bash
sudo dnf install gcc openssl-devel pkg-config
```

On macOS (using Homebrew):
```bash
brew install openssl pkg-config
```

On Windows, make sure you have Visual Studio Build Tools installed.

### Step 3: Build the Project

To build a debug version:
```bash
cargo build
```

To build an optimized release version:
```bash
cargo build --release
```

The binary will be located at:
- Debug: `target/debug/vex`
- Release: `target/release/vex`

### Step 4: Install (Optional)

To install vex system-wide:
```bash
cargo install --path .
```

This will place the binary in your Cargo bin directory (usually `$HOME/.cargo/bin`).

## Running vex

After building, run vex with your GitHub token:

```bash
# Using command line argument
./target/release/vex --token YOUR_GITHUB_TOKEN

# Using environment variable
export GITHUB_TOKEN=YOUR_GITHUB_TOKEN
./target/release/vex
```

## Verifying Installation

To verify that vex is installed correctly, run:
```bash
vex --help
```

This should display the help message with available options.

## Updating vex

To update to the latest version:
```bash
git pull
cargo build --release
```

## Troubleshooting Installation

### Common Issues

**Missing OpenSSL headers**: Install libssl-dev (Ubuntu/Debian) or openssl-devel (Fedora)

**Linker errors**: Ensure you have build tools installed (build-essential, Xcode command line tools, etc.)

**Permission denied**: Make sure you have write permissions to the target directory

**Outdated Rust version**: Update rustc using `rustup update`

### Getting Help

If you encounter issues during installation:
1. Check the [Troubleshooting](troubleshooting.md) guide
2. Look for similar issues in the GitHub repository
3. File a new issue with details about your system and the error message