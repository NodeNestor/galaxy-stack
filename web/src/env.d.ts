/// <reference types="astro/client" />

interface ImportMetaEnv {
  readonly PUBLIC_STDB_URL: string;
  readonly PUBLIC_STDB_DB: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
