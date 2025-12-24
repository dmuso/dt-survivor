# Donny Tango: Survivor

A game in the style of Vampire Survivors and Brotato, built with Rust and the Bevy ECS framework.

## Development Setup

This project uses Nix for development environment management.

### Prerequisites

- [Nix](https://nixos.org/download.html) package manager

### Getting Started

1. Enter the Nix shell:
   ```bash
   nix-shell
   ```

2. Build and run the game:
   ```bash
   cargo run
   ```

## Project Structure

- `src/` - Rust source code
- `shell.nix` - Nix shell configuration
- `Cargo.toml` - Rust dependencies

## Building

```bash
cargo build
```

## Running

```bash
cargo run
```

## Testing

```bash
cargo test
```