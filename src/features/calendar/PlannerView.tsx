import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import FullCalendar from "@fullcalendar/react";
import dayGridPlugin from "@fullcalendar/daygrid";
import timeGridPlugin from "@fullcalendar/timegrid";
import interactionPlugin from "@fullcalendar/interaction";
import arLocale from "@fullcalendar/core/locales/ar-dz";
import frLocale from "@fullcalendar/core/locales/fr";
import type { EventDropArg, EventInput } from "@fullcalendar/core";
import type { EventResizeDoneArg } from "@fullcalendar/interaction";

import { ErrorNote } from "@/components/ui/Surface";
import { currentLanguage } from "@/i18n";
import { isAppError } from "@/lib/candidates";
import { KINDS } from "@/lib/kinds";
import { personName } from "@/lib/names";
import { isDesktop } from "@/lib/tauri";
import { listRange, rescheduleLesson, type DayEntry } from "@/lib/training";

import { GRID_END_HOUR, GRID_START_HOUR } from "./grid";

/**
 * The planning calendar — week and month.
 *
 * Built on FullCalendar rather than by hand. Laying overlapping sessions out
 * in a day column is the one genuinely hard problem on this screen, and the
 * version written here divided the column evenly: at a realistic fifty-odd
 * sessions a week that left three candidates sharing a mid-morning slot about
 * thirty pixels each, too narrow for a time, let alone a name.
 *
 * The day view is deliberately NOT this. It is a to-do list with Fait and
 * Absent on every row, which is the morning routine and the thing that keeps
 * the hours current; a grid would make that worse, not better. Week and month
 * are for planning, and that is what a calendar library is good at.
 *
 * Licence note: core, timegrid, daygrid and interaction are MIT and free to
 * ship in a commercial product. Columns per instructor or per vehicle are
 * FullCalendar Premium and are NOT included — see docs/03-STACK.md before
 * promising that as a feature.
 */

/** FullCalendar wants a local ISO datetime; a session may have no time. */
function startOf(entry: DayEntry): string {
  return entry.scheduledTime ? `${entry.day}T${entry.scheduledTime}:00` : entry.day;
}

function endOf(entry: DayEntry): string | undefined {
  if (!entry.scheduledTime) return undefined;
  const [h, m] = entry.scheduledTime.split(":").map(Number);
  const total = (h ?? 0) * 60 + (m ?? 0) + entry.minutes;
  const pad = (n: number) => String(n).padStart(2, "0");
  // A session running past midnight is not a thing a school books, so
  // clamping at 23:59 is safer than spilling onto the next day.
  const end = Math.min(total, 23 * 60 + 59);
  return `${entry.day}T${pad(Math.floor(end / 60))}:${pad(end % 60)}:00`;
}

/** "YYYY-MM-DD" and "HH:MM" from a Date, in LOCAL time, never UTC. */
function localParts(date: Date): { day: string; time: string } {
  const pad = (n: number) => String(n).padStart(2, "0");
  return {
    day: `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`,
    time: `${pad(date.getHours())}:${pad(date.getMinutes())}`,
  };
}

export default function PlannerView({
  view,
  date,
  setDate,
  openDay,
  onPickSlot,
}: {
  view: "week" | "month";
  date: string;
  setDate: (iso: string) => void;
  openDay: (iso: string) => void;
  /** Clicking empty space books there — day, and time when the view has one. */
  onPickSlot: (day: string, time: string | null) => void;
}) {
  const { t } = useTranslation();
  const locale = currentLanguage();
  const calendar = useRef<FullCalendar>(null);

  const [rows, setRows] = useState<DayEntry[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [kinds, setKinds] = useState<string[]>([]);
  const [range, setRange] = useState<{ from: string; to: string } | null>(null);

  const errorText = useCallback(
    (raw: unknown): string =>
      isAppError(raw) ? t(raw.code, { defaultValue: raw.message }) : String(raw),
    [t],
  );

  const refresh = useCallback(async () => {
    if (!range) return;
    if (!isDesktop()) {
      setError(t("candidates.needsDesktop"));
      return;
    }
    try {
      setRows(await listRange(range.from, range.to));
      setError(null);
    } catch (raw: unknown) {
      setError(errorText(raw));
    }
  }, [range, errorText, t]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  // Keep the shared date in step with whatever the calendar is showing, so
  // switching to the day tab lands on the week you were looking at.
  useEffect(() => {
    const api = calendar.current?.getApi();
    if (!api) return;
    const current = localParts(api.getDate()).day;
    if (current !== date) api.gotoDate(date);
  }, [date]);

  const visible = kinds.length === 0 ? rows : rows.filter((r) => kinds.includes(r.kind));

  /**
   * One block per SESSION, not per candidate.
   *
   * A créneau taught to three people is three lesson rows sharing a session
   * id (migration 0009). Drawn one block each they land on the same slot and
   * collapse into "+2", which is the cramping this view was rebuilt to fix —
   * and it would misread as three separate lessons rather than one with three
   * people in it.
   */
  const events: EventInput[] = useMemo(() => {
    const bySession = new Map<string, DayEntry[]>();
    for (const entry of visible) {
      // A lesson with no session id is its own session.
      const key = entry.sessionId === null ? `l${entry.id}` : `s${entry.sessionId}`;
      const found = bySession.get(key);
      if (found) found.push(entry);
      else bySession.set(key, [entry]);
    }

    return [...bySession.values()].flatMap((group) => {
      const first = group[0];
      if (!first) return [];

      const event: EventInput = {
        // The first row carries the block: dragging it reschedules the whole
        // session, because db::lessons::reschedule moves every row sharing
        // the session id.
        id: String(first.id),
        title: group.map((e) => personName(e, locale)).join(" · "),
        start: startOf(first),
        allDay: !first.scheduledTime,
        editable: first.status === "planned",
        classNames: [`fc-kind-${first.kind}`, `fc-status-${first.status}`],
        extendedProps: {
          candidateId: first.candidateId,
          planNote: first.planNote,
          kindLabel: t(`lessons.kind_${first.kind}`),
          statusLabel: t(`calendar.status_${first.status}`),
          names: group.map((e) => personName(e, locale)),
        },
      };
      const end = endOf(first);
      if (end) event.end = end;
      return [event];
    });
  }, [visible, locale, t]);

  async function move(id: string, start: Date | null, end: Date | null, revert: () => void) {
    if (!start) return revert();
    const { day, time } = localParts(start);
    const minutes = end ? Math.round((end.getTime() - start.getTime()) / 60000) : 60;
    try {
      await rescheduleLesson(Number(id), day, time, minutes);
      await refresh();
    } catch (raw: unknown) {
      // Put the block back where it was: the screen must never show a time
      // the database did not accept.
      revert();
      setError(errorText(raw));
    }
  }

  function toggleKind(kind: string) {
    setKinds((current) =>
      current.includes(kind) ? current.filter((k) => k !== kind) : [...current, kind],
    );
  }

  return (
    <div>
      {/* Kind filters. None selected means all — the common case costs no
          clicks, and there is no "All" state to keep in sync. */}
      <div className="mb-4 flex flex-wrap items-center gap-1.5">
        <button
          type="button"
          onClick={() => setKinds([])}
          aria-pressed={kinds.length === 0}
          className={[
            "rounded-full px-3 py-1 text-xs font-medium transition-colors",
            kinds.length === 0 ? "bg-ink text-white" : "bg-sunken text-ink-soft hover:bg-line",
          ].join(" ")}
        >
          {t("calendar.allKinds")}
        </button>
        {KINDS.map((kind) => (
          <button
            key={kind}
            type="button"
            onClick={() => toggleKind(kind)}
            aria-pressed={kinds.includes(kind)}
            className={[
              "inline-flex items-center gap-1.5 rounded-full px-3 py-1 text-xs font-medium transition-colors",
              kinds.includes(kind)
                ? `fc-chip-${kind}`
                : "bg-sunken text-ink-soft hover:bg-line",
            ].join(" ")}
          >
            <span className={`fc-dot-${kind} size-1.5 rounded-full`} aria-hidden="true" />
            {t(`lessons.kind_${kind}`)}
          </button>
        ))}
      </div>

      {error && <ErrorNote>{error}</ErrorNote>}

      <div className="tariqi-calendar rounded-lg border border-line bg-surface p-3 shadow-card">
        <FullCalendar
          ref={calendar}
          plugins={[timeGridPlugin, dayGridPlugin, interactionPlugin]}
          initialView={view === "week" ? "timeGridWeek" : "dayGridMonth"}
          initialDate={date}
          locale={locale === "ar" ? arLocale : frLocale}
          direction={locale === "ar" ? "rtl" : "ltr"}
          /* Algerian auto-écoles work Saturday to Thursday and rest on Friday,
             so the week opens on Saturday. */
          firstDay={6}
          height="auto"
          slotMinTime={`${String(GRID_START_HOUR).padStart(2, "0")}:00:00`}
          slotMaxTime={`${String(GRID_END_HOUR).padStart(2, "0")}:00:00`}
          slotDuration="00:30:00"
          /* FullCalendar's default for a time grid is to let overlapping
             events COVER one another, which reads as a pile of cards. Side
             by side instead: narrower, but every block's own edges are
             visible and nothing hides behind anything. */
          slotEventOverlap={false}
          slotLabelInterval="01:00"
          nowIndicator
          /* ONE session across the column, the rest behind a "+N" link.
             A day column is about 100px: split even two ways, neither name
             fits, which is the same failure the hand-rolled grid had. The
             week is for seeing how full a day is — the day view is where you
             read every name. */
          eventMaxStack={1}
          dayMaxEvents={4}
          expandRows
          allDaySlot
          allDayText={t("calendar.noTime")}
          headerToolbar={false}
          /* FullCalendar renders its day heading and its "+N more" badge as
             plain text, which the bidi algorithm then reorders in Arabic:
             "17/10" came out "10/17" and "+1" came out "1+". Both are number
             runs, so both are rendered here inside an isolate. Same rule as
             <Num> everywhere else — see CLAUDE.md. */
          dayHeaderContent={(arg) => {
            const weekday = new Intl.DateTimeFormat(
              locale === "ar" ? "ar-DZ" : "fr-DZ",
              { weekday: "short" },
            ).format(arg.date);
            // In a month view each column heading is a weekday, not a date:
            // the date handed in is only a reference day for that column, so
            // printing it gave "sam. 10/01" in a week of October.
            if (arg.view.type === "dayGridMonth") return <span>{weekday}</span>;
            return (
              <span className="fc-tariqi-dayhead">
                <span>{weekday}</span>
                <span className="num">
                  {String(arg.date.getDate()).padStart(2, "0")}/
                  {String(arg.date.getMonth() + 1).padStart(2, "0")}
                </span>
              </span>
            );
          }}
          moreLinkContent={(arg) => <span className="num">+{arg.num}</span>}
          events={events}
          /* Start time only. The block's height already says when it ends,
             and "08:00 - 09:00" costs most of the width a name needs. */
          displayEventEnd={false}
          eventTimeFormat={{ hour: "2-digit", minute: "2-digit", hour12: false }}
          slotLabelFormat={{ hour: "2-digit", minute: "2-digit", hour12: false }}
          datesSet={(arg) => {
            const from = localParts(arg.start).day;
            // FullCalendar's end is exclusive; the query's is inclusive.
            const last = new Date(arg.end.getTime() - 86_400_000);
            setRange({ from, to: localParts(last).day });
            const shown = localParts(arg.view.currentStart).day;
            if (shown !== date) setDate(shown);
          }}
          eventContent={(arg) => {
            const names = (arg.event.extendedProps["names"] as string[]) ?? [
              arg.event.title,
            ];
            return (
              <div className="fc-tariqi-event">
                <span className="fc-tariqi-time">{arg.timeText}</span>
                {names.map((n, i) => (
                  <span key={i} className="fc-tariqi-title">
                    {n}
                  </span>
                ))}
                {arg.event.extendedProps["planNote"] ? (
                  <span className="fc-tariqi-note" dir="auto">
                    {String(arg.event.extendedProps["planNote"])}
                  </span>
                ) : null}
              </div>
            );
          }}
          eventClick={(arg) => {
            const start = arg.event.start;
            if (start) openDay(localParts(start).day);
          }}
          /* Clicking an empty slot books it. The week grid knows the hour
              that was clicked; the month only knows the day, so the dialog
              keeps its own default time there. This is the whole point of
              looking at the week — the gap you can see is the gap you fill. */
          dateClick={(arg) => {
            const { day, time } = localParts(arg.date);
            onPickSlot(day, arg.allDay ? null : time);
          }}
          eventDrop={(arg: EventDropArg) =>
            void move(arg.event.id, arg.event.start, arg.event.end, arg.revert)
          }
          eventResize={(arg: EventResizeDoneArg) =>
            void move(arg.event.id, arg.event.start, arg.event.end, arg.revert)
          }
        />
      </div>
    </div>
  );
}
