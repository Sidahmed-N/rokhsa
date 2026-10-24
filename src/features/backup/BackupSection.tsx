import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { open, save } from "@tauri-apps/plugin-dialog";

import Modal from "@/components/Modal";
import { Num } from "@/components/Num";
import { isAppError } from "@/lib/candidates";
import {
  backupNow,
  backupStatus,
  backupTo,
  cancelRestore,
  formatSize,
  formatWhen,
  inspectBackup,
  stageRestore,
  suggestedFileName,
  type BackupInfo,
  type BackupStatus,
} from "@/lib/backup";
import { isDesktop } from "@/lib/tauri";

const DB_FILTER = [{ name: "Tariqi", extensions: ["db"] }];

/**
 * Backup and restore.
 *
 * Restore shows what is inside the chosen file — when it was made and how many
 * candidates and payments it holds — before the owner commits. "Restore" on a
 * blind filename is how somebody overwrites a year of work with last March.
 */
export default function BackupSection() {
  const { t } = useTranslation();

  const [status, setStatus] = useState<BackupStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [confirming, setConfirming] = useState<BackupInfo | null>(null);

  const errorText = useCallback(
    (raw: unknown): string =>
      isAppError(raw) ? t(raw.code, { defaultValue: raw.message }) : String(raw),
    [t],
  );

  const refresh = useCallback(async () => {
    if (!isDesktop()) return;
    try {
      setStatus(await backupStatus());
    } catch (raw: unknown) {
      setError(errorText(raw));
    }
  }, [errorText]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  async function run(action: () => Promise<string | null>) {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const message = await action();
      if (message) setNotice(message);
      await refresh();
    } catch (raw: unknown) {
      setError(errorText(raw));
    } finally {
      setBusy(false);
    }
  }

  const onBackupHere = () =>
    void run(async () => {
      const info = await backupNow();
      return t("backup.madeHere", { name: info.fileName });
    });

  const onBackupToUsb = () =>
    void run(async () => {
      const path = await save({
        defaultPath: suggestedFileName(),
        filters: DB_FILTER,
      });
      if (!path) return null;
      const info = await backupTo(path);
      return t("backup.madeAt", { path: info.path });
    });

  const onChooseRestore = () =>
    void run(async () => {
      const picked = await open({ multiple: false, filters: DB_FILTER });
      if (typeof picked !== "string") return null;
      // Read it first — nothing is touched until the owner confirms.
      setConfirming(await inspectBackup(picked));
      return null;
    });

  const onConfirmRestore = () => {
    const target = confirming;
    if (!target) return;
    setConfirming(null);
    void run(async () => {
      await stageRestore(target.path);
      return t("backup.restoreStaged");
    });
  };

  const onCancelRestore = () =>
    void run(async () => {
      await cancelRestore();
      return t("backup.restoreCancelled");
    });

  return (
    <section className="mb-6 rounded-lg border border-line bg-surface shadow-card">
      <div className="px-4 py-4">
        {error && (
          <p className="mb-3 rounded-md border border-owed/30 bg-owed-soft px-3 py-2 text-sm text-owed">
            {error}
          </p>
        )}
        {notice && (
          <p className="mb-3 rounded-md border border-ok bg-surface px-3 py-2 text-sm text-ok">
            {notice}
          </p>
        )}

        {status?.pendingRestore && (
          <div className="mb-4 rounded-md border border-warn bg-surface px-3 py-3">
            <p className="text-sm font-medium text-warn">{t("backup.pendingTitle")}</p>
            <p className="mt-1 text-xs text-muted">{t("backup.pendingBody")}</p>
            <button
              type="button"
              onClick={onCancelRestore}
              disabled={busy}
              className="mt-2 inline-flex h-8 shrink-0 items-center justify-center gap-1.5 rounded-md border border-line-strong bg-surface px-2.5 text-xs font-medium text-ink shadow-card transition-colors hover:bg-sunken disabled:cursor-not-allowed disabled:text-faint"
            >
              {t("backup.cancelRestore")}
            </button>
          </div>
        )}

        <p className="mb-3 text-sm text-muted">{t("backup.explain")}</p>

        <div className="mb-4 flex flex-wrap gap-2">
          <button
            type="button"
            onClick={onBackupHere}
            disabled={busy}
            className="inline-flex h-9 shrink-0 items-center justify-center gap-2 rounded-md bg-brand px-3.5 text-sm font-medium text-white shadow-card transition-colors hover:bg-brand-hover disabled:cursor-not-allowed disabled:bg-faint"
          >
            {t("backup.now")}
          </button>
          <button
            type="button"
            onClick={onBackupToUsb}
            disabled={busy}
            className="inline-flex h-9 shrink-0 items-center justify-center gap-2 rounded-md border border-line-strong bg-surface px-3.5 text-sm font-medium text-ink shadow-card transition-colors hover:bg-sunken disabled:cursor-not-allowed disabled:text-faint"
          >
            {t("backup.toUsb")}
          </button>
          <button
            type="button"
            onClick={onChooseRestore}
            disabled={busy}
            className="inline-flex h-9 shrink-0 items-center justify-center gap-2 rounded-md border border-line-strong bg-surface px-3.5 text-sm font-medium text-ink shadow-card transition-colors hover:bg-sunken disabled:cursor-not-allowed disabled:text-faint"
          >
            {t("backup.restore")}
          </button>
        </div>

        {status && (
          <p className="mb-2 text-xs text-muted">
            {t("backup.folder")}: <span dir="ltr">{status.folder}</span>
          </p>
        )}

        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-line text-xs uppercase tracking-wide text-muted">
              <th className="py-2 text-start font-medium">{t("backup.when")}</th>
              <th className="py-2 text-end font-medium">{t("backup.candidates")}</th>
              <th className="py-2 text-end font-medium">{t("backup.payments")}</th>
              <th className="py-2 text-end font-medium">{t("backup.size")}</th>
            </tr>
          </thead>
          <tbody>
            {(status?.backups.length ?? 0) === 0 && (
              <tr>
                <td colSpan={4} className="py-4 text-center text-sm text-muted">
                  {t("backup.none")}
                </td>
              </tr>
            )}
            {status?.backups.map((b) => (
              <tr key={b.path} className="border-b border-line last:border-b-0">
                <td className="py-2">
                  <Num>{formatWhen(b.createdAt)}</Num>
                  <span className="ms-2 text-xs text-muted" dir="ltr">
                    {b.fileName}
                  </span>
                </td>
                <td className="py-2 text-end">
                  <Num>{b.candidates}</Num>
                </td>
                <td className="py-2 text-end">
                  <Num>{b.payments}</Num>
                </td>
                <td className="py-2 text-end">
                  <Num>{formatSize(b.sizeBytes)}</Num>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {confirming && (
        <Modal
          title={t("backup.confirmTitle")}
          onClose={() => setConfirming(null)}
          onSubmit={onConfirmRestore}
          submitLabel={t("backup.confirmButton")}
        >
          <p className="mb-3 text-sm">{t("backup.confirmBody")}</p>
          <ul className="mb-3 rounded-md border border-line bg-brand-soft px-3 py-2 text-sm">
            <li>
              {t("backup.when")}: <Num>{formatWhen(confirming.createdAt)}</Num>
            </li>
            <li>
              {t("backup.candidates")}: <Num>{confirming.candidates}</Num>
            </li>
            <li>
              {t("backup.payments")}: <Num>{confirming.payments}</Num>
            </li>
          </ul>
          <p className="text-xs text-muted">{t("backup.confirmNote")}</p>
        </Modal>
      )}
    </section>
  );
}
