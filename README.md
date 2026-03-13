# Galaxy Stack

A zero-fluff web stack for building fast, cheap, static-first web apps. Built for AI-assisted development.

**Astro + SpacetimeDB + Tailwind. No WebSocket. No polling. No SDK. No SSR.**

```
Static HTML (Cloudflare Pages, free)  ←→  SpacetimeDB (single VPS, ~€5/mo)
```

## Why Galaxy?

Most stacks are bloated. You don't need a Node.js server, an ORM, a REST framework, WebSocket subscriptions, and a separate database. Galaxy replaces all of that with two things:

- **Astro** — builds static HTML. Deploys to any CDN. Perfect SEO. Islands for interactivity.
- **SpacetimeDB** — your database AND your server. Business logic in Rust WASM. Auth built in.

That's the whole stack. Your frontend is static files. Your backend is a single Rust file.

## What's Included

- 4 pages — Home, Items (CRUD), Login, Profile
- Auth system — SpacetimeDB identity tokens, OAuth-ready (Google, GitHub)
- Tailwind v4 — clean, modern styling out of the box
- CLAUDE.md — complete AI coder instructions
- Docker dev environment — one command to run everything

## Quick Start

**Prerequisites:** Docker, Rust (`rustup target add wasm32-unknown-unknown`)

```bash
git clone <this-repo> my-app
cd my-app
bash scripts/setup.sh
```

Open **http://localhost:4321**. You have a working app with auth, CRUD, and styling.

## How It Works

```
Browser loads static HTML from CDN
  │
  ├── Page load → fetch('SQL query') → render data
  │
  ├── User action → fetch('call reducer') → re-fetch data
  │
  └── Auth: POST /v1/identity → JWT in localStorage → auto-included on writes
```

There are only two functions:

```typescript
// Read data (no auth needed for public tables)
const rows = await query('SELECT * FROM item');

// Write data (auth auto-included)
await call('create_item', ['Title', 'Description']);
```

## Auth

Auth is automatic and invisible:

1. User clicks "Sign in" → SpacetimeDB creates an identity (JWT)
2. Token stored in `localStorage`, auto-included on every write
3. `ctx.sender()` in Rust reducers = the authenticated user
4. Owner checks prevent unauthorized mutations

**Adding Google/GitHub login:**

1. Add the OAuth provider's client-side SDK
2. On callback: `call('link_oauth', ['google:' + sub, email, name])`
3. Done — no auth server needed

## Project Structure

```
server/src/lib.rs       ← the entire backend (tables + reducers)
web/src/lib/auth.ts     ← token management (automatic)
web/src/lib/api.ts      ← query() and call() helpers
web/src/pages/          ← Astro pages (static HTML)
web/src/components/     ← interactive islands
web/src/layouts/        ← shared layout with nav + auth
scripts/setup.sh        ← one-command dev setup
scripts/publish.sh      ← rebuild WASM after server changes
CLAUDE.md               ← AI coder instructions
```

## Commands

| Command | What it does |
|---------|--------------|
| `bash scripts/setup.sh` | First-time setup (Docker + build + publish) |
| `bash scripts/publish.sh` | Rebuild + republish WASM after server changes |
| `cd web && npm run build` | Build static site → `web/dist/` |
| `docker compose up -d` | Start dev containers |
| `docker compose down -v` | Stop + wipe data |

## Deploy

**Frontend:** `cd web && npm run build` → push `dist/` to Cloudflare Pages. Set `PUBLIC_STDB_URL` env var to your SpacetimeDB server.

**Backend:** SpacetimeDB on any VPS. Publish the WASM module. That's it.

## Cost

| Scale | Monthly |
|-------|---------|
| Hobby | ~€5 (1 VPS + free Cloudflare Pages) |
| Small | ~€15 (bigger VPS) |
| Growth | ~€50+ (multiple nodes) |

## Built for AI Coders

Galaxy includes a comprehensive `CLAUDE.md` with every convention, pattern, and gotcha. Any AI assistant that reads project files will immediately know how to add tables, create pages, handle auth, and avoid pitfalls.

## License

MIT
