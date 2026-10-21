/**
 * A person's name in the reader's script.
 *
 * Both forms are stored and neither is derived from the other, so a school
 * that only ever typed the Latin name still sees something in Arabic, and
 * one that only typed Arabic still sees something in French.
 *
 * Takes the four fields rather than a specific row type, because candidates,
 * day entries and receivable rows all carry the same four and none of them
 * should need converting to another shape first.
 */
export type NameFields = {
  lastNameFr: string;
  firstNameFr: string;
  lastNameAr: string;
  firstNameAr: string;
};

export function personName(row: NameFields, locale: "fr" | "ar"): string {
  const arabic = `${row.lastNameAr} ${row.firstNameAr}`.trim();
  const latin = `${row.lastNameFr} ${row.firstNameFr}`.trim();
  return locale === "ar" ? arabic || latin : latin || arabic;
}
