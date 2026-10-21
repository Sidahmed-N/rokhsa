import type { DayEntry } from "@/lib/training";

/**
 * The candidate's name in the reader's script, falling back to the other.
 *
 * Both forms are stored and neither is derived from the other, so a school
 * that only ever typed the Latin name still sees something in Arabic.
 */
export function candidateName(row: DayEntry, locale: "fr" | "ar"): string {
  const arabic = `${row.lastNameAr} ${row.firstNameAr}`.trim();
  const latin = `${row.lastNameFr} ${row.firstNameFr}`.trim();
  return locale === "ar" ? arabic || latin : latin || arabic;
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
