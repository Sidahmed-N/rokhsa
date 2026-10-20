import { useCallback, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { useTranslation } from "react-i18next";

import { Duration, Num } from "@/components/Num";
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

function name(row: DayEntry, locale: "fr" | "ar"): string {
  const arabic = `${row.lastNameAr} ${row.firstNameAr}`.trim();
  const latin = `${row.lastNameFr} ${row.firstNameFr}`.trim();
  return locale === "ar" ? arabic || latin : latin || arabic;
}

/**
 * The day.
 *
 * Answers the question the school actually opens the app with: who is coming
 * in, and what are they doing. Until this existed the app only accounted for
 * the day after it had happened, which left the wall planner in charge — and
 * an app that doesn't help run the day gets opened weekly, at which point the
 * hours go stale and the money screens that depend on them go wrong.
 *
 * Marking a session done here is what records the hours, so logging stops
 * being a separate chore someone has to remember.
 */
export default function TodayPage() {
  const { t } = useTranslation();
  const locale = currentLanguage();

  const [date, setDate] = useState(todayIso());
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
      <div className="mb-4 flex flex-wrap items-center gap-2">
        <h1 className="text-xl font-semibold">{t("today.title")}</h1>
        <button
          type="button"
          onClick={() => setDate((d) => shiftIso(d, -1))}
          className="rounded-md border border-line px-3 py-1.5 text-sm"
        >
          ← {t("today.previousDay")}
        </button>
        <input
          type="date"
          value={date}
          dir="ltr"
          onChange={(e) => setDate(e.target.value)}
          className="rounded-md border border-line bg-white px-3 py-1.5 text-sm"
        />
        <button
          type="button"
          onClick={() => setDate((d) => shiftIso(d, 1))}
          className="rounded-md border border-line px-3 py-1.5 text-sm"
        >
          {t("today.nextDay")} →
        </button>
        {!isToday && (
          <button
            type="button"
            onClick={() => setDate(todayIso())}
            className="rounded-md bg-brand px-3 py-1.5 text-sm font-medium text-white"
          >
            {t("today.backToToday")}
          </button>
        )}
      </div>

      {error && (
        <p className="mb-3 rounded-md border border-owed bg-white px-3 py-2 text-sm text-owed">
          {error}
        </p>
      )}

      <p className="mb-3 text-sm text-muted">
        {loading
          ? t("common.loading")
          : t("today.summary", { planned: planned.length, total: rows.length })}
      </p>

      {!loading && rows.length === 0 && (
        <p className="rounded-lg border border-line bg-white px-4 py-8 text-center text-sm text-muted">
          {t("today.empty")}
        </p>
      )}

      <div className="grid gap-3">
        {rows.map((row) => {
          const done = row.status === "done";
          const notHeld = row.status === "missed" || row.status === "cancelled";

          return (
            <section
              key={row.id}
              className={[
                "rounded-lg border bg-white px-4 py-3",
                done ? "border-ok" : notHeld ? "border-line opacity-60" : "border-line",
              ].join(" ")}
            >
              <div className="flex flex-wrap items-baseline gap-3">
                <span className="text-sm font-semibold">
                  {row.scheduledTime ? (
                    <Num>{row.scheduledTime}</Num>
                  ) : (
                    <span className="text-muted">{t("today.noTime")}</span>
                  )}
                </span>

                <Link
                  to={`/candidates/${row.candidateId}`}
                  className="text-base font-medium text-brand hover:underline"
                >
                  {name(row, locale)}
                </Link>

                <span className="rounded-full bg-brand-soft px-2 py-0.5 text-xs text-brand">
                  {t(`lessons.kind_${row.kind}`)}
                </span>

                <span className="text-xs text-muted">
                  <Duration minutes={row.minutes} />
                </span>

                {row.phone && (
                  <span className="text-xs text-muted">
                    <Num>{row.phone}</Num>
                  </span>
                )}

                <span
                  className={[
                    "ms-auto text-xs",
                    done ? "text-ok" : notHeld ? "text-warn" : "text-muted",
                  ].join(" ")}
                >
                  {t(`today.status_${row.status}`)}
                </span>
              </div>

              {/* What to work on — the reason this screen is worth opening. */}
              {row.planNote && (
                <p className="mt-2 rounded-md bg-brand-soft px-3 py-1.5 text-sm">
                  {row.planNote}
                </p>
              )}
              {done && row.notes && (
                <p className="mt-2 text-sm text-muted">{row.notes}</p>
              )}

              {row.status === "planned" && (
                <div className="mt-3 flex flex-wrap gap-2">
                  <button
                    type="button"
                    onClick={() =>
                      void run(() =>
                        completeLesson(row.id, { occurredOn: "", minutes: 0, notes: "" }),
                      )
                    }
                    className="rounded-md bg-brand px-3 py-1.5 text-xs font-medium text-white"
                  >
                    {t("today.markDone")}
                  </button>
                  <button
                    type="button"
                    onClick={() => void run(() => setLessonNotHeld(row.id, "missed"))}
                    className="rounded-md border border-line px-3 py-1.5 text-xs"
                  >
                    {t("today.markMissed")}
                  </button>
                  <button
                    type="button"
                    onClick={() => void run(() => setLessonNotHeld(row.id, "cancelled"))}
                    className="rounded-md border border-line px-3 py-1.5 text-xs"
                  >
                    {t("today.markCancelled")}
                  </button>
                </div>
              )}
            </section>
          );
        })}
      </div>
    </div>
  );
}
