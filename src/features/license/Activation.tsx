import { useState } from "react";
import i18next from "i18next";
import { useTranslation } from "react-i18next";
import { KeyRound, Lock } from "lucide-react";

import Logo from "@/components/Logo";
import Button from "@/components/ui/Button";
import { isAppError } from "@/lib/candidates";
import { unlockApp } from "@/lib/license";

/**
 * Asks for the owner's code the first time the app starts on a PC.
 *
 * The owner types it himself while installing, before the school ever sees
 * the app — which is why it comes before the welcome screen, and why it shows
 * both languages: nobody has chosen one yet.
 *
 * "Continue read-only" is always offered. A copy that lost its unlock (a new
 * Windows, a new PC) still holds the school's records, and those are never
 * hidden: it opens, shows everything, and can be backed up. Rust refuses the
 * writes.
 */
export function ActivationScreen({
  onUnlocked,
  onReadOnly,
}: {
  onUnlocked: () => void;
  onReadOnly: () => void;
}) {
  const fr = i18next.getFixedT("fr");
  const ar = i18next.getFixedT("ar");
  const [code, setCode] = useState("");
  const [busy, setBusy] = useState(false);
  const [wrong, setWrong] = useState<"wrong" | "other" | null>(null);

  async function submit() {
    if (!code.trim() || busy) return;
    setBusy(true);
    setWrong(null);
    try {
      await unlockApp(code);
      onUnlocked();
    } catch (raw: unknown) {
      setWrong(isAppError(raw) && raw.code === "license.wrongCode" ? "wrong" : "other");
      setCode("");
    } finally {
      setBusy(false);
    }
  }

  const both = (key: string) => (
    <>
      <span className="block" dir="ltr" lang="fr">
        {fr(key)}
      </span>
      <span className="block" dir="rtl" lang="ar">
        {ar(key)}
      </span>
    </>
  );

  return (
    <div className="fixed inset-0 z-[70] flex items-center justify-center bg-canvas p-6">
      <div className="w-full max-w-md rounded-2xl border border-line bg-surface px-8 py-9 text-center shadow-overlay">
        <Logo className="mx-auto size-16" />
        <h1 className="mt-5 text-lg font-semibold tracking-tight text-ink">
          {both("license.title")}
        </h1>
        <p className="mt-3 text-sm text-muted">{both("license.explain")}</p>

        <form
          className="mt-6"
          onSubmit={(e) => {
            e.preventDefault();
            void submit();
          }}
        >
          <div className="relative">
            <KeyRound
              className="pointer-events-none absolute start-3 top-1/2 size-4 -translate-y-1/2 text-faint"
              aria-hidden="true"
            />
            <input
              type="password"
              autoFocus
              autoComplete="off"
              spellCheck={false}
              dir="ltr"
              value={code}
              onChange={(e) => setCode(e.target.value)}
              aria-label={`${fr("license.codeLabel")} · ${ar("license.codeLabel")}`}
              className="h-11 w-full rounded-lg border border-line-strong bg-surface ps-9 pe-3 text-center text-base tracking-wider outline-none focus:border-brand focus:ring-2 focus:ring-brand-soft"
            />
          </div>
          {wrong && (
            <p role="alert" className="mt-2 text-sm text-owed">
              {both(wrong === "wrong" ? "license.wrongCode" : "license.failed")}
            </p>
          )}
          <Button
            type="submit"
            variant="primary"
            className="mt-4 w-full"
            disabled={busy || !code.trim()}
          >
            {fr("license.unlock")} · <span lang="ar">{ar("license.unlock")}</span>
          </Button>
        </form>

        <button
          type="button"
          onClick={onReadOnly}
          className="mt-5 text-xs text-muted underline-offset-2 hover:text-ink hover:underline"
        >
          {fr("license.readOnly")} · <span lang="ar">{ar("license.readOnly")}</span>
        </button>
      </div>
    </div>
  );
}

/** Shown above every page while the app is open read-only. */
export function ReadOnlyBanner({ onUnlock }: { onUnlock: () => void }) {
  const { t } = useTranslation();
  return (
    <div
      role="status"
      className="mb-5 flex flex-wrap items-center gap-3 rounded-lg border border-warn/40 bg-warn-soft px-4 py-3"
    >
      <Lock className="size-4 shrink-0 text-warn" aria-hidden="true" />
      <p className="text-sm text-ink">{t("license.readOnlyBanner")}</p>
      <Button size="sm" className="ms-auto" onClick={onUnlock}>
        {t("license.unlock")}
      </Button>
    </div>
  );
}
