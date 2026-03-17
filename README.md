# Galaxy Stack

Production-ready starter template for SpacetimeDB + Astro applications. Static HTML frontend, Rust WASM backend, in-memory search worker. Built for AI-assisted development.

## Architecture

```
Browser (Cloudflare Pages)
    |
    +-- Static HTML + Astro islands (JS only where needed)
    |
    +-- HTTP to SpacetimeDB (database + auth + business logic)
    |
    +-- HTTP to Worker (search + file uploads)
```

No WebSocket. No polling. No SDK. No SSR.

## What's Included

- **SpacetimeDB server** with auth, rate limiting, owner checks, procedures (no raw SQL from frontend)
- **Astro 5 static frontend** with Tailwind v4, dark/light theme, 4 example pages
- **Search worker** with in-memory text index, numeric/geo filters, file uploads, incremental sync
- **Docker Compose** for local development (SpacetimeDB + Astro + Worker)
- **Security patterns** — XSS prevention, input validation, identity-based auth, rate limiting
- **CLAUDE.md** — comprehensive AI coder instructions for every convention and pattern

## Quick Start

**Prerequisites:** Docker, Rust (`rustup target add wasm32-unknown-unknown`)

```bash
git clone <this-repo> my-app
cd my-app
bash scripts/setup.sh
```

Open **http://localhost:4321**. You have a working app with auth, CRUD, search, and styling.

## Stack

| Layer   | Tech                           | Purpose                      |
|---------|--------------------------------|------------------------------|
| Frontend| Astro 5.7.5 (static output)    | HTML pages, Cloudflare Pages |
| Styling | Tailwind v4 (@tailwindcss/vite)| Utility CSS                  |
| Backend | SpacetimeDB 2.0 (Rust WASM)    | Database + business logic    |
| Worker  | Rust (axum + tokio)            | Search index + file uploads  |
| Auth    | SpacetimeDB identity tokens    | JWT stored in localStorage   |
| Hosting | Cloudflare Pages + any VPS     | Static CDN + DB server       |

## Project Structure

```
server/src/lib.rs       — all tables, reducers, and procedures (SpacetimeDB WASM module)
worker/src/main.rs      — worker entry point (sync loop, HTTP server, graceful shutdown)
worker/src/index.rs     — generic in-memory search index (text, numeric, geo, pagination)
worker/src/search.rs    — POST /search endpoint
worker/src/upload.rs    — POST /upload + GET /files/{name} endpoints
worker/src/health.rs    — GET /health endpoint
web/src/lib/api.ts      — proc(), procAuth(), call() HTTP helpers
web/src/lib/auth.ts     — identity token management (localStorage)
web/src/pages/          — Astro pages (Home, Discoveries, Login, Profile)
web/src/layouts/        — shared layout with nav, theme toggle, auth display
web/src/styles/         — Tailwind + space color palette
scripts/setup.sh        — one-command dev setup (Docker + build + publish)
scripts/publish.sh      — rebuild + republish WASM after server changes
CLAUDE.md               — AI coder instructions (conventions, patterns, gotchas)
docker-compose.yml      — local dev environment
```

## Development

| Command                    | What it does                                    |
|----------------------------|-------------------------------------------------|
| `bash scripts/setup.sh`   | First-time setup (Docker + build WASM + publish) |
| `bash scripts/publish.sh` | Rebuild + republish WASM after server changes    |
| `docker compose up -d`    | Start all services                               |
| `docker compose down -v`  | Stop and wipe all data                           |
| `cd web && npm run build` | Build static site for deployment (`web/dist/`)   |
| `cd worker && cargo build --release` | Build worker binary                  |

### How Data Flows

```
Page load → proc('get_data') → SpacetimeDB procedure → JSON response → render
User action → call('reducer', [args]) → SpacetimeDB reducer → re-fetch data
Search → POST /search to worker → in-memory index → JSON results
Upload → POST /upload to worker → saved to disk → GET /files/{name}
```

## Deployment

**Frontend:** Build with `cd web && npm run build`, deploy `web/dist/` to Cloudflare Pages. Set `PUBLIC_STDB_URL` to your SpacetimeDB server URL.

**Backend:** Run SpacetimeDB on any VPS. Publish the WASM module with `scripts/publish.sh`.

**Worker:** Build with `cd worker && cargo build --release`. Run the binary on the same VPS. Set `STDB_URL`, `STDB_DB`, `UPLOAD_DIR`, and `PORT` environment variables.

## Security

- **Auth:** SpacetimeDB identity tokens (JWT). Created on first interaction, stored in localStorage. `ctx.sender()` in reducers identifies the caller.
- **No raw SQL from frontend:** All reads go through procedures (`proc()` / `procAuth()`), not direct SQL queries.
- **Owner checks:** Every mutation verifies `thing.owner == ctx.sender()` before allowing changes.
- **Rate limiting:** `rate_limit` table throttles expensive operations per identity.
- **Input validation:** All strings trimmed and length-checked in reducers. Enums validated against allowlists.
- **XSS prevention:** `esc()` helper escapes all user content before DOM insertion.
- **OAuth-ready:** Google and GitHub sign-in scaffolded. Link to SpacetimeDB identity via `link_oauth` reducer.

## Cost

| Scale  | Monthly                                    |
|--------|--------------------------------------------|
| Hobby  | ~5 EUR (1 VPS + free Cloudflare Pages)     |
| Small  | ~15 EUR (bigger VPS)                       |
| Growth | ~50+ EUR (multiple nodes, more storage)    |

## License

MIT
