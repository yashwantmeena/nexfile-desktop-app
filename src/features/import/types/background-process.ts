export type BackgroundProcessStatus =
  | "queued"
  | "running"
  | "paused"
  | "completed"
  | "failed"
  | "cancelled";

export interface BackgroundProcess {
  processId: string;
  processType: string;
  status: BackgroundProcessStatus;
  priority: number;
  totalItems: number;
  processedItems: number;
  failedItems: number;
  remark: string | null;
  createdAtMs: number;
  updatedAtMs: number;
  startedAtMs: number | null;
  finishedAtMs: number | null;
}
