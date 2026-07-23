// Mirrors the Rust `PgConnectConfig` (spec §3.9), serialized camelCase on the
// wire: a union of a full connection string OR discrete fields.
export type PgConnectConfig =
  | { connectionString: string }
  | {
      host: string;
      port: number;
      database: string;
      user: string;
      password: string;
      sslMode: string;
      schema?: string;
    };
