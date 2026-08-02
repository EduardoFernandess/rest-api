# rest-api

Axum REST API with PostgreSQL (SQLx), JWT authentication, per-user item CRUD, and a simple per-IP rate limit.

## Stack

- **Rust / Tokio / Axum** — async HTTP service
- **SQLx + PostgreSQL** — typed SQL and migrations
- **jsonwebtoken + Argon2** — bearer tokens and password hashing
- **tower-http / tracing** — CORS, request tracing
- **Docker Compose** — app + database for local demos

## What was built

- Register and login endpoints that issue JWT access tokens
- Protected item CRUD scoped to the authenticated user
- Health check for orchestration probes
- Per-IP rate limiting middleware
- SQL migration for users and items
- Unit tests for auth helpers and middleware behavior

## Run locally

```bash
cp .env.example .env
docker compose up -d db
cargo run
```

API listens on `http://localhost:3000` by default.

### Example requests

```bash
curl localhost:3000/health

curl -X POST localhost:3000/auth/register \
  -H 'content-type: application/json' \
  -d '{"email":"you@example.com","password":"password123"}'

TOKEN=$(curl -s -X POST localhost:3000/auth/login \
  -H 'content-type: application/json' \
  -d '{"email":"you@example.com","password":"password123"}' | jq -r .token)

curl localhost:3000/items -H "authorization: Bearer $TOKEN"
curl -X POST localhost:3000/items \
  -H "authorization: Bearer $TOKEN" \
  -H 'content-type: application/json' \
  -d '{"title":"notes","body":"hello"}'
```

### Full stack with Docker

```bash
docker compose up --build
```

### Tests

```bash
cargo test
```
