/**
 * SpacetimeDB Identity Auth
 *
 * Flow:
 * 1. On first visit -> POST /v1/identity -> get { identity, token }
 * 2. Store token in localStorage
 * 3. Include Authorization: Bearer <token> on all reducer calls
 * 4. ctx.sender() in reducers = this identity
 *
 * For OAuth (Google, GitHub, etc.):
 * 1. Do normal SpacetimeDB auth first (get identity)
 * 2. Run OAuth flow client-side (Google Sign-In, etc.)
 * 3. Call link_oauth reducer with provider info
 * 4. SpacetimeDB identity is now linked to the OAuth account
 *
 * NOTE: Every `publish` (WASM module update) invalidates ALL tokens.
 * Users will need to re-authenticate after deploys.
 */

const TOKEN_KEY = 'stdb_token';
const IDENTITY_KEY = 'stdb_identity';

export interface AuthState {
  token: string;
  identity: string;
}

/** Get stored auth or create a new identity. */
export async function getAuth(apiBase: string): Promise<AuthState> {
  const stored = loadAuth();
  if (stored) return stored;
  return createIdentity(apiBase);
}

/** Create a new SpacetimeDB identity. */
async function createIdentity(apiBase: string): Promise<AuthState> {
  const res = await fetch(apiBase + '/v1/identity', { method: 'POST' });
  if (!res.ok) throw new Error('Failed to create identity: ' + res.status);
  const data = await res.json();
  if (!data.token || !data.identity) {
    throw new Error('Invalid identity response');
  }
  const auth: AuthState = { token: data.token, identity: data.identity };
  saveAuth(auth);
  return auth;
}

/** Load auth from localStorage. */
function loadAuth(): AuthState | null {
  try {
    const token = localStorage.getItem(TOKEN_KEY);
    const identity = localStorage.getItem(IDENTITY_KEY);
    if (token && identity) return { token, identity };
  } catch {}
  return null;
}

/** Save auth to localStorage. */
function saveAuth(auth: AuthState): void {
  try {
    localStorage.setItem(TOKEN_KEY, auth.token);
    localStorage.setItem(IDENTITY_KEY, auth.identity);
  } catch {}
}

/** Clear stored auth (logout). Also use after detecting an invalidated token. */
export function clearAuth(): void {
  try {
    localStorage.removeItem(TOKEN_KEY);
    localStorage.removeItem(IDENTITY_KEY);
  } catch {}
}

/** Get identity from localStorage without network call. Returns null if not authenticated. */
export function getStoredIdentity(): string | null {
  try {
    return localStorage.getItem(IDENTITY_KEY);
  } catch {
    return null;
  }
}

/** Validate that a string looks like a hex identity (prevents SQL injection). */
export function isValidIdentity(s: string): boolean {
  return /^[0-9a-f]{64}$/i.test(s);
}
