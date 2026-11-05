import { useCallback, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { motion } from "motion/react";
import {
  AlertTriangle,
  ArrowDownLeft,
  ArrowUpRight,
  Check,
  ChevronRight,
  Users,
} from "lucide-react";

import FreeText from "@/components/FreeText";
import { Money, Num } from "@/components/Num";
import Button from "@/components/ui/Button";
import { Badge, Card, ErrorNote, Hero, PageHeader } from "@/components/ui/Surface";
import RuntimeBadge from "@/features/home/RuntimeBadge";
import { isAppError, listCandidates } from "@/lib/candidates";
import { useToday } from "@/lib/today";
import { kindStyle } from "@/lib/kinds";
import { personName } from "@/lib/names";
import { nextExamSession, type NextExam } from "@/lib/exams";
import {
  getCashFlow,
  getReceivables,
  thisMonth,
  type CashFlow,
  type Receivables,
} from "@/lib/reports";
import { isDesktop } from "@/lib/tauri";
import {
  completeLesson,
  listDay,
  listOverdue,
  localTime,
  setLessonNotHeld,
  todayIso,
  type DayEntry,
} from "@/lib/training";

/** How many rows each panel shows before deferring to its own screen.
 * Not the unmarked-sessions panel: no other screen lists those across days,
 * so it shows every one of them. */
const PANEL_ROWS = 5;

/** "3 more" under a panel that stopped at PANEL_ROWS, linking to its screen. */
function MoreRows({ shown, total, to }: { shown: number; total: number; to: string }) {
  const { t } = useTranslation();
  if (total <= shown) return null;
  return (
    <Link
      to={to}
      className="block border-t border-line px-4 py-2.5 text-sm font-medium text-brand hover:bg-sunken"
    >
      {t("home.moreRows", { count: total - shown })}
    </Link>
  );
}

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
  tourId,
  children,
}: {
  tourId?: string | undefined;
  title: string;
  count: number;
  to: string;
  linkLabel: string;
  children: React.ReactNode;
}) {
  return (
    <Card flush tourId={tourId} className="flex flex-col">
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

  const [receivables, setReceivables] = useState<Receivables | null>(null);
  const [flow, setFlow] = useState<CashFlow | null>(null);
  const [candidateCount, setCandidateCount] = useState<number | null>(null);
  const [today, setToday] = useState<DayEntry[]>([]);
  const [overdue, setOverdue] = useState<DayEntry[]>([]);
  const [next, setNext] = useState<NextExam | null>(null);
  /**
   * Today, kept true while the app stays open. Both lists below are asked
   * FOR a date, so pinning it at mount meant that after midnight the panel
   * showed yesterday’s sessions and the overdue card asked what was unmarked
   * before yesterday — missing the day it exists to catch.
   */
  const today$ = useToday();
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    if (!isDesktop()) return;
    const range = thisMonth();
    try {
      const [r, f, page, day, late] = await Promise.all([
        getReceivables(),
        getCashFlow(range.from, range.to),
        listCandidates("", false, 1, 0),
        listDay(today$),
        listOverdue(today$),
      ]);
      setReceivables(r);
      setFlow(f);
      setCandidateCount(page.total);
      setToday(day);
      setOverdue(late);
      setError(null);
    } catch (raw: unknown) {
      setError(isAppError(raw) ? t(raw.code, { defaultValue: raw.message }) : String(raw));
    }
  }, [t, today$]);

  useEffect(() => {
    void load();
  }, [load]);

  /**
   * The next exam, asked with the time as well as the date: a sitting today
   * gives way to the next one once its time has passed (Rust decides, see
   * exams::next_session). The app stays open all day, so it is asked again
   * every minute and whenever the window comes back, not only at load.
   */
  const loadNext = useCallback(async () => {
    if (!isDesktop()) return;
    try {
      setNext(await nextExamSession(todayIso(), localTime(new Date())));
    } catch (raw: unknown) {
      setError(isAppError(raw) ? t(raw.code, { defaultValue: raw.message }) : String(raw));
    }
  }, [t]);

  useEffect(() => {
    void loadNext();
    const timer = window.setInterval(() => void loadNext(), 60_000);
    const onFocus = () => void loadNext();
    window.addEventListener("focus", onFocus);
    return () => {
      window.clearInterval(timer);
      window.removeEventListener("focus", onFocus);
    };
    // today$ too: a new day must not wait for the next minute.
  }, [loadNext, today$]);

  async function run(action: () => Promise<unknown>) {
    try {
      await action();
      await load();
    } catch (raw: unknown) {
      setError(isAppError(raw) ? t(raw.code, { defaultValue: raw.message }) : String(raw));
    }
  }

  return (
    <div>
      <PageHeader title={t("home.title")} description={t("home.subtitle")} />

      {error && <ErrorNote>{error}</ErrorNote>}

      {/* The money owed is what an owner walks in worrying about, so it stops
          being a peer of the other three and becomes the screen's hero. */}
      <Hero
        tourId="home-owed"
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
          tourId="home-today"
          title={t("home.todayTitle")}
          count={today.length}
          to="/calendar"
          linkLabel={t("nav.calendar")}
        >
          {today.length === 0 ? (
            <p className="px-4 py-8 text-center text-sm text-muted">{t("calendar.empty")}</p>
          ) : (
            <>
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
                    {personName(row)}
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
            <MoreRows shown={Math.min(today.length, PANEL_ROWS)} total={today.length} to="/calendar" />
            </>
          )}
        </Panel>

        {/* The next exam sitting. It replaced "À relancer", which repeated
            Créances cut to five names and showed the same people every
            morning until they paid. This one changes week to week, has a
            date, and says whose fee is still to collect. */}
        <Panel
          tourId="home-exam"
          title={t("home.examTitle")}
          count={next?.candidates ?? 0}
          to={next ? `/exams?session=${next.session.id}` : "/exams"}
          linkLabel={t("nav.exams")}
        >
          {next === null ? (
            <p className="px-4 py-8 text-center text-sm text-muted">{t("home.examNone")}</p>
          ) : (
            <Link
              to={`/exams?session=${next.session.id}`}
              className="block px-4 py-4 transition-colors hover:bg-sunken"
            >
              <span className="flex flex-wrap items-center gap-2">
                <span
                  className={[
                    "rounded px-1.5 py-0.5 text-xs font-semibold",
                    kindStyle(next.session.kind).chip,
                  ].join(" ")}
                >
                  {t(`lessons.kind_${next.session.kind}`)}
                </span>
                <Num className="text-sm font-semibold text-ink">{next.session.heldOn}</Num>
                {next.session.heldAt && (
                  <Num className="text-xs text-muted">{next.session.heldAt}</Num>
                )}
                {next.session.heldOn === today$ && (
                  <Badge tone="brand">{t("home.examToday")}</Badge>
                )}
              </span>
              {next.session.place && (
                <FreeText as="span" className="mt-1 block truncate text-xs text-muted">
                  {next.session.place}
                </FreeText>
              )}
              <span className="mt-3 block text-sm text-ink-soft">
                {t("home.examCandidates", { n: next.candidates })}
              </span>
              {next.fees.withFee > 0 && (
                <span
                  className={[
                    "mt-1 block text-sm font-medium",
                    next.fees.missingCentimes > 0 ? "text-owed" : "text-ok",
                  ].join(" ")}
                >
                  {next.fees.missingCentimes > 0 ? (
                    <>
                      {t("exams.feesPaidCount", { paid: next.fees.paid, total: next.fees.withFee })}
                      {" · "}
                      {t("exams.feesMissing")} <Money centimes={next.fees.missingCentimes} />
                    </>
                  ) : (
                    t("exams.feesAllPaid")
                  )}
                </span>
              )}
            </Link>
          )}
        </Panel>
      </div>
      {/* Sessions whose day has gone that nobody marked.
          Rendered ONLY when there are some: a dashboard that congratulates
          you for an empty backlog every morning teaches you to skip the
          dashboard. Placed above the two standing panels because it is the
          only thing here that is actively wrong - every one of these rows is
          a candidate whose hours, and therefore whose balance, is understated
          until somebody decides what happened.

          Marked from here rather than linked away: the owner who skipped
          Thursday will not go hunting through dates for it.

          Below the two standing panels, not above: today and the money are
          what the dashboard is opened for, and a backlog that pushes them
          down the page every morning makes the page worse on the days it is
          long. overflow-hidden because Card rounds its corners but does not
          clip, so the filled header painted past them. */}
      {overdue.length > 0 && (
        <div className="mt-4" data-tour="home-overdue">
          <Card flush tone="warn" className="overflow-hidden">
            <header className="flex items-center gap-2 border-b border-line bg-warn-soft px-4 py-2.5">
              <AlertTriangle className="size-4 shrink-0 text-warn" aria-hidden="true" />
              <h2 className="text-sm font-semibold text-ink">{t("home.overdueTitle")}</h2>
              <span className="rounded-full bg-surface px-2 py-0.5 text-xs font-medium text-muted">
                <Num>{overdue.length}</Num>
              </span>
              <p className="ms-auto text-xs text-muted">{t("home.overdueHint")}</p>
            </header>

            <ul className="divide-y divide-line">
              {overdue.map((row) => (
                <li
                  key={row.id}
                  className="flex flex-wrap items-center gap-x-3 gap-y-2 px-4 py-2.5"
                >
                  {/* The date, because unlike every other panel these rows
                      come from different days. */}
                  <Num className="text-xs font-medium text-warn">{row.day}</Num>
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
                    {personName(row)}
                  </Link>

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
                </li>
              ))}
            </ul>
          </Card>
        </div>
      )}


      <RuntimeBadge />
    </div>
  );
}
