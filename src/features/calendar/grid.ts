import type { DayEntry } from "@/lib/training";

/**
 * Laying sessions out on a time grid.
 *
 * Kept as pure functions so the arithmetic — which is where a calendar grid
 * actually goes wrong — can be reasoned about without a browser.
 */

/** The grid runs 07:00–20:00: an auto-école's day with an hour either side. */
export const GRID_START_HOUR = 7;
export const GRID_END_HOUR = 20;
export const HOUR_HEIGHT = 52;

export const GRID_HOURS = Array.from(
  { length: GRID_END_HOUR - GRID_START_HOUR + 1 },
  (_, i) => GRID_START_HOUR + i,
);

export const GRID_HEIGHT = (GRID_END_HOUR - GRID_START_HOUR) * HOUR_HEIGHT;

/** "HH:MM" to minutes since midnight, or null if it is not a time. */
export function minutesOfDay(time: string | null): number | null {
  if (!time) return null;
  const [h, m] = time.split(":").map(Number);
  if (!Number.isFinite(h) || !Number.isFinite(m)) return null;
  return (h ?? 0) * 60 + (m ?? 0);
}

export type Placed = {
  entry: DayEntry;
  /** Pixels from the top of the grid. */
  top: number;
  height: number;
  /** Fraction of the column width, for sessions that overlap. */
  widthPercent: number;
  leftPercent: number;
};

/**
 * Place one day's timed sessions, splitting the width where they overlap.
 *
 * Two candidates booked at the same hour is normal — one instructor may be
 * out on the road while another takes a class — so overlapping blocks sit
 * side by side rather than hiding each other.
 */
export function placeDay(entries: DayEntry[]): Placed[] {
  const timed = entries
    .map((entry) => ({ entry, start: minutesOfDay(entry.scheduledTime) }))
    .filter((x): x is { entry: DayEntry; start: number } => x.start !== null)
    .sort((a, b) => a.start - b.start);

  // Group into runs of sessions that overlap each other, so a width is only
  // ever split between blocks that actually collide.
  const groups: { entry: DayEntry; start: number; end: number }[][] = [];
  let current: { entry: DayEntry; start: number; end: number }[] = [];
  let groupEnd = -1;

  for (const { entry, start } of timed) {
    const end = start + Math.max(entry.minutes, 15);
    if (current.length > 0 && start < groupEnd) {
      current.push({ entry, start, end });
      groupEnd = Math.max(groupEnd, end);
    } else {
      if (current.length > 0) groups.push(current);
      current = [{ entry, start, end }];
      groupEnd = end;
    }
  }
  if (current.length > 0) groups.push(current);

  const placed: Placed[] = [];
  const gridStart = GRID_START_HOUR * 60;

  for (const group of groups) {
    const share = 100 / group.length;
    group.forEach((item, index) => {
      // A very short session still needs to be readable, and one booked
      // outside the grid's hours is pinned to its edge rather than dropped.
      const top = ((item.start - gridStart) / 60) * HOUR_HEIGHT;
      const height = Math.max((item.entry.minutes / 60) * HOUR_HEIGHT, 26);
      placed.push({
        entry: item.entry,
        top: Math.max(0, Math.min(top, GRID_HEIGHT - 26)),
        height: Math.min(height, GRID_HEIGHT),
        widthPercent: share,
        leftPercent: share * index,
      });
    });
  }

  return placed;
}

/** Sessions with no time booked. They cannot sit on the grid. */
export function untimed(entries: DayEntry[]): DayEntry[] {
  return entries.filter((e) => minutesOfDay(e.scheduledTime) === null);
}

/**
 * Where "now" sits on the grid, or null when it is outside the visible hours.
 * Drives the current-time line, which is what makes the grid feel live.
 */
export function nowOffset(now: Date): number | null {
  const minutes = now.getHours() * 60 + now.getMinutes();
  const gridStart = GRID_START_HOUR * 60;
  const gridEnd = GRID_END_HOUR * 60;
  if (minutes < gridStart || minutes > gridEnd) return null;
  return ((minutes - gridStart) / 60) * HOUR_HEIGHT;
}
