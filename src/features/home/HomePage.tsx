import { useCallback, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { useTranslation } from "react-i18next";

import { Money, Num } from "@/components/Num";
import RuntimeBadge from "@/features/home/RuntimeBadge";
import { isAppError, listCandidates } from "@/lib/candidates";
import { getCashFlow, getReceivables, thisMonth, type CashFlow, type Receivables } from "@/lib/reports";
import { isDesktop } from "@/lib/tauri";

function Tile({
  label,
  children,
  to,
  tone,
}: {
  label: string;
  children: React.ReactNode;
  to: string;
  tone?: "owed" | "ok" | undefined;
}) {
  return (
    <Link
      to={to}
      className="block rounded-lg border border-line bg-white px-4 py-3 transition-colors hover:border-brand"
    >
      <p className="text-xs uppercase tracking-wide text-muted">{label}</p>
      <p
        className={[
          "mt-1 text-2xl font-bold",
          tone === "owed" ? "text-owed" : tone === "ok" ? "text-ok" : "",
        ].join(" ")}
      >
        {children}
      </p>
    </Link>
  );
}

/**
 * Dashboard.
 *
 * The first screen in any demo, so it answers the questions an owner actually
 * walks in with: how much am I owed, what came in this month, what went out.
 */
export default function HomePage() {
  const { t } = useTranslation();

  const [receivables, setReceivables] = useState<Receivables | null>(null);
  const [flow, setFlow] = useState<CashFlow | null>(null);
  const [candidateCount, setCandidateCount] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    if (!isDesktop()) return;
    const range = thisMonth();
    try {
      const [r, f, page] = await Promise.all([
        getReceivables(),
        getCashFlow(range.from, range.to),
        listCandidates("", 1, 0),
      ]);
      setReceivables(r);
      setFlow(f);
      setCandidateCount(page.total);
      setError(null);
    } catch (raw: unknown) {
      setError(isAppError(raw) ? t(raw.code, { defaultValue: raw.message }) : String(raw));
    }
  }, [t]);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <div className="max-w-4xl">
      <h1 className="text-xl font-semibold">{t("home.title")}</h1>
      <p className="mt-1 text-sm text-muted">{t("home.subtitle")}</p>

      {error && (
        <p className="mt-3 rounded-md border border-owed bg-white px-3 py-2 text-sm text-owed">
          {error}
        </p>
      )}

      <section className="mt-5 grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
        <Tile label={t("receivables.totalOwed")} to="/receivables" tone="owed">
          <Money centimes={receivables?.totalOwedCentimes ?? 0} whole />
        </Tile>
        <Tile label={t("home.inThisMonth")} to="/cash-flow" tone="ok">
          <Money centimes={flow?.inCentimes ?? 0} whole />
        </Tile>
        <Tile label={t("home.outThisMonth")} to="/cash-flow" tone="owed">
          <Money centimes={flow?.outCentimes ?? 0} whole />
        </Tile>
        <Tile label={t("home.candidates")} to="/candidates">
          <Num>{candidateCount ?? 0}</Num>
        </Tile>
      </section>

      <p className="mt-4 text-sm text-muted">
        {t("home.owingCount", { n: receivables?.enrollmentsOwing ?? 0 })}
      </p>

      <RuntimeBadge />
    </div>
  );
}
