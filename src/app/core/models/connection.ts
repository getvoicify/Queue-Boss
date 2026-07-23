export type ConnectionStatus = "idle" | "connecting" | "connected" | "error";

export interface ConnectionEntry {
  id: string;
  status: ConnectionStatus;
  message?: string;
}
