import { personName } from "@/lib/names";
import type { DayEntry } from "@/lib/training";

/** The candidate's name in the reader's script. */
export function candidateName(row: DayEntry, locale: "fr" | "ar"): string {
  return personName(row, locale);
}

function asDate(iso: string): Date {
  const [y, m, d] = iso.split("-").map(Number);
  return new Date(y ?? 1970, (m ?? 1) - 1, d ?? 1);
}

function intl(locale: "fr" | "ar", options: Intl.DateTimeFormatOptions) {
  return new Intl.DateTimeFormat(locale === "ar" ? "ar-DZ" : "fr-DZ", options);
}

/** Day and month for a range heading, e.g. "sam. 25 oct." */
export function formatDayLabel(iso: string, locale: "fr" | "ar"): string {
  return intl(locale, { weekday: "short", day: "numeric", month: "short" }).format(asDate(iso));
}

/** Just the weekday, for the week view's date rail. */
export function formatWeekday(iso: string, locale: "fr" | "ar"): string {
  return intl(locale, { weekday: "short" }).format(asDate(iso));
}

/** Just the day of the month, sitting under the weekday in the rail. */
export function formatDayNumber(iso: string, locale: "fr" | "ar"): string {
  return intl(locale, { day: "numeric" }).format(asDate(iso));
}
