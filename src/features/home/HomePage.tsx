import { useCallback, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { useTranslation } from "react-i18next";
import {
  ArrowDownLeft,
  ArrowUpRight,
  CalendarDays,
  ChevronRight,
  Users,
  Wallet,
} from "lucide-react";

import { Money, Num } from "@/components/Num";
import { Card, ErrorNote, PageHeader } from "@/components/ui/Surface";
import RuntimeBadge from "@/features/home/RuntimeBadge";
import { isAppError, listCandidates } from "@/lib/candidates";
import {
  getCashFlow,
  getReceivables,
  thisMonth,
  type CashFlow,
  type Receivables,
} from "@/lib/reports";
import { isDesktop } from "@/lib/tauri";

/**
 * One headline number, as a link to the screen that explains it.
 *
 * The figure is the biggest thing in the tile and the label the smallest:
 * the owner is scanning for an amount, not reading a sentence.
 */
function Stat({
  label,
  icon,
  tone,
  to,
  children,
}: {
  label: string;
  icon: React.ReactNode;
  tone?: "owed" | "ok" | undefined;
  to: string;
  children: React.ReactNode;
}) {
  return (
    <Link
      to={to}
      className="group rounded-lg border border-line bg-surface p-4 shadow-card transition-colors hover:border-brand-line hover:bg-brand-soft/40"
    >
      <div className="flex items-center gap-2">
        <span className="text-faint">{icon}</span>
        <span className="text-xs font-medium tracking-wide text-muted uppercase">{label}</span>
        <ChevronRight
          className="ms-auto size-4 text-faint opacity-0 transition-opacity group-hover:opacity-100 rtl:rotate-180"
          aria-hidden="true"
        />
      </div>
      <p
        className={[
          "mt-2 text-2xl font-semibold tracking-tight",
          tone === "owed" ? "text-owed" : tone === "ok" ? "text-ok" : "text-ink",
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
    <div>
      <PageHeader title={t("home.title")} description={t("home.subtitle")} />

      {error && <ErrorNote>{error}</ErrorNote>}

      <section className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
        <Stat
          label={t("receivables.totalOwed")}
          to="/receivables"
          tone="owed"
          icon={<Wallet className="size-4" aria-hidden="true" />}
        >
          <Money centimes={receivables?.totalOwedCentimes ?? 0} whole />
        </Stat>
        <Stat
          label={t("home.inThisMonth")}
          to="/cash-flow"
          tone="ok"
          icon={<ArrowDownLeft className="size-4" aria-hidden="true" />}
        >
          <Money centimes={flow?.inCentimes ?? 0} whole />
        </Stat>
        <Stat
          label={t("home.outThisMonth")}
          to="/cash-flow"
          icon={<ArrowUpRight className="size-4" aria-hidden="true" />}
        >
          <Money centimes={flow?.outCentimes ?? 0} whole />
        </Stat>
        <Stat
          label={t("home.candidates")}
          to="/candidates"
          icon={<Users className="size-4" aria-hidden="true" />}
        >
          <Num>{candidateCount ?? 0}</Num>
        </Stat>
      </section>

      <Card className="mt-3">
        <div className="flex flex-wrap items-center gap-3">
          <CalendarDays className="size-4 shrink-0 text-faint" aria-hidden="true" />
          <p className="text-sm text-ink-soft">
            {t("home.owingCount", { n: receivables?.enrollmentsOwing ?? 0 })}
          </p>
          <Link
            to="/calendar"
            className="ms-auto text-sm font-medium text-brand hover:underline"
          >
            {t("calendar.title")}
          </Link>
        </div>
      </Card>

      <RuntimeBadge />
    </div>
  );
}
