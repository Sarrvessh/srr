# Setup Guide

## Prerequisites
- Rust 1.70+
- PostgreSQL 15+

## Installation Steps

1. Clone the repository
2. Run `cargo build`
3. Set up database with `cargo run --bin migrate`
4. Start the server with `cargo run`

## Configuration

Copy `config/app.toml.example` to `config/app.toml` and adjust settings.
