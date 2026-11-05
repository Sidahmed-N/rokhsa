import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { dismissRestoreNotice, restoreNotice, type RestoreNotice as Notice } from "@/lib/backup";
import { isDesktop } from "@/lib/tauri";

/**
 * Says what happened to a restore, once, after the restart.
 *
 * The swap happens at startup before any window exists, so the owner who
 * clicked "Restaurer" and restarted used to see nothing: the "en attente"
 * card simply vanished, whether the file had been put in place, refused as
 * damaged, or put off because Windows still held the old one. Believing old
 * data is back when it is not is how a day's payments get typed twice.
 *
 * Shown on every screen (the app opens on Accueil, not on Sauvegarde) until
 * the owner presses OK.
 */
export default function RestoreNotice() {
  const { t } = useTranslation();
  const [notice, setNotice] = useState<Notice | null>(null);

  useEffect(() => {
    if (!isDesktop()) return;
    restoreNotice()
      .then(setNotice)
      .catch(() => setNotice(null));
  }, []);

  if (!notice) return null;
  const applied = notice.outcome === "applied";

  return (
    <div
      className={[
        "screen-only border-b px-4 py-2",
        applied ? "border-ok bg-ok/10" : "border-warn bg-warn/10",
      ].join(" ")}
    >
      <div className="mx-auto flex max-w-6xl flex-wrap items-center gap-x-3 gap-y-1 text-sm">
        <span className="text-ink">
          {applied
            ? t("backup.notice.applied", { candidates: notice.candidates, payments: notice.payments })
            : t(`backup.notice.${notice.outcome}`)}
        </span>
        <button
          type="button"
          onClick={() => {
            setNotice(null);
            void dismissRestoreNotice().catch(() => undefined);
          }}
          className="ms-auto rounded-md border border-line bg-surface px-2.5 py-1 text-xs font-medium text-ink hover:bg-sunken"
        >
          {t("common.ok")}
        </button>
      </div>
    </div>
  );
}
