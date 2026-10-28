import { useEffect, useState } from "react";

import { todayIso } from "@/lib/training";

/**
 * Today's date, kept true while the app stays open.
 *
 * A driving school does not close this app. It is the till: it goes on at
 * eight and stays on. Every screen that asked `todayIso()` once at mount was
 * therefore pinned to whichever day the app was started, and the longer it
 * ran the wronger it got — the calendar opened on yesterday, and the overdue
 * card asked "what was unmarked before yesterday", so a session forgotten
 * yesterday did not appear on the morning it was meant to be caught.
 *
 * Two triggers, because neither alone is enough:
 *
 *   * a timer aimed at the next local midnight, re-armed each time. Exact,
 *     and it costs nothing between firings — no polling.
 *   * `visibilitychange` and `focus`, because a timer does not fire reliably
 *     across a machine that was asleep, and a PC in an auto-école is shut or
 *     suspended most nights. Coming back to the window re-checks the clock.
 *
 * Both paths only call `setState` when the string actually differs, so a
 * focus event on the same day re-renders nothing.
 */
export function useToday(): string {
  const [today, setToday] = useState(todayIso);

  useEffect(() => {
    let timer: number | undefined;

    const check = () => {
      const now = todayIso();
      setToday((previous) => (previous === now ? previous : now));
    };

    const arm = () => {
      const now = new Date();
      const midnight = new Date(
        now.getFullYear(),
        now.getMonth(),
        now.getDate() + 1,
        0,
        0,
        // A second past midnight, so the timer cannot land on 23:59:59.999
        // and read the old day.
        1,
      );
      timer = window.setTimeout(() => {
        check();
        arm();
      }, midnight.getTime() - now.getTime());
    };

    arm();
    window.addEventListener("visibilitychange", check);
    window.addEventListener("focus", check);

    return () => {
      if (timer !== undefined) window.clearTimeout(timer);
      window.removeEventListener("visibilitychange", check);
      window.removeEventListener("focus", check);
    };
  }, []);

  return today;
}
