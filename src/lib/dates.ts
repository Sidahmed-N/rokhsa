/**
 * A stored day ("2026-11-05") the way a school reads it: "05/11/2026".
 *
 * Days are stored ISO so they sort and compare as text, and the screens used
 * to print them as stored. An Algerian secretary reads jj/mm/aaaa, the date
 * boxes already show it, and the ISO form reads especially badly in Arabic.
 * Anything that is not a plain day is returned unchanged.
 *
 * Display only: never parse this back, never store it.
 */
export function formatDay(iso: string): string {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(iso.trim());
  return match ? `${match[3]}/${match[2]}/${match[1]}` : iso;
}
