/**
 * A person's name, as the school typed it.
 *
 * One name per candidate, in whichever script the secretary used: a French
 * page can show "مرابط بلال" and an Arabic page "MERABET Bilal". Nothing is
 * transliterated — a computer cannot turn a name into the other script
 * reliably, and a wrong spelling on a candidate's own name is worse than the
 * other script. See migration 0010.
 *
 * Because the script is the school's choice, not the page's, a TRUNCATED name
 * needs an inner `dir="auto"` inline-block inside an element that keeps the
 * page's direction (see ProgressBoard): the outer one puts the name on the
 * page's side, the inner one clips it at its own end.
 *
 * Takes the two fields rather than a specific row type, because candidates,
 * day entries and receivable rows all carry them and none of them should need
 * converting to another shape first.
 */
export type NameFields = {
  lastName: string;
  firstName: string;
};

export function personName(row: NameFields): string {
  return `${row.lastName} ${row.firstName}`.trim();
}
