import { useCallback, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { useTranslation } from "react-i18next";
import {
  CalendarPlus,
  Check,
  ChevronLeft,
  ChevronRight,
  Phone,
  Plus,
  X,
} from "lucide-react";

import FreeText from "@/components/FreeText";
import { Duration, Num } from "@/components/Num";
import Button, { IconButton } from "@/components/ui/Button";
import DateField from "@/components/ui/DateField";
import { Badge, Card, EmptyState, ErrorNote, type Tone } from "@/components/ui/Surface";
import { isAppError } from "@/lib/candidates";
import { personName } from "@/lib/names";
import { isDesktop } from "@/lib/tauri";
import {
  completeLesson,
  listDay,
  setLessonNotHeld,
  shiftIso,
  todayIso,
  type DayEntry,
} from "@/lib/training";

import { kindStyle } from "@/lib/kinds";

const STATUS_TONE: Record<string, Tone> = {
  planned: "brand",
  done: "ok",
  missed: "warn",
  cancelled: "neutral",
};

/**
 * One day, as a to-do list.
 *
 * Answers the question the school actually opens the app with: who is coming
 * in, and what are they doing. Until this existed the app only accounted for
 * the day after it had happened, which left the wall planner in charge — and
 * an app that doesn't help run the day gets opened weekly, at which point the
 * hours go stale and the money screens that depend on them go wrong.
 *
 * Marking a session done here is what records the hours, so logging stops
 * being a separate chore someone has to remember. "Fait" is the only primary
 * button on the screen for that reason: it is pressed many times a day, and
 * the two beside it are exceptions.
 */
export default function DayView({
  date,
  setDate,
  onBook,
}: {
  date: string;
  setDate: (iso: string) => void;
  /**
   * The second argument is set when rebooking for somebody who missed: the
   * owner said he has to make it up another week, and the app knowing who and
   * which kind is the difference between one click and finding the candidate
   * again.
   */
  onBook: (day: string, makeUp?: { enrollmentId: number; kind: string }) => void;
}) {
  const { t } = useTranslation();

  const [rows, setRows] = useState<DayEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const errorText = useCallback(
    (raw: unknown): string =>
      isAppError(raw) ? t(raw.code, { defaultValue: raw.message }) : String(raw),
    [t],
  );

  const refresh = useCallback(async () => {
    if (!isDesktop()) {
      setError(t("candidates.needsDesktop"));
      setLoading(false);
      return;
    }
    setLoading(true);
    try {
      setRows(await listDay(date));
      setError(null);
    } catch (raw: unknown) {
      setError(errorText(raw));
    } finally {
      setLoading(false);
    }
  }, [date, errorText, t]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  async function run(action: () => Promise<unknown>) {
    try {
      await action();
      await refresh();
    } catch (raw: unknown) {
      setError(errorText(raw));
    }
  }

  const planned = rows.filter((r) => r.status === "planned");
  const isToday = date === todayIso();

  return (
    <div>
      <div className="mb-5 flex flex-wrap items-center gap-2">
        {/* The chevrons point at the previous and next day on screen. In
            Arabic the page is mirrored, so they are rotated to keep pointing
            the way the reader is travelling. */}
        <IconButton
          aria-label={t("calendar.previousDay")}
          onClick={() => setDate(shiftIso(date, -1))}
          variant="secondary"
        >
          <ChevronLeft className="size-4 rtl:rotate-180" aria-hidden="true" />
        </IconButton>

        <DateField value={date} onChange={setDate} aria-label={t("calendar.day")} />

        <IconButton
          aria-label={t("calendar.nextDay")}
          onClick={() => setDate(shiftIso(date, 1))}
          variant="secondary"
        >
          <ChevronRight className="size-4 rtl:rotate-180" aria-hidden="true" />
        </IconButton>

        {!isToday && (
          <Button onClick={() => setDate(todayIso())}>{t("calendar.backToToday")}</Button>
        )}

        <Button
          variant="primary"
          icon={<Plus className="size-4" aria-hidden="true" />}
          onClick={() => onBook(date)}
        >
          {t("calendar.book")}
        </Button>

        <p className="ms-auto text-sm text-muted">
          {loading
            ? t("common.loading")
            : t("calendar.summary", { planned: planned.length, total: rows.length })}
        </p>
      </div>

      {error && <ErrorNote>{error}</ErrorNote>}

      {!loading && rows.length === 0 && (
        <EmptyState
          title={t("calendar.empty")}
          action={
            <Button
              variant="primary"
              icon={<Plus className="size-4" aria-hidden="true" />}
              onClick={() => onBook(date)}
            >
              {t("calendar.book")}
            </Button>
          }
        />
      )}

      <div className="grid gap-2.5">
        {rows.map((row) => {
          const settled = row.status !== "planned";

          return (
            <Card
              key={row.id}
              flush
              tone={row.status === "done" ? "ok" : undefined}
              className={settled ? "opacity-75" : ""}
            >
              <div className="flex">
                {/* Time rail. Gives the day a spine to scan down, instead of
                    the eye hunting for the hour inside each row. */}
                <div
                  className={[
                    "flex w-24 shrink-0 flex-col items-center justify-center px-3 py-3.5",
                    kindStyle(row.kind).block,
                  ].join(" ")}
                >
                  {row.scheduledTime ? (
                    <Num className="text-base font-semibold">{row.scheduledTime}</Num>
                  ) : (
                    <span className="text-center text-xs opacity-70">{t("calendar.noTime")}</span>
                  )}
                  <span className="mt-0.5 text-xs opacity-80">
                    <Duration minutes={row.minutes} />
                  </span>
                </div>

                <div className="min-w-0 flex-1 px-4 py-3">
                  <div className="flex flex-wrap items-center gap-x-2.5 gap-y-1">
                    <Link
                      to={`/candidates/${row.candidateId}`}
                      className="text-sm font-semibold text-ink hover:text-brand hover:underline"
                    >
                      {personName(row)}
                    </Link>
                    <span className="text-xs font-medium text-muted">
                      {t(`lessons.kind_${row.kind}`)}
                    </span>

                    <Badge tone={STATUS_TONE[row.status] ?? "neutral"} className="ms-auto">
                      {t(`calendar.status_${row.status}`)}
                    </Badge>
                  </div>

                  {/* What to work on — the reason this screen is worth opening. */}
                  {row.planNote && (
                    <div className="mt-1.5">
                      <FreeText className="text-sm text-ink-soft">{row.planNote}</FreeText>
                    </div>
                  )}
                  {row.status === "done" && row.notes && (
                    <div className="mt-1.5">
                      <FreeText className="text-sm text-muted">{row.notes}</FreeText>
                    </div>
                  )}

                  <div className="mt-2.5 flex flex-wrap items-center gap-2">
                    {row.phone && (
                      <span className="inline-flex items-center gap-1.5 text-xs text-muted">
                        <Phone className="size-3.5 text-faint" aria-hidden="true" />
                        <Num>{row.phone}</Num>
                      </span>
                    )}

                    {/* A missed session is history and cannot be moved, so a
                        make-up is a NEW booking. The absence stays on the
                        record: a candidate who misses often is something an
                        owner wants to see. */}
                    {row.status === "missed" && (
                      <div className="ms-auto">
                        <Button
                          size="sm"
                          icon={<CalendarPlus className="size-3.5" aria-hidden="true" />}
                          onClick={() =>
                            onBook(date, {
                              enrollmentId: row.enrollmentId,
                              kind: row.kind,
                            })
                          }
                        >
                          {t("calendar.rebook")}
                        </Button>
                      </div>
                    )}

                    {row.status === "planned" && (
                      <div className="ms-auto flex flex-wrap gap-1.5">
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
                        <Button
                          variant="ghost"
                          size="sm"
                          icon={<X className="size-3.5" aria-hidden="true" />}
                          onClick={() => void run(() => setLessonNotHeld(row.id, "cancelled"))}
                        >
                          {t("calendar.markCancelled")}
                        </Button>
                      </div>
                    )}
                  </div>
                </div>
              </div>
            </Card>
          );
        })}
      </div>
    </div>
  );
}
