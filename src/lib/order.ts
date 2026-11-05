/**
 * Keep rows where they were when a list is reloaded after a click.
 *
 * The day's sessions come "still to do first", which is right when the page
 * opens. But reloading after each Fait / Absent re-sorted them: the marked
 * row dropped to the bottom and every row below moved up under the pointer,
 * so the next click, aimed at one candidate's "Absent", landed on the next
 * candidate's. Rows already on screen keep their place; rows that are new
 * (booked meanwhile) go after them, in the order they arrived.
 *
 * Display order only — nothing about what a row says comes from here.
 */
export function keepOrder<T extends { id: number }>(previous: T[], next: T[]): T[] {
  if (previous.length === 0) return next;
  const place = new Map(previous.map((row, i) => [row.id, i]));
  const kept = next.filter((row) => place.has(row.id));
  const added = next.filter((row) => !place.has(row.id));
  kept.sort((a, b) => (place.get(a.id) ?? 0) - (place.get(b.id) ?? 0));
  return [...kept, ...added];
}
