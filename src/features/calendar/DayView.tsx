import { useCallback, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { Check, ChevronLeft, ChevronRight, Phone, X } from "lucide-react";

import { Duration, Num } from "@/components/Num";
import Button, { IconButton } from "@/components/ui/Button";
import DateField from "@/components/ui/DateField";
import { Badge, Card, EmptyState, ErrorNote, type Tone } from "@/components/ui/Surface";
import { currentLanguage } from "@/i18n";
import { isAppError } from "@/lib/candidates";
import { isDesktop } from "@/lib/tauri";
import {
  completeLesson,
  listDay,
  setLessonNotHeld,
  shiftIso,
  todayIso,
  type DayEntry,
} from "@/lib/training";

import { candidateName } from "./shared";

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
}: {
  date: string;
  setDate: (iso: string) => void;
}) {
  const { t } = useTranslation();
  const locale = currentLanguage();

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

        <p className="ms-auto text-sm text-muted">
          {loading
            ? t("common.loading")
            : t("calendar.summary", { planned: planned.length, total: rows.length })}
        </p>
      </div>

      {error && <ErrorNote>{error}</ErrorNote>}

      {!loading && rows.length === 0 && (
        <EmptyState title={t("calendar.empty")} description={t("calendar.emptyHint")} />
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
                <div className="flex w-24 shrink-0 flex-col items-center justify-center border-e border-line px-3 py-3.5">
                  {row.scheduledTime ? (
                    <Num className="text-base font-semibold text-ink">{row.scheduledTime}</Num>
                  ) : (
                    <span className="text-center text-xs text-faint">
                      {t("calendar.noTime")}
                    </span>
                  )}
                  <span className="mt-0.5 text-xs text-muted">
                    <Duration minutes={row.minutes} />
                  </span>
                </div>

                <div className="min-w-0 flex-1 px-4 py-3">
                  <div className="flex flex-wrap items-center gap-x-2.5 gap-y-1">
                    <Link
                      to={`/candidates/${row.candidateId}`}
                      className="text-sm font-semibold text-ink hover:text-brand hover:underline"
                    >
                      {candidateName(row, locale)}
                    </Link>
                    <Badge tone="brand">{t(`lessons.kind_${row.kind}`)}</Badge>

                    <Badge tone={STATUS_TONE[row.status] ?? "neutral"} className="ms-auto">
                      {t(`calendar.status_${row.status}`)}
                    </Badge>
                  </div>

                  {/* What to work on — the reason this screen is worth opening. */}
                  {row.planNote && <p className="mt-1.5 text-sm text-ink-soft">{row.planNote}</p>}
                  {row.status === "done" && row.notes && (
                    <p className="mt-1.5 text-sm text-muted">{row.notes}</p>
                  )}

                  <div className="mt-2.5 flex flex-wrap items-center gap-2">
                    {row.phone && (
                      <span className="inline-flex items-center gap-1.5 text-xs text-muted">
                        <Phone className="size-3.5 text-faint" aria-hidden="true" />
                        <Num>{row.phone}</Num>
                      </span>
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
