/**
 * Backup and restore.
 *
 * Restoring is deliberately two steps: staging parks the chosen file, and the
 * swap happens at the next startup before anything opens the database. The UI
 * must say so plainly — an owner who thinks a restore already happened and
 * keeps working would lose that work when it does.
 */

import { call } from "@/lib/tauri";

export interface BackupInfo {
  path: string;
  fileName: string;
  createdAt: string;
  sizeBytes: number;
  candidates: number;
  payments: number;
}

export interface BackupStatus {
  backups: BackupInfo[];
  folder: string;
  pendingRestore: string | null;
}

export const backupStatus = () => call<BackupStatus>("backup_status");
export const backupNow = () => call<BackupInfo>("backup_now");
export const backupTo = (path: string) => call<BackupInfo>("backup_to", { path });
export const inspectBackup = (path: string) => call<BackupInfo>("inspect_backup", { path });
export const stageRestore = (path: string) => call<BackupInfo>("stage_restore", { path });
export const cancelRestore = () => call<void>("cancel_restore");

/** "1.4 MB" — display only. */
export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/** ISO timestamp to "18/10/2026 14:32". */
export function formatWhen(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${pad(d.getDate())}/${pad(d.getMonth() + 1)}/${d.getFullYear()} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

/** Default filename offered when saving to a USB stick. */
export function suggestedFileName(): string {
  const d = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  return `tariqi-${d.getFullYear()}${pad(d.getMonth() + 1)}${pad(d.getDate())}.db`;
}
