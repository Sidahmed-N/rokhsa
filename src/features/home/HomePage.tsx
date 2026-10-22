import { useCallback, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { motion } from "motion/react";
import {
  ArrowDownLeft,
  ArrowUpRight,
  Check,
  ChevronRight,
  Phone,
  Users,
} from "lucide-react";

import FreeText from "@/components/FreeText";
import { Money, Num } from "@/components/Num";
import Button from "@/components/ui/Button";
import { Card, ErrorNote, Hero, PageHeader } from "@/components/ui/Surface";
import { currentLanguage } from "@/i18n";
import RuntimeBadge from "@/features/home/RuntimeBadge";
import { isAppError, listCandidates } from "@/lib/candidates";
import { kindStyle } from "@/lib/kinds";
import { personName } from "@/lib/names";
import {
  getCashFlow,
  getReceivables,
  thisMonth,
  type CashFlow,
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
  // Lifts under the pointer. A transform only — no layout and no blur — so
  // it stays cheap on the machines this runs on.
  return (
    <motion.div whileHover={{ y: -2 }} transition={{ type: "spring", stiffness: 400, damping: 28 }}>
      <Link
        to={to}
        className="group surface-blur block rounded-lg border border-line bg-surface p-4 shadow-card transition-all hover:border-brand-line hover:shadow-raised"
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
    </motion.div>
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
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    if (!isDesktop()) return;
    const range = thisMonth();
    try {
      const [r, f, page, day] = await Promise.all([
        getReceivables(),
        getCashFlow(range.from, range.to),
        listCandidates("", 1, 0),
        listDay(todayIso()),
      ]);
      setReceivables(r);
      setFlow(f);
      setCandidateCount(page.total);
      setToday(day);
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

      {/* The money owed is what an owner walks in worrying about, so it stops
          being a peer of the other three and becomes the screen's hero. */}
      <Hero
        label={t("receivables.totalOwed")}
        footer={t("home.owingCount", { n: receivables?.enrollmentsOwing ?? 0 })}
        action={
          <Link
            to="/receivables"
            className="inline-flex items-center gap-1 rounded-lg bg-white/15 px-3 py-2 text-sm font-medium text-white transition-colors hover:bg-white/25"
          >
            {t("nav.receivables")}
            <ChevronRight className="size-4 rtl:rotate-180" aria-hidden="true" />
          </Link>
        }
      >
        <Money centimes={receivables?.totalOwedCentimes ?? 0} whole />
      </Hero>

      <section className="mt-4 grid gap-3 sm:grid-cols-3">
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
