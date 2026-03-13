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
// SQL Reads — public tables, no auth needed
// ---------------------------------------------------------------------------

/** Run a SQL query against the database. Returns rows from the first result. */
export async function query<T = any[]>(sql: string): Promise<T[]> {
  const res = await fetch(API_BASE + '/v1/database/' + DB_NAME + '/sql', {
    method: 'POST',
    headers: { 'Content-Type': 'text/plain' },
    body: sql,
  });
  if (!res.ok) throw new Error('SQL query failed: ' + res.status);
  const data = await res.json();
  if (Array.isArray(data) && data.length > 0 && data[0]?.rows) {
    return data[0].rows;
  }
  return [];
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
// Helpers
// ---------------------------------------------------------------------------

/** Escape HTML to prevent XSS. */
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
