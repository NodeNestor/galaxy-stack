/**
 * SpacetimeDB HTTP Client
 *
 * Plain HTTP — no WebSocket, no SDK, no polling.
 * Fetch when you need data. Call reducers to write. That's it.
 */

import { getAuth, type AuthState } from './auth';

// Configure these for your environment
const API_BASE = import.meta.env.PUBLIC_STDB_URL || 'http://localhost:3000';
const DB_NAME = import.meta.env.PUBLIC_STDB_DB || 'app';

let authCache: AuthState | null = null;

/** Get auth (cached after first call). */
async function auth(): Promise<AuthState> {
  if (!authCache) authCache = await getAuth(API_BASE);
  return authCache;
}

// ---------------------------------------------------------------------------
// Reducer Calls — writes, requires auth
// ---------------------------------------------------------------------------

/** Call a reducer with arguments. Automatically includes auth token. */
export async function call(reducer: string, args: any[] = []): Promise<boolean> {
  const { token } = await auth();
  const res = await fetch(API_BASE + '/v1/database/' + DB_NAME + '/call/' + reducer, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': 'Bearer ' + token,
    },
    body: JSON.stringify(args),
  });
  return res.ok;
}

// ---------------------------------------------------------------------------
// Procedure Calls — can return data
// ---------------------------------------------------------------------------

/** Call a procedure (no auth). Returns parsed JSON response. */
export async function proc<T = any>(name: string, args: any[] = []): Promise<T> {
  const res = await fetch(API_BASE + '/v1/database/' + DB_NAME + '/call/' + name, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(args),
  });
  if (!res.ok) throw new Error('Procedure call failed: ' + res.status);
  return res.json();
}

/** Call a procedure with auth. Returns parsed JSON response. */
export async function procAuth<T = any>(name: string, args: any[] = []): Promise<T> {
  const { token } = await auth();
  const res = await fetch(API_BASE + '/v1/database/' + DB_NAME + '/call/' + name, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': 'Bearer ' + token,
    },
    body: JSON.stringify(args),
  });
  if (!res.ok) throw new Error('Procedure call failed: ' + res.status);
  return res.json();
}

// ---------------------------------------------------------------------------
// Sanitization — prevent SQL injection
// ---------------------------------------------------------------------------

/** Validate and return a hex string (for identity values in SQL). Throws on invalid input. */
export function sanitizeHex(s: string): string {
  if (!/^[0-9a-f]+$/i.test(s)) throw new Error('Invalid hex string');
  return s;
}

/** Validate and return a slug (alphanumeric, hyphens, underscores). Throws on invalid input. */
export function sanitizeSlug(s: string): string {
  if (!/^[a-z0-9_-]+$/i.test(s)) throw new Error('Invalid slug');
  return s;
}

/** Validate and return a numeric ID string. Throws on invalid input. */
export function sanitizeId(s: string): string {
  if (!/^\d+$/.test(s)) throw new Error('Invalid ID');
  return s;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Escape HTML to prevent XSS. Use this whenever inserting user content into the DOM. */
export function esc(s: string): string {
  const d = document.createElement('div');
  d.textContent = s;
  return d.innerHTML;
}

/** Get the current user's identity string. */
export async function getIdentity(): Promise<string> {
  const { identity } = await auth();
  return identity;
}
