# Architecture

## System Design

The system uses a layered architecture with the following components:

### Frontend
React-based single page application.

### API Layer
RESTful API built with Axum in Rust.

### Services
Business logic layer handling all application operations.

### Database
PostgreSQL for persistent storage with SQL migrations.

## Data Flow

User → Frontend → API → Services → Database
