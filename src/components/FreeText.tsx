import type { ReactNode } from "react";

/**
 * Text the school typed, in whichever language they typed it.
 *
 * A plan note, a lesson note, an expense description: the app cannot know
 * whether a secretary wrote "créneau puis circulation" or
 * "المناورة ثم السياقة", and the two want opposite directions. `dir="auto"`
 * lets the browser decide from the first strong character in the string.
 *
 * WHY THIS MATTERS, beyond alignment: with `truncate`, the ellipsis goes at
 * the END of the text *in that text's own direction*. A French note sitting
 * in an Arabic page without `dir="auto"` is laid out right-to-left, so the
 * clipping happens at its beginning and the week grid showed
 * "...ation en ville, giratoires" — the half the instructor needs is the
 * half that gets cut. It reads perfectly in French and is wrong only in
 * Arabic, which is the signature of every bidi bug in this project.
 *
 * Do not add a margin or padding utility to this component's own element:
 * logical spacing resolves against the element's own direction, and this
 * element's direction is whatever the user typed. Put spacing outside.
 */
export default function FreeText({
  children,
  className,
  as = "p",
}: {
  children: ReactNode;
  className?: string | undefined;
  as?: "p" | "span" | undefined;
}) {
  const Tag = as;
  return (
    <Tag dir="auto" className={className}>
      {children}
    </Tag>
  );
}
