# Galaxy Stack

## Architecture

```
Browser (static HTML from Cloudflare Pages)
    |
    +-- Astro islands (client-side JS only where needed)
    |
    +-- Plain HTTP to SpacetimeDB
        +-- POST /v1/database/{db}/call/{name} → reads (procedures) + writes (reducers)
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
server/src/lib.rs    — all tables, reducers, and procedures (SpacetimeDB module)
worker/src/          — search worker (in-memory index, file uploads, HTTP API)
web/src/lib/auth.ts  — identity token management (localStorage)
web/src/lib/api.ts   — proc() for reads, call() for writes (no raw SQL — query() removed)
web/src/pages/       — Astro pages (static HTML)
web/src/components/  — Astro islands (interactive UI)
web/src/layouts/     — page layouts
scripts/setup.sh     — one-command dev environment
scripts/publish.sh   — rebuild + republish WASM module
```

## Key Conventions

### SpacetimeDB (server/src/lib.rs)

- **Tables**: `#[spacetimedb::table(accessor = name, public)]` — use `accessor`, NOT `name`
- **Private tables**: `#[spacetimedb::table(accessor = name)]` — omit `public` for tables that should not be queryable via SQL endpoint
- **Primary keys**: `#[primary_key]` + `#[auto_inc]` for ID fields, `u64` type
- **Identity**: Use `ctx.sender()` (method, not field) for the caller's identity
- **Timestamp**: Use `ctx.timestamp` (field, not method) for current time
- **Validation**: Do it in reducers. `.trim()` strings, then check length. Return early with `log::error!()` on bad input. Never panic.
- **Status codes**: Use `u8` enums with comments (e.g. `0 = draft, 1 = active, 2 = sold`)
- **Owner checks**: Always verify `thing.owner == ctx.sender()` before mutations
- **Crate type**: Must be `cdylib` in Cargo.toml
- **Build target**: `wasm32-unknown-unknown`

### Procedures

SpacetimeDB has **procedures** (`#[spacetimedb::procedure]`) that can return data to the caller.

```rust
#[spacetimedb::procedure]
pub fn get_user_stats(ctx: &ProcedureContext) -> Vec<u8> {
    let caller = ctx.sender();
    let stats = ctx.with_tx(|tx| {
        // access db tables via tx
    });
    serde_json::to_vec(&stats).unwrap_or_default()
}
```

- Use `ProcedureContext` (not `ReducerContext`)
- Access the database via `ctx.with_tx(|tx| { ... })`
- Called via the same HTTP endpoint as reducers: `POST /v1/database/{db}/call/{name}`
- Can return data (unlike reducers which are fire-and-forget)
- Use for authenticated reads where SQL queries on public tables are insufficient

### Frontend (web/src/)

- **Static output**: `output: 'static'` in astro.config.mjs — NO SSR
- **Reads**: Use `proc('name', [args])` or `procAuth('name', [args])` from `lib/api.ts` — NEVER use raw SQL
- **No raw SQL**: `query()` has been removed. All reads go through server-side procedures.
- **Writes**: Use `call('reducer_name', [args])` from `lib/api.ts` — auto-includes auth
- **Procedures**: Use `proc('name', [args])` or `procAuth('name', [args])` from `lib/api.ts`
- **Auth**: Handled automatically by `lib/auth.ts` on first `call()` — creates identity if needed
- **No polling**: Fetch on page load + fetch after mutations. Never setInterval.
- **XSS prevention**: Always use `esc()` from `lib/api.ts` when inserting user content into HTML
- **SQL injection prevention**: Use `sanitizeHex()`, `sanitizeSlug()`, `sanitizeId()` from `lib/api.ts` before interpolating values into SQL
- **Islands**: Use `<script>` in .astro files for client interactivity. Zero JS on read-only pages.
- **Tailwind v4**: Via `@tailwindcss/vite` plugin, imported in global.css as `@import "tailwindcss"`
- **Astro version**: Pinned to 5.7.5 (newer versions have tinyexec bug)
- **NO TEMPLATE LITERALS in Astro `<script>` tags** — esbuild chokes on backticks. Use string concatenation (`'a' + var + 'b'`) instead. Template literals work fine in standalone .ts files.

### Auth Flow

1. First `call()` triggers `getAuth()` -> `POST /v1/identity` -> stores JWT in localStorage
2. All subsequent `call()` includes `Authorization: Bearer <token>`
3. `ctx.sender()` in reducers returns the caller's identity
4. For OAuth: run OAuth client-side -> call `link_oauth` reducer with provider info
5. SpacetimeDB identity = the real auth. OAuth = profile info linked to it.
6. Always validate identity with `isValidIdentity()` from `lib/auth.ts` before using in SQL queries
7. **Every `publish` invalidates ALL existing JWT tokens** — users must re-authenticate

### Security Patterns

- **Verify sender exists**: Always check that `ctx.sender()` has a row in the user table before allowing mutations
- **Owner checks**: `thing.owner == ctx.sender()` before updates/deletes
- **Rate limiting**: Use a `rate_limit` table with `identity` + `last_action_at` to throttle expensive operations
- **Input validation**: `.trim()` all strings, check lengths, validate enums against allowlists
- **Constant-time comparison**: Never use `==` for password hashes — use `constant_time_eq` or similar
- **Private tables**: Omit `public` from `#[spacetimedb::table]` for sensitive data — private tables cannot be queried via the SQL endpoint
- **Frontend sanitization**: Use `sanitizeHex()` and `sanitizeSlug()` from `lib/api.ts` to prevent SQL injection when interpolating user input into SQL strings
- **Admin checks**: Use a `role` field on the user table and verify role before admin operations

### Performance

- **Use LIMIT for pagination**: `SELECT * FROM thing WHERE id > {cursor} LIMIT {page_size}`
- **Use COUNT(*)**: `SELECT COUNT(*) FROM thing WHERE ...` instead of fetching all rows just to count
- **Select only needed columns**: `SELECT id, name FROM thing` — never `SELECT *` from frontend
- **Cursor-based pagination**: Use `WHERE id > {last_id} LIMIT N` since OFFSET is not supported
- **In-memory indexing**: For search/filtering at scale, use a worker process that maintains in-memory indexes
- **Incremental sync**: Track `updated_at` on tables, only fetch changes since last sync

### Docker

- **Never use `depends_on` between services with persistent volumes** — recreating the dependency recreates volumes and wipes data
- **Use `restart: unless-stopped`** on all services for automatic recovery
- **Every publish invalidates ALL existing JWT tokens** — plan for re-authentication after deploys

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
3. Add a procedure to return data: `#[spacetimedb::procedure] pub fn get_things(...) -> String`
4. Run `bash scripts/publish.sh`
5. Read from frontend: `proc('get_things')` or `procAuth('get_my_things')`
6. Write from frontend: `call('create_thing', ['name'])`

## Adding a New Page

1. Create `web/src/pages/thing.astro`
2. Use Layout: `import Layout from '../layouts/Layout.astro'`
3. Add `<script>` island for interactivity (NO backtick template literals — use string concat)
4. Import `{ proc, call, esc }` from `'../lib/api'`

## Adding OAuth (e.g. Google)

1. Add Google Sign-In script to Layout.astro `<head>`
2. On sign-in callback, call: `call('link_oauth', ['google:' + sub, email, name])`
3. The SpacetimeDB identity (already in localStorage) gets linked to Google profile
4. Check `oauth_link` field in user table to show profile info

## SpacetimeDB SQL Support

**Supported:** `SELECT`, `WHERE`, `JOIN`, `LIMIT`, `COUNT(*)`, basic comparisons (`=`, `!=`, `<`, `>`), `AND`/`OR`, hex identity literals (`X'...'`).

**NOT supported:** `ORDER BY` (sort client-side), `GROUP BY` / `HAVING`, `LIKE` / `ILIKE` (filter client-side), `OFFSET` (use cursor pagination), subqueries.

## SpacetimeDB HTTP API Reference

| Operation          | Method | Endpoint                              | Content-Type       | Body           |
|--------------------|--------|---------------------------------------|--------------------|----------------|
| SQL query          | POST   | `/v1/database/{db}/sql`               | `text/plain`       | SQL string     |
| Call reducer       | POST   | `/v1/database/{db}/call/{reducer}`    | `application/json` | JSON array     |
| Call procedure     | POST   | `/v1/database/{db}/call/{procedure}`  | `application/json` | JSON array     |
| Create identity    | POST   | `/v1/identity`                        | —                  | —              |
| Health check       | GET    | `/v1/ping`                            | —                  | —              |

## Windows Notes

- Use `MSYS_NO_PATHCONV=1` before docker exec commands in Git Bash
- Scripts handle this automatically
