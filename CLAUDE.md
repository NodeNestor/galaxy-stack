# Galaxy Stack

## Architecture

```
Browser (static HTML from Cloudflare Pages)
    |
    +-- Astro islands (client-side JS only where needed)
    |
    +-- Plain HTTP to SpacetimeDB
        +-- POST /v1/database/{db}/sql        → reads (text/plain body)
        +-- POST /v1/database/{db}/call/{name} → writes (JSON array body)
        +-- POST /v1/identity                  → create auth identity
```

**No WebSocket. No polling. No SDK.** Fetch when you need data. Call reducers to write. Done.

## Stack

| Layer    | Tech                          | Purpose                        |
|----------|-------------------------------|--------------------------------|
| Frontend | Astro 5.7.5 (static output)   | HTML pages, Cloudflare Pages   |
| Styling  | Tailwind v4 (@tailwindcss/vite)| Utility CSS                    |
| Backend  | SpacetimeDB 2.0 (Rust WASM)   | Database + business logic      |
| Auth     | SpacetimeDB identity tokens    | JWT stored in localStorage     |
| Hosting  | Cloudflare Pages + any VPS     | Static CDN + DB server         |

## File Structure

```
server/src/lib.rs    — all tables + reducers (SpacetimeDB module)
web/src/lib/auth.ts  — identity token management (localStorage)
web/src/lib/api.ts   — query() for reads, call() for writes
web/src/pages/       — Astro pages (static HTML)
web/src/components/  — Astro islands (interactive UI)
web/src/layouts/     — page layouts
scripts/setup.sh     — one-command dev environment
scripts/publish.sh   — rebuild + republish WASM module
```

## Key Conventions

### SpacetimeDB (server/src/lib.rs)

- **Tables**: `#[spacetimedb::table(accessor = name, public)]` — always `public` for static sites
- **Primary keys**: `#[primary_key]` + `#[auto_inc]` for ID fields, `u64` type
- **Identity**: Use `ctx.sender()` (method, not field) for the caller's identity
- **Timestamp**: Use `ctx.timestamp` (field, not method) for current time
- **Validation**: Do it in reducers. `.trim()` strings, then check length. Return early with `log::error!()` on bad input. Never panic.
- **Status codes**: Use `u8` enums with comments (e.g. `0 = draft, 1 = active, 2 = sold`)
- **Owner checks**: Always verify `thing.owner == ctx.sender()` before mutations
- **Crate type**: Must be `cdylib` in Cargo.toml
- **Build target**: `wasm32-unknown-unknown`

### Frontend (web/src/)

- **Static output**: `output: 'static'` in astro.config.mjs — NO SSR
- **Reads**: Use `query('SELECT ...')` from `lib/api.ts` — no auth needed for public tables
- **Writes**: Use `call('reducer_name', [args])` from `lib/api.ts` — auto-includes auth
- **Auth**: Handled automatically by `lib/auth.ts` on first `call()` — creates identity if needed
- **No polling**: Fetch on page load + fetch after mutations. Never setInterval.
- **XSS prevention**: Always use `esc()` from `lib/api.ts` when inserting user content into HTML
- **Islands**: Use `<script>` in .astro files for client interactivity. Zero JS on read-only pages.
- **Tailwind v4**: Via `@tailwindcss/vite` plugin, imported in global.css as `@import "tailwindcss"`
- **Astro version**: Pinned to 5.7.5 (newer versions have tinyexec bug)
- **NO TEMPLATE LITERALS in Astro `<script>` tags** — esbuild chokes on backticks. Use string concatenation (`'a' + var + 'b'`) instead. Template literals work fine in standalone .ts files.

### Auth Flow

1. First `call()` triggers `getAuth()` → `POST /v1/identity` → stores JWT in localStorage
2. All subsequent `call()` includes `Authorization: Bearer <token>`
3. `ctx.sender()` in reducers returns the caller's identity
4. For OAuth: run OAuth client-side → call `link_oauth` reducer with provider info
5. SpacetimeDB identity = the real auth. OAuth = profile info linked to it.
6. Always validate identity with `isValidIdentity()` from `lib/auth.ts` before using in SQL queries

### Environment Variables

- `PUBLIC_STDB_URL` — SpacetimeDB URL (default: `http://localhost:3000`)
- `PUBLIC_STDB_DB` — Database name (default: `app`)
- Must be prefixed with `PUBLIC_` to be available in client-side Astro code

## Commands

```bash
# Dev environment (first time)
bash scripts/setup.sh

# Rebuild WASM after changing server/src/lib.rs
bash scripts/publish.sh

# Build static site for deployment
cd web && npm run build    # output in web/dist/

# Run Astro dev server (without Docker)
cd web && npm run dev
```

## Adding a New Table

1. Add struct in `server/src/lib.rs`:
   ```rust
   #[spacetimedb::table(accessor = thing, public)]
   pub struct Thing {
       #[primary_key]
       #[auto_inc]
       pub id: u64,
       pub owner: Identity,
       pub name: String,
       pub created_at: Timestamp,
   }
   ```
2. Add reducers (create, update, delete) with owner checks and `.trim()` validation
3. Run `bash scripts/publish.sh`
4. Query from frontend: `query('SELECT * FROM thing')`
5. Write from frontend: `call('create_thing', ['name'])`

## Adding a New Page

1. Create `web/src/pages/thing.astro`
2. Use Layout: `import Layout from '../layouts/Layout.astro'`
3. Add `<script>` island for interactivity (NO backtick template literals — use string concat)
4. Import `{ query, call, esc }` from `'../lib/api'`

## Adding OAuth (e.g. Google)

1. Add Google Sign-In script to Layout.astro `<head>`
2. On sign-in callback, call: `call('link_oauth', ['google:' + sub, email, name])`
3. The SpacetimeDB identity (already in localStorage) gets linked to Google profile
4. Check `oauth_link` field in user table to show profile info

## SpacetimeDB SQL Limitations

SpacetimeDB's SQL is **not full SQL**. Known unsupported features:
- `ORDER BY` — not supported, sort client-side in JS
- `LIMIT` / `OFFSET` — not supported
- `JOIN` — not supported, query tables separately
- `GROUP BY` / `HAVING` — not supported
- Subqueries — not supported
- `LIKE` / `ILIKE` — not supported, filter client-side

What works: `SELECT`, `WHERE`, basic comparisons (`=`, `!=`, `<`, `>`), `AND`/`OR`.

## SpacetimeDB HTTP API Reference

| Operation       | Method | Endpoint                              | Content-Type     | Body             |
|-----------------|--------|---------------------------------------|------------------|------------------|
| SQL query       | POST   | `/v1/database/{db}/sql`               | `text/plain`     | SQL string       |
| Call reducer    | POST   | `/v1/database/{db}/call/{reducer}`    | `application/json` | JSON array     |
| Create identity | POST   | `/v1/identity`                        | —                | —                |
| Health check    | GET    | `/v1/ping`                            | —                | —                |

## Windows Notes

- Use `MSYS_NO_PATHCONV=1` before docker exec commands in Git Bash
- Scripts handle this automatically
