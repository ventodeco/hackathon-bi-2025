# Backend Project

A Rust-based backend API with authentication using JWT.

## Features

- User registration and login
- JWT-based authentication
- PostgreSQL database
- Docker support
- GitHub Actions CI/CD

## Prerequisites

- Rust (latest stable)
- Docker and Docker Compose
- PostgreSQL 14+
- SQLx CLI (`cargo install sqlx-cli`)

## Setup

1. Clone the repository
2. Copy `.env.example` to `.env` and update the values
3. Start the database:
```bash
docker-compose up -d db
```

4. Run database migrations:
```bash
sqlx migrate run
```

5. Start the application:
```bash
docker-compose up -d
```

## Database Migrations

The project uses SQLx for database migrations. Here are the common commands:

```bash
# Create a new migration
sqlx migrate add <migration_name>

# Run pending migrations
sqlx migrate run

# Revert the last migration
sqlx migrate revert

# Create the database
sqlx database create

# Drop the database
sqlx database drop
```

# prepare the query that we use
```
cargo sqlx prepare
```

## API Endpoints

### Register User
```
POST /v1/auth/register
Content-Type: application/json

{
  "email": "user@example.com",
  "password": "yourpassword",
  "name": "John Doe"
}
```

### Login
```
POST /v1/auth/login
Content-Type: application/json

{
  "email": "user@example.com",
  "password": "yourpassword"
}
```

## Development

1. Install dependencies:
```bash
cargo build
```

2. Run the application:
```bash
cargo run
```

## Testing

```bash
cargo test
```

## Docker

Build and run with Docker Compose:
```bash
docker-compose up --build
```

## License

MIT 