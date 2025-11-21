# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

**IMPORTANT**: Always re-read this entire file after making any changes to it to ensure you understand and follow all current standards and workflows.

## Project Overview

This is a Docker TUI (Terminal User Interface) application written in Rust. The repository is currently in its initial stage with only a README.md file present.

## Development Setup

This appears to be a Rust project for building a Docker terminal user interface. The project structure suggests it will use Cargo for dependency management and building.

## Architecture

The project is in early development stage. Based on the repository name "docker-tui", this will likely be a terminal-based interface for managing Docker containers, images, and other Docker resources.

## Development Workflow

This project uses Stacked Git (stg) for patch management. For every command or task:

1. **Start with a new patch**: `stg new <patch-name>` - Create a new, empty patch at the top of the stack
2. **Make your changes**: Use normal git commands to add/modify files
3. **Format code**: Always run `cargo fmt` after writing code
4. **Check code**: Always run `cargo check` and `cargo build` to ensure no warnings or errors
5. **Refresh the patch**: `stg refresh` - Include all changes in the current patch
6. **Add description**: `stg edit` - Add an appropriate description to the patch

## Code Quality Standards

- Always run `cargo fmt` after writing code
- Always run `cargo check` and `cargo build` when making changes, ensuring no warnings or errors
- Always put a period (".") at the end of one-line commit messages and patch messages
- When writing `use` statements, always group imports from the same crate using common prefixes. For example:
  - Instead of: `use std::io::{self, stdout};\nuse std::time::Duration;`
  - Use: `use std::{io::{self, stdout}, time::Duration};`

## Common Commands

Since this is a Rust project, typical commands would be:
- `cargo build` - Build the project
- `cargo run` - Run the application
- `cargo test` - Run tests
- `cargo clippy` - Run the Rust linter
- `cargo fmt` - Format code

## Stacked Git Commands

- `stg series` - Show all patches in the stack
- `stg new <name>` - Create a new patch
- `stg refresh` - Update current patch with changes
- `stg edit` - Edit patch description
- `stg push/pop` - Move patches up/down the stack
