# Sample Project Documentation

A sample web app for testing SRR Context Compressor tool.

## About

Demonstrate SRR's capability to analyze, compress, and summarize project contexts.

## Getting Started

```bash
git clone https://github.com/example/sample-project
cd sample-project
cargo build --release
```

## System Architecture

Layered architecture design:
- Frontend (React) → API (Rust) → Services → Database (PostgreSQL)
- Authentication layer cross-cuts all architecture layers

## Capabilities

- User authentication with OAuth2 providers
- RESTful API with full CRUD operations
- PostgreSQL database with automated migrations
- Service-oriented business logic layer

## Current Gaps

- No caching layer implemented
- Rate limiting is basic
- Documentation needs improvement
