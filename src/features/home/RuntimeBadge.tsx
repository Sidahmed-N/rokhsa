import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { Num } from "@/components/Num";
import { appVersion, isDesktop } from "@/lib/tauri";

/**
 * Shows whether we are in the desktop shell and, if so, the version Rust
 * reports.
 *
 * This is the end-to-end smoke test for the Rust↔JS bridge: a version string
 * appearing here means the whole chain works. It also makes it obvious at a
 * glance whether you are looking at the real app or a browser tab.
 */
export default function RuntimeBadge() {
  const { t } = useTranslation();
  const desktop = isDesktop();
  const [version, setVersion] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!desktop) return;
    let cancelled = false;

    appVersion()
      .then((v) => {
        if (!cancelled) setVersion(v);
      })
      .catch((e: unknown) => {
        if (!cancelled) setError(e instanceof Error ? e.message : String(e));
      });

    return () => {
      cancelled = true;
    };
  }, [desktop]);

  return (
    <p className="mt-6 text-xs text-muted">
      {desktop ? t("home.runtimeDesktop") : t("home.runtimeBrowser")}
      {version !== null && (
        <>
          {" · "}
          {t("home.version")} <Num>{version}</Num>
        </>
      )}
      {error !== null && <span className="text-owed"> · {error}</span>}
    </p>
  );
}
