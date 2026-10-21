/**
 * The colour language for session kinds.
 *
 * One place, so a Circulation block is the same blue on the week grid, the
 * day list, the candidate's file and the filter chips. An instructor should
 * be able to read the week without reading any words.
 */
export const KINDS = ["theory", "maneuvers", "road"] as const;
export type Kind = (typeof KINDS)[number];

type KindStyle = {
  /** Filled block, as used on the week grid. */
  block: string;
  /** Small chip, as used in a list row. */
  chip: string;
  /** The dot on a filter button. */
  dot: string;
};

const STYLES: Record<Kind, KindStyle> = {
  theory: {
    block: "bg-theory text-theory-ink",
    chip: "bg-theory text-theory-ink",
    dot: "bg-theory-ink",
  },
  maneuvers: {
    block: "bg-maneuvers text-maneuvers-ink",
    chip: "bg-maneuvers text-maneuvers-ink",
    dot: "bg-maneuvers-ink",
  },
  road: {
    block: "bg-road text-road-ink",
    chip: "bg-road text-road-ink",
    dot: "bg-road-ink",
  },
};

/** Falls back to the neutral surface for a kind we do not know about. */
export function kindStyle(kind: string): KindStyle {
  return STYLES[kind as Kind] ?? { block: "bg-sunken text-ink", chip: "bg-sunken text-ink", dot: "bg-faint" };
}
