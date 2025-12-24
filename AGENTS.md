# Donny Tango: Survivor

A game in the style of Vampire Survivors and Brotato, built with Rust and the Bevy ECS framework.

## Development Commands

All development tooling commands require Nix Shell to run.

- Type Checking: `nix-shell --run "cargo check"`
- Linting: `nix-shell --run "cargo clippy"`
- Testing: `nix-shell --run "cargo test"`
- Building: `nix-shell --run "cargo build"`
- Running: `nix-shell --run "cargo run"`

## Testing and Linting

- You should maintain 90% code coverage via automated tests
- Run linting and testing after every change
- Fix any errors or warnings that you get as feedback from linting and tests
