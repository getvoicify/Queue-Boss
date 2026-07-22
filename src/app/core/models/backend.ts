export interface BackendInfo {
  name: string;
  healthy: boolean;
  detail: string | null;
}

export interface Capabilities {
  priority: boolean;
  singleton: boolean;
  deadLetter: boolean;
  extensions: string[];
}

export type CommandErrorKind =
  | "connection"
  | "unsupported"
  | "notFound"
  | "internal";

export interface CommandError {
  kind: CommandErrorKind;
  message: string;
}
