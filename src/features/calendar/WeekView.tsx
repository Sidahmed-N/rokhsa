import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { Duration, Num } from "@/components/Num";
import { currentLanguage } from "@/i18n";
import { isAppError } from "@/lib/candidates";
import { isDesktop } from "@/lib/tauri";
import {
  listRange,
  shiftIso,
  todayIso,
  weekDays,
  type DayEntry,
} from "@/lib/training";

import { candidateName, formatDayLabel } from "./shared";

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
      <div className="mb-4 flex flex-wrap items-center gap-2">
        <button
          type="button"
          onClick={() => setDate(shiftIso(date, -7))}
          className="rounded-md border border-line px-3 py-1.5 text-sm"
        >
          ← {t("calendar.previousWeek")}
        </button>
        <span className="rounded-md border border-line bg-white px-3 py-1.5 text-sm">
          <Num>{formatDayLabel(from, locale)}</Num>
          <span className="px-1">–</span>
          <Num>{formatDayLabel(to, locale)}</Num>
        </span>
        <button
          type="button"
          onClick={() => setDate(shiftIso(date, 7))}
          className="rounded-md border border-line px-3 py-1.5 text-sm"
        >
          {t("calendar.nextWeek")} →
        </button>
        <button
          type="button"
          onClick={() => setDate(today)}
          className="rounded-md border border-line px-3 py-1.5 text-sm"
        >
          {t("calendar.thisWeek")}
        </button>
      </div>

      {error && (
        <p className="mb-3 rounded-md border border-owed bg-white px-3 py-2 text-sm text-owed">
          {error}
        </p>
      )}

      <p className="mb-3 text-sm text-muted">
        {loading
          ? t("common.loading")
          : t("calendar.weekSummary", { planned: planned.length, total: rows.length })}
      </p>

      <div className="grid gap-2">
        {days.map((day) => {
          const forDay = rows.filter((r) => r.day === day);
          const isToday = day === today;

          return (
            <section
              key={day}
              className={[
                "rounded-lg border bg-white",
                isToday ? "border-brand" : "border-line",
              ].join(" ")}
            >
              <div className="flex items-baseline gap-2 border-b border-line px-4 py-2">
                <button
                  type="button"
                  onClick={() => openDay(day)}
                  className={[
                    "text-sm font-semibold hover:underline",
                    isToday ? "text-brand" : "",
                  ].join(" ")}
                >
                  <Num>{formatDayLabel(day, locale)}</Num>
                </button>
                {isToday && (
                  <span className="rounded-full bg-brand-soft px-2 py-0.5 text-xs text-brand">
                    {t("calendar.todayBadge")}
                  </span>
                )}
                <span className="ms-auto text-xs text-muted">
                  {forDay.length === 0
                    ? t("calendar.dayEmpty")
                    : t("calendar.dayCount", { count: forDay.length })}
                </span>
              </div>

              {forDay.length > 0 && (
                <ul className="divide-y divide-line">
                  {forDay.map((row) => {
                    const done = row.status === "done";
                    const notHeld =
                      row.status === "missed" || row.status === "cancelled";

                    return (
                      <li
                        key={row.id}
                        className={[
                          "flex flex-wrap items-baseline gap-3 px-4 py-2 text-sm",
                          notHeld ? "opacity-60" : "",
                        ].join(" ")}
                      >
                        <span className="font-medium">
                          {row.scheduledTime ? (
                            <Num>{row.scheduledTime}</Num>
                          ) : (
                            <span className="text-muted">{t("calendar.noTime")}</span>
                          )}
                        </span>
                        <span>{candidateName(row, locale)}</span>
                        <span className="rounded-full bg-brand-soft px-2 py-0.5 text-xs text-brand">
                          {t(`lessons.kind_${row.kind}`)}
                        </span>
                        <span className="text-xs text-muted">
                          <Duration minutes={row.minutes} />
                        </span>
                        {row.planNote && (
                          <span className="text-xs text-muted">{row.planNote}</span>
                        )}
                        <span
                          className={[
                            "ms-auto text-xs",
                            done ? "text-ok" : notHeld ? "text-warn" : "text-muted",
                          ].join(" ")}
                        >
                          {t(`calendar.status_${row.status}`)}
                        </span>
                      </li>
                    );
                  })}
                </ul>
              )}
            </section>
          );
        })}
      </div>
    </div>
  );
}
