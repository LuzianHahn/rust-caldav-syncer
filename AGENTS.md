# AGENTS.md

This file provides guidance to agents when working with code in this repository.

## Non-Obvious Project Patterns
- Config uses YAML format (not JSON/TOML) for human readability in Termux environment
- Sync operates on user-specified folders from config, not automatic photo discovery
- Target platform is Android via Termux, requiring awareness of storage paths like /storage/emulated/0/
- Incremental sync uses SHA256 hashes stored locally to track changes

## Commands
- Build for Android ARM: `cargo build --release` (Termux provides native Rust)
- Single test run: `cargo test -- --test <test_name>`
- Cross-compile if needed: `cross build --target aarch64-linux-android --release`
- Run unit tests: `cargo test`
- Run integration tests: `cargo test --test sync_integration` (starts dummy WebDAV server automatically)