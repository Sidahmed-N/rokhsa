import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Check, ChevronLeft, ChevronRight } from "lucide-react";

import FreeText from "@/components/FreeText";
import { Num } from "@/components/Num";
import Button, { IconButton } from "@/components/ui/Button";
import { ErrorNote } from "@/components/ui/Surface";
import { currentLanguage } from "@/i18n";
import { isAppError } from "@/lib/candidates";
import { isDesktop } from "@/lib/tauri";
import { listRange, shiftIso, todayIso, weekDays, type DayEntry } from "@/lib/training";

import {
  GRID_HEIGHT,
  GRID_HOURS,
  HOUR_HEIGHT,
  nowOffset,
  placeDay,
  untimed,
} from "./grid";
import { KINDS, kindStyle } from "./kinds";
import { candidateName, formatDayLabel, formatDayNumber, formatWeekday } from "./shared";

/**
 * The week, as a time grid.
 *
 * A list told the owner what was booked; a grid tells him what is *not* —
 * and the gap is the thing he acts on. "Thursday afternoon is empty, I can
 * fit someone in" is the question this screen exists to answer, and no list
 * answers it.
 *
 * Blocks are coloured by kind, not by status, so the shape of a week is
 * legible without reading: a wall of blue is a week of road work, a column
 * of purple is a theory day. Status is shown inside the block instead.
 *
 * Blocks are read-only. Marking a session done belongs on the day screen,
 * where there is room to see the plan note and correct the length; clicking
 * one opens it there.
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
  const [kinds, setKinds] = useState<string[]>([]);

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
  const visible = kinds.length === 0 ? rows : rows.filter((r) => kinds.includes(r.kind));
  const planned = visible.filter((r) => r.status === "planned");
  const marker = nowOffset(new Date());

  function toggleKind(kind: string) {
    setKinds((current) =>
      current.includes(kind) ? current.filter((k) => k !== kind) : [...current, kind],
    );
  }

  return (
    <div>
      <div className="mb-4 flex flex-wrap items-center gap-2">
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
            : t("calendar.weekSummary", { planned: planned.length, total: visible.length })}
        </p>
      </div>

      {/* Kind filters. None selected means all — the common case costs no
          clicks, and there is no "All" chip to keep in sync. */}
      <div className="mb-4 flex flex-wrap items-center gap-1.5">
        <button
          type="button"
          onClick={() => setKinds([])}
          aria-pressed={kinds.length === 0}
          className={[
            "inline-flex items-center gap-1.5 rounded-full px-3 py-1 text-xs font-medium transition-colors",
            kinds.length === 0
              ? "bg-ink text-white"
              : "bg-sunken text-ink-soft hover:bg-line",
          ].join(" ")}
        >
          {kinds.length === 0 && <Check className="size-3" aria-hidden="true" />}
          {t("calendar.allKinds")}
        </button>

        {KINDS.map((kind) => {
          const on = kinds.includes(kind);
          const style = kindStyle(kind);
          return (
            <button
              key={kind}
              type="button"
              onClick={() => toggleKind(kind)}
              aria-pressed={on}
              className={[
                "inline-flex items-center gap-1.5 rounded-full px-3 py-1 text-xs font-medium transition-colors",
                on ? style.chip : "bg-sunken text-ink-soft hover:bg-line",
              ].join(" ")}
            >
              <span className={["size-1.5 rounded-full", style.dot].join(" ")} aria-hidden="true" />
              {t(`lessons.kind_${kind}`)}
            </button>
          );
        })}
      </div>

      {error && <ErrorNote>{error}</ErrorNote>}

      <div className="overflow-hidden rounded-lg border border-line bg-surface shadow-card">
        {/* Day headings */}
        <div className="flex border-b border-line">
          <div className="w-14 shrink-0 border-e border-line" />
          {days.map((day) => {
            const isToday = day === today;
            return (
              <button
                key={day}
                type="button"
                onClick={() => openDay(day)}
                className="flex-1 border-e border-line py-2 text-center transition-colors last:border-e-0 hover:bg-sunken"
              >
                <span
                  className={[
                    "inline-flex items-center gap-1.5 rounded-full px-2.5 py-1 text-xs font-medium",
                    isToday ? "bg-ink text-white" : "text-muted",
                  ].join(" ")}
                >
                  {formatWeekday(day, locale)}
                  <Num className={isToday ? "text-white" : "text-ink"}>
                    {formatDayNumber(day, locale)}
                  </Num>
                </span>
              </button>
            );
          })}
        </div>

        {/* Sessions with no time booked. Some schools say "Thursday" and the
            candidate turns up when he can, so these cannot be dropped. */}
        {days.some((day) => untimed(visible.filter((r) => r.day === day)).length > 0) && (
          <div className="flex border-b border-line bg-sunken/40">
            <div className="flex w-14 shrink-0 items-center justify-center border-e border-line px-1 py-1.5 text-center text-[0.625rem] leading-tight text-faint">
              {t("calendar.noTime")}
            </div>
            {days.map((day) => (
              <div
                key={day}
                className="flex-1 space-y-1 border-e border-line p-1 last:border-e-0"
              >
                {untimed(visible.filter((r) => r.day === day)).map((entry) => (
                  <button
                    key={entry.id}
                    type="button"
                    onClick={() => openDay(day)}
                    className={[
                      "block w-full truncate rounded px-1.5 py-1 text-start text-[0.6875rem] font-medium",
                      kindStyle(entry.kind).block,
                    ].join(" ")}
                  >
                    {candidateName(entry, locale)}
                  </button>
                ))}
              </div>
            ))}
          </div>
        )}

        {/* The grid itself */}
        <div className="flex">
          <div className="w-14 shrink-0 border-e border-line">
            {GRID_HOURS.slice(0, -1).map((hour) => (
              <div
                key={hour}
                style={{ height: HOUR_HEIGHT }}
                className="relative border-b border-line last:border-b-0"
              >
                <Num className="absolute -top-2 end-1.5 text-[0.6875rem] text-faint">
                  {`${String(hour).padStart(2, "0")}:00`}
                </Num>
              </div>
            ))}
          </div>

          {days.map((day) => {
            const isToday = day === today;
            const placed = placeDay(visible.filter((r) => r.day === day));

            return (
              <div
                key={day}
                className={[
                  "relative flex-1 border-e border-line last:border-e-0",
                  isToday ? "bg-brand-soft/30" : "",
                ].join(" ")}
                style={{ height: GRID_HEIGHT }}
              >
                {/* Hour lines */}
                {GRID_HOURS.slice(0, -1).map((hour) => (
                  <div
                    key={hour}
                    style={{ height: HOUR_HEIGHT }}
                    className="border-b border-line"
                  />
                ))}

                {/* Current time */}
                {isToday && marker !== null && (
                  <div
                    className="pointer-events-none absolute inset-x-0 z-10 flex items-center"
                    style={{ top: marker }}
                    aria-hidden="true"
                  >
                    <span className="size-1.5 rounded-full bg-owed" />
                    <span className="h-px flex-1 bg-owed" />
                  </div>
                )}

                {placed.map(({ entry, top, height, widthPercent, leftPercent }) => {
                  const settled = entry.status !== "planned";
                  return (
                    <button
                      key={entry.id}
                      type="button"
                      onClick={() => openDay(day)}
                      style={{
                        top,
                        height,
                        // Inline-start, so the columns pack from the right
                        // in Arabic without any extra work.
                        insetInlineStart: `${leftPercent}%`,
                        width: `calc(${widthPercent}% - 4px)`,
                      }}
                      className={[
                        "absolute z-[5] mx-0.5 overflow-hidden rounded-md px-1.5 py-1 text-start",
                        "transition-shadow hover:shadow-raised",
                        kindStyle(entry.kind).block,
                        settled ? "opacity-55" : "",
                      ].join(" ")}
                    >
                      <span className="flex items-center gap-1 text-[0.6875rem] leading-tight font-semibold">
                        {entry.status === "done" && (
                          <Check className="size-3 shrink-0" aria-hidden="true" />
                        )}
                        <Num>{entry.scheduledTime ?? ""}</Num>
                      </span>
                      <span className="block truncate text-[0.6875rem] leading-tight font-medium">
                        {candidateName(entry, locale)}
                      </span>
                      {height > 44 && entry.planNote && (
                        <FreeText
                          as="span"
                          className="block truncate text-[0.625rem] leading-tight opacity-80"
                        >
                          {entry.planNote}
                        </FreeText>
                      )}
                    </button>
                  );
                })}
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
