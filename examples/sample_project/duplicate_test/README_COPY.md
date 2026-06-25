# Sample Project

A sample web application for testing SRR Context Compressor.

## Purpose

Demonstrate SRR's ability to analyze, compress, and summarize project context.

## Installation

```bash
git clone https://github.com/example/sample-project
cd sample-project
cargo build
```

## Architecture

The application follows a layered architecture:
- Frontend → API → Services → Database
- Authentication layer cross-cuts all layers

## Key Features

- User authentication with OAuth2
- RESTful API with CRUD operations
- PostgreSQL database with migrations
- Service-oriented business logic

## Known Limitations

- No caching layer
- Rate limiting is basic
- Documentation is incomplete
