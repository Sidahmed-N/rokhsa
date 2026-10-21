import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ChevronLeft, ChevronRight } from "lucide-react";

import { Duration, Num } from "@/components/Num";
import Button, { IconButton } from "@/components/ui/Button";
import { Badge, Card, ErrorNote, type Tone } from "@/components/ui/Surface";
import { currentLanguage } from "@/i18n";
import { isAppError } from "@/lib/candidates";
import { isDesktop } from "@/lib/tauri";
import { listRange, shiftIso, todayIso, weekDays, type DayEntry } from "@/lib/training";

import { candidateName, formatDayLabel, formatWeekday, formatDayNumber } from "./shared";

const STATUS_TONE: Record<string, Tone> = {
  planned: "brand",
  done: "ok",
  missed: "warn",
  cancelled: "neutral",
};

/**
 * The week, as a diary.
 *
 * The day screen answers "what is happening now"; this answers the questions
 * an owner asks when he is planning rather than working — who is coming on
 * Thursday, is next week filling up, which days are empty enough to offer
 * someone a slot. Without it the real schedule stays on the wall planner.
 *
 * Rows are deliberately read-only. Marking a session done belongs on the day
 * screen, where there is room to see the plan note and correct the length;
 * clicking a day here opens it there.
 */
export default function WeekView({
  date,
  setDate,
  openDay,
}: {
  date: string;
  setDate: (iso: string) => void;
  openDay: (iso: string) => void;
}) {
  const { t } = useTranslation();
  const locale = currentLanguage();

  const [rows, setRows] = useState<DayEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const days = weekDays(date);
  const from = days[0] ?? date;
  const to = days[6] ?? date;

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
      setRows(await listRange(from, to));
      setError(null);
    } catch (raw: unknown) {
      setError(errorText(raw));
    } finally {
      setLoading(false);
    }
  }, [from, to, errorText, t]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const today = todayIso();
  const planned = rows.filter((r) => r.status === "planned");

  return (
    <div>
      <div className="mb-5 flex flex-wrap items-center gap-2">
        <IconButton
          aria-label={t("calendar.previousWeek")}
          onClick={() => setDate(shiftIso(date, -7))}
          variant="secondary"
        >
          <ChevronLeft className="size-4 rtl:rotate-180" aria-hidden="true" />
        </IconButton>

        <span className="inline-flex h-9 items-center rounded-md border border-line-strong bg-surface px-3 text-sm font-medium shadow-card">
          <Num>{formatDayLabel(from, locale)}</Num>
          <span className="px-1.5 text-faint">–</span>
          <Num>{formatDayLabel(to, locale)}</Num>
        </span>

        <IconButton
          aria-label={t("calendar.nextWeek")}
          onClick={() => setDate(shiftIso(date, 7))}
          variant="secondary"
        >
          <ChevronRight className="size-4 rtl:rotate-180" aria-hidden="true" />
        </IconButton>

        <Button onClick={() => setDate(today)}>{t("calendar.thisWeek")}</Button>

        <p className="ms-auto text-sm text-muted">
          {loading
            ? t("common.loading")
            : t("calendar.weekSummary", { planned: planned.length, total: rows.length })}
        </p>
      </div>

      {error && <ErrorNote>{error}</ErrorNote>}

      <div className="grid gap-2">
        {days.map((day) => {
          const forDay = rows.filter((r) => r.day === day);
          const isToday = day === today;

          return (
            <Card key={day} flush tone={isToday ? "brand" : undefined}>
              <div className="flex">
                {/* Date rail, mirroring the day view's time rail so the two
                    tabs feel like one screen in two zooms. */}
                <button
                  type="button"
                  onClick={() => openDay(day)}
                  className={[
                    "flex w-24 shrink-0 flex-col items-center justify-center gap-0.5",
                    "border-e border-line px-3 py-3 transition-colors hover:bg-sunken",
                    isToday ? "bg-brand-soft" : "",
                  ].join(" ")}
                >
                  <span
                    className={[
                      "text-xs",
                      isToday ? "font-medium text-brand" : "text-muted",
                    ].join(" ")}
                  >
                    {formatWeekday(day, locale)}
                  </span>
                  <Num
                    className={[
                      "text-lg font-semibold",
                      isToday ? "text-brand" : "text-ink",
                    ].join(" ")}
                  >
                    {formatDayNumber(day, locale)}
                  </Num>
                </button>

                <div className="min-w-0 flex-1">
                  {forDay.length === 0 ? (
                    <p className="px-4 py-4 text-sm text-faint">{t("calendar.dayEmpty")}</p>
                  ) : (
                    <ul className="divide-y divide-line">
                      {forDay.map((row) => (
                        <li
                          key={row.id}
                          className={[
                            "flex flex-wrap items-center gap-x-3 gap-y-1 px-4 py-2.5 text-sm",
                            row.status === "missed" || row.status === "cancelled"
                              ? "opacity-60"
                              : "",
                          ].join(" ")}
                        >
                          <Num className="w-12 font-semibold text-ink">
                            {row.scheduledTime ?? "—"}
                          </Num>
                          <span className="font-medium text-ink">
                            {candidateName(row, locale)}
                          </span>
                          <Badge tone="brand">{t(`lessons.kind_${row.kind}`)}</Badge>
                          <span className="text-xs text-muted">
                            <Duration minutes={row.minutes} />
                          </span>
                          {row.planNote && (
                            <span className="truncate text-xs text-muted">{row.planNote}</span>
                          )}
                          <Badge
                            tone={STATUS_TONE[row.status] ?? "neutral"}
                            className="ms-auto"
                          >
                            {t(`calendar.status_${row.status}`)}
                          </Badge>
                        </li>
                      ))}
                    </ul>
                  )}
                </div>
              </div>
            </Card>
          );
        })}
      </div>
    </div>
  );
}
