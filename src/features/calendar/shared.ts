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

/** Day and month for a column heading, e.g. "sam. 25 oct." */
export function formatDayLabel(iso: string, locale: "fr" | "ar"): string {
  const [y, m, d] = iso.split("-").map(Number);
  const date = new Date(y ?? 1970, (m ?? 1) - 1, d ?? 1);
  return new Intl.DateTimeFormat(locale === "ar" ? "ar-DZ" : "fr-DZ", {
    weekday: "short",
    day: "numeric",
    month: "short",
  }).format(date);
}
