import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { Num } from "@/components/Num";
import { call, isDesktop } from "@/lib/tauri";

interface ClockStatus {
  ok: boolean;
  newestRecord: string | null;
  now: string;
  daysBehind: number | null;
}

/**
 * Warns when the computer's clock is wrong.
 *
 * A PC with a dead CMOS battery boots years in the past. Every record then
 * gets that date — payments land in a month nobody looks at, and the receipt
 * the candidate is holding is dated 2019. Nothing inside the data reveals it,
 * because every row is wrong in the same direction.
 *
 * Shown on every screen, and not dismissible: carrying on would keep writing
 * bad dates, and each one is a receipt already handed over.
 */
export default function ClockWarning() {
  const { t } = useTranslation();
  const [status, setStatus] = useState<ClockStatus | null>(null);

  useEffect(() => {
    if (!isDesktop()) return;
    call<ClockStatus>("clock_status")
      .then(setStatus)
      .catch(() => setStatus(null));
  }, []);

  if (!status || status.ok) return null;

  return (
    <div className="screen-only border-b border-warn bg-warn/10 px-4 py-2">
      <div className="mx-auto max-w-6xl text-sm">
        <strong className="text-warn">{t("clock.title")}</strong>{" "}
        <span className="text-ink">
          {t("clock.body", { days: Math.abs(status.daysBehind ?? 0) })}
        </span>
        {status.daysBehind !== null && (
          <span className="ms-2 text-xs text-muted">
            (<Num>{Math.abs(status.daysBehind)}</Num> {t("clock.days")})
          </span>
        )}
      </div>
    </div>
  );
}
