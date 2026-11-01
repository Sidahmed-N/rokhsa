import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Download } from "lucide-react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

import { Num } from "@/components/Num";
import Button from "@/components/ui/Button";
import { backupNow } from "@/lib/backup";
import { isDesktop } from "@/lib/tauri";

/** Look again this often while the app stays open: it is on all day. */
const RECHECK_MS = 6 * 60 * 60 * 1000;

type Phase =
  | { kind: "idle" }
  | { kind: "backup" }
  | { kind: "download"; percent: number | null }
  | { kind: "failed" };

/**
 * Offers a new version when one has been published.
 *
 * Looked for quietly, only when the PC happens to be online: with no internet
 * the check fails and nothing is shown, ever — the app works offline forever.
 * Never installed on its own. This app is the school's till, open from eight
 * in the morning, and an install restarts it; the owner picks the moment.
 *
 * A backup is made before anything is downloaded. An update never touches the
 * database folder, but the new version may migrate the database on its first
 * start, and a copy from just before is the way back if that ever goes wrong.
 * If the backup fails the update does not start.
 *
 * Every update is verified against the public key in tauri.conf.json; one not
 * signed with the school's publisher key is refused by the plugin.
 */
export default function UpdateBanner() {
  const { t } = useTranslation();
  const [update, setUpdate] = useState<Update | null>(null);
  const [dismissed, setDismissed] = useState(false);
  const [phase, setPhase] = useState<Phase>({ kind: "idle" });

  useEffect(() => {
    if (!isDesktop()) return;
    let alive = true;
    const look = () => {
      check()
        .then((found) => {
          if (alive && found) setUpdate(found);
        })
        // Offline, GitHub unreachable, nothing published yet: all mean "no
        // update to offer", and none of them is the school's problem.
        .catch(() => undefined);
    };
    look();
    const timer = window.setInterval(look, RECHECK_MS);
    return () => {
      alive = false;
      window.clearInterval(timer);
    };
  }, []);

  async function install(target: Update) {
    try {
      setPhase({ kind: "backup" });
      await backupNow();

      let total = 0;
      let received = 0;
      setPhase({ kind: "download", percent: null });
      await target.downloadAndInstall((event) => {
        if (event.event === "Started") total = event.data.contentLength ?? 0;
        if (event.event === "Progress") {
          received += event.data.chunkLength;
          setPhase({
            kind: "download",
            percent: total > 0 ? Math.min(100, Math.round((received / total) * 100)) : null,
          });
        }
      });
      // On Windows the installer closes the app itself; elsewhere, restart it.
      await relaunch();
    } catch {
      setPhase({ kind: "failed" });
    }
  }

  if (!update || dismissed) return null;

  const busy = phase.kind === "backup" || phase.kind === "download";

  return (
    <div
      role="status"
      className="mb-5 flex flex-wrap items-center gap-3 rounded-lg border border-brand/30 bg-brand-soft px-4 py-3"
    >
      <Download className="size-4 shrink-0 text-brand" aria-hidden="true" />
      <p className="text-sm text-ink">
        {t("updates.available")} <Num>{update.version}</Num>
        {phase.kind === "idle" && (
          <span className="ms-2 text-muted">{t("updates.hint")}</span>
        )}
        {phase.kind === "backup" && (
          <span className="ms-2 text-muted">{t("updates.backingUp")}</span>
        )}
        {phase.kind === "download" && (
          <span className="ms-2 text-muted">
            {t("updates.downloading")}
            {phase.percent !== null && (
              <>
                {" "}
                <Num>{`${phase.percent}%`}</Num>
              </>
            )}
          </span>
        )}
        {phase.kind === "failed" && (
          <span className="ms-2 text-owed">{t("updates.failed")}</span>
        )}
      </p>
      <div className="ms-auto flex gap-2">
        <Button variant="primary" size="sm" disabled={busy} onClick={() => void install(update)}>
          {phase.kind === "failed" ? t("updates.retry") : t("updates.install")}
        </Button>
        {!busy && (
          <Button variant="ghost" size="sm" onClick={() => setDismissed(true)}>
            {t("updates.later")}
          </Button>
        )}
      </div>
    </div>
  );
}
