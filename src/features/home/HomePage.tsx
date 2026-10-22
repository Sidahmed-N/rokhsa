import { useCallback, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { useTranslation } from "react-i18next";
import {
  ArrowDownLeft,
  ArrowUpRight,
  Check,
  ChevronRight,
  Phone,
  TrendingDown,
  TrendingUp,
  Users,
  Wallet,
} from "lucide-react";

import FreeText from "@/components/FreeText";
import { Money, Num } from "@/components/Num";
import Button from "@/components/ui/Button";
import { Card, ErrorNote, PageHeader } from "@/components/ui/Surface";
import { currentLanguage } from "@/i18n";
import RuntimeBadge from "@/features/home/RuntimeBadge";
import { isAppError, listCandidates } from "@/lib/candidates";
import { kindStyle } from "@/lib/kinds";
import { personName } from "@/lib/names";
import MoneyChart from "@/features/home/MoneyChart";
import {
  getCashFlow,
  getCashFlowDaily,
  getReceivables,
  previousPeriod,
  thisMonth,
  type CashFlow,
  type DayTotal,
  type ReceivableRow,
  type Receivables,
} from "@/lib/reports";
import { isDesktop } from "@/lib/tauri";
import {
  completeLesson,
  listDay,
  setLessonNotHeld,
  todayIso,
  type DayEntry,
} from "@/lib/training";

/** How many rows each panel shows before deferring to its own screen. */
const PANEL_ROWS = 5;

/**
 * One headline number, as a link to the screen that explains it.
 *
 * The figure is the biggest thing in the tile and the label the smallest:
 * the owner is scanning for an amount, not reading a sentence.
 */
/**
 * The change against the same span one period back.
 *
 * Direction is not the same as good: more money in is good, more money out
 * is not, so the caller says which way is up. Omitted entirely when the
 * previous period was zero — "+100%" against nothing means nothing.
 */
function Delta({ now, before, moreIsGood }: { now: number; before: number; moreIsGood: boolean }) {
  const { t } = useTranslation();
  if (before <= 0) return null;

  const percent = Math.round(((now - before) / before) * 100);
  if (percent === 0) return null;

  const up = percent > 0;
  const good = up === moreIsGood;
  const Icon = up ? TrendingUp : TrendingDown;

  return (
    <span className="mt-1.5 flex flex-wrap items-center gap-x-1.5 gap-y-0.5 text-xs">
      <span
        className={[
          "inline-flex items-center gap-0.5 rounded-full px-1.5 py-0.5 font-medium",
          good ? "bg-ok-soft text-ok" : "bg-owed-soft text-owed",
        ].join(" ")}
      >
        <Icon className="size-3" aria-hidden="true" />
        {/* The sign and the % both sit INSIDE the isolate; left outside, the
            bidi algorithm moves them to the far side of the digits. */}
        <Num>{`${up ? "+" : "−"}${Math.abs(percent)}%`}</Num>
      </span>
      <span className="text-muted">{t("home.vsLastPeriod")}</span>
    </span>
  );
}

function Stat({
  label,
  icon,
  tone,
  to,
  delta,
  children,
}: {
  label: string;
  icon: React.ReactNode;
  tone?: "owed" | "ok" | undefined;
  to: string;
  delta?: React.ReactNode | undefined;
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
      {delta}
    </Link>
  );
}

/** A titled panel with a count and a link through to the full screen. */
function Panel({
  title,
  count,
  to,
  linkLabel,
  children,
}: {
  title: string;
  count: number;
  to: string;
  linkLabel: string;
  children: React.ReactNode;
}) {
  return (
    <Card flush className="flex flex-col">
      <header className="flex items-center gap-2 border-b border-line px-4 py-2.5">
        <h2 className="text-sm font-semibold text-ink">{title}</h2>
        <span className="rounded-full bg-sunken px-2 py-0.5 text-xs font-medium text-muted">
          <Num>{count}</Num>
        </span>
        <Link
          to={to}
          className="ms-auto inline-flex items-center gap-0.5 text-xs font-medium text-brand hover:underline"
        >
          {linkLabel}
          <ChevronRight className="size-3.5 rtl:rotate-180" aria-hidden="true" />
        </Link>
      </header>
      <div className="flex-1">{children}</div>
    </Card>
  );
}

/**
 * Dashboard.
 *
 * The first screen of the day, so it answers the two questions an owner
 * actually walks in with: who is coming today, and who owes me money. It
 * used to show four totals and nothing else, which told him how the school
 * was doing but gave him nothing to do about it.
 *
 * Marking a session done is possible from here as well as from the
 * calendar, on purpose. The hours are only trustworthy if they get entered,
 * and they only get entered if recording one is never more than a click
 * from wherever the owner already is.
 */
export default function HomePage() {
  const { t } = useTranslation();
  const locale = currentLanguage();

  const [receivables, setReceivables] = useState<Receivables | null>(null);
  const [flow, setFlow] = useState<CashFlow | null>(null);
  const [candidateCount, setCandidateCount] = useState<number | null>(null);
  const [today, setToday] = useState<DayEntry[]>([]);
  const [before, setBefore] = useState<CashFlow | null>(null);
  const [series, setSeries] = useState<DayTotal[]>([]);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    if (!isDesktop()) return;
    const range = thisMonth();
    try {
      const last = previousPeriod(range);
      const [r, f, page, day, previous, daily] = await Promise.all([
        getReceivables(),
        getCashFlow(range.from, range.to),
        listCandidates("", 1, 0),
        listDay(todayIso()),
        getCashFlow(last.from, last.to),
        getCashFlowDaily(range.from, range.to),
      ]);
      setReceivables(r);
      setFlow(f);
      setCandidateCount(page.total);
      setToday(day);
      setBefore(previous);
      setSeries(daily);
      setError(null);
    } catch (raw: unknown) {
      setError(isAppError(raw) ? t(raw.code, { defaultValue: raw.message }) : String(raw));
    }
  }, [t]);

  useEffect(() => {
    void load();
  }, [load]);

  async function run(action: () => Promise<unknown>) {
    try {
      await action();
      await load();
    } catch (raw: unknown) {
      setError(isAppError(raw) ? t(raw.code, { defaultValue: raw.message }) : String(raw));
    }
  }

  const plannedToday = today.filter((r) => r.status === "planned");

  // Worst first: the biggest balance that has been quiet longest is the call
  // worth making before the others.
  const chase: ReceivableRow[] = [...(receivables?.rows ?? [])]
    .sort(
      (a, b) =>
        (b.daysSincePayment ?? Number.MAX_SAFE_INTEGER) -
          (a.daysSincePayment ?? Number.MAX_SAFE_INTEGER) ||
        b.balanceCentimes - a.balanceCentimes,
    )
    .slice(0, PANEL_ROWS);

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
          delta={
            <Delta
              now={flow?.inCentimes ?? 0}
              before={before?.inCentimes ?? 0}
              moreIsGood
            />
          }
        >
          <Money centimes={flow?.inCentimes ?? 0} whole />
        </Stat>
        <Stat
          label={t("home.outThisMonth")}
          to="/cash-flow"
          icon={<ArrowUpRight className="size-4" aria-hidden="true" />}
          delta={
            <Delta
              now={flow?.outCentimes ?? 0}
              before={before?.outCentimes ?? 0}
              moreIsGood={false}
            />
          }
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

      {series.length > 0 && (
        <Card className="mt-4">
          <h2 className="mb-3 text-sm font-semibold text-ink">{t("home.chartTitle")}</h2>
          <MoneyChart days={series} />
        </Card>
      )}

      <div className="mt-4 grid gap-4 lg:grid-cols-2">
        {/* Who is coming in */}
        <Panel
          title={t("home.todayTitle")}
          count={plannedToday.length}
          to="/calendar"
          linkLabel={t("nav.calendar")}
        >
          {today.length === 0 ? (
            <p className="px-4 py-8 text-center text-sm text-muted">{t("calendar.empty")}</p>
          ) : (
            <ul className="divide-y divide-line">
              {today.slice(0, PANEL_ROWS).map((row) => (
                <li key={row.id} className="flex flex-wrap items-center gap-x-3 gap-y-2 px-4 py-2.5">
                  <span
                    className={[
                      "rounded px-1.5 py-0.5 text-xs font-semibold",
                      kindStyle(row.kind).chip,
                    ].join(" ")}
                  >
                    <Num>{row.scheduledTime ?? "—"}</Num>
                  </span>
                  <Link
                    to={`/candidates/${row.candidateId}`}
                    className="text-sm font-medium text-ink hover:text-brand hover:underline"
                  >
                    {personName(row, locale)}
                  </Link>
                  {row.planNote && (
                    <FreeText as="span" className="min-w-0 flex-1 truncate text-xs text-muted">
                      {row.planNote}
                    </FreeText>
                  )}

                  {row.status === "planned" ? (
                    <span className="ms-auto flex gap-1.5">
                      <Button
                        variant="primary"
                        size="sm"
                        icon={<Check className="size-3.5" aria-hidden="true" />}
                        onClick={() =>
                          void run(() =>
                            completeLesson(row.id, { occurredOn: "", minutes: 0, notes: "" }),
                          )
                        }
                      >
                        {t("calendar.markDone")}
                      </Button>
                      <Button
                        size="sm"
                        onClick={() => void run(() => setLessonNotHeld(row.id, "missed"))}
                      >
                        {t("calendar.markMissed")}
                      </Button>
                    </span>
                  ) : (
                    <span className="ms-auto text-xs text-muted">
                      {t(`calendar.status_${row.status}`)}
                    </span>
                  )}
                </li>
              ))}
            </ul>
          )}
        </Panel>

        {/* Who to call */}
        <Panel
          title={t("home.chaseTitle")}
          count={receivables?.enrollmentsOwing ?? 0}
          to="/receivables"
          linkLabel={t("nav.receivables")}
        >
          {chase.length === 0 ? (
            <p className="px-4 py-8 text-center text-sm text-muted">
              {t("receivables.nobodyOwes")}
            </p>
          ) : (
            <ul className="divide-y divide-line">
              {chase.map((row) => (
                <li
                  key={row.enrollmentId}
                  className="flex flex-wrap items-center gap-x-3 gap-y-1 px-4 py-2.5"
                >
                  <Link
                    to={`/candidates/${row.candidateId}`}
                    className="text-sm font-medium text-ink hover:text-brand hover:underline"
                  >
                    {personName(row, locale)}
                  </Link>
                  {row.phone && (
                    <span className="inline-flex items-center gap-1 text-xs text-muted">
                      <Phone className="size-3 text-faint" aria-hidden="true" />
                      <Num>{row.phone}</Num>
                    </span>
                  )}
                  <span className="ms-auto flex items-center gap-3">
                    <span className="text-xs text-muted">
                      {row.daysSincePayment === null ? (
                        t("receivables.never")
                      ) : (
                        <>
                          <Num>{row.daysSincePayment}</Num> {t("receivables.daysAgo")}
                        </>
                      )}
                    </span>
                    <Money
                      centimes={row.balanceCentimes}
                      whole
                      className="text-sm font-semibold text-owed"
                    />
                  </span>
                </li>
              ))}
            </ul>
          )}
        </Panel>
      </div>

      <RuntimeBadge />
    </div>
  );
}
