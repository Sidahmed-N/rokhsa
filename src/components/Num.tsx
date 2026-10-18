import type { ReactNode } from "react";

import { currentLanguage } from "@/i18n";
import { formatDZD, formatDinars, type Centimes } from "@/lib/money";

/**
 * Bidirectional isolation for numbers and Latin text.
 *
 * WHY THIS EXISTS — do not replace these with bare strings.
 *
 * When a value containing spaces sits in Arabic (RTL) text, the Unicode
 * bidirectional algorithm reorders its space-separated runs. A national ID
 * number stored as "109 2004 16 0054821" renders on screen and on paper as
 * "0054821 16 2004 109". It is not a font problem and it is not visible to
 * anyone reading the French version, which is what makes it dangerous: it
 * silently prints a wrong identity number on an official receipt.
 *
 * `dir="ltr"` plus `unicode-bidi: isolate` pins the run so it reads correctly
 * in both languages.
 *
 * This is a COMPONENT rather than a CSS class on purpose. A class has to be
 * remembered at every call site; the first draft of the receipt used one and
 * two of its three money rows were already wrong. A component that formats and
 * isolates in one step cannot be got wrong.
 */
export function Num({
  children,
  className,
}: {
  children: ReactNode;
  // `| undefined` is required by exactOptionalPropertyTypes in tsconfig.
  className?: string | undefined;
}) {
  return (
    <span dir="ltr" className={["num", className].filter(Boolean).join(" ")}>
      {children}
    </span>
  );
}

/**
 * A money amount, formatted and isolated.
 *
 * Always prefer this to calling formatDZD() directly in JSX — see above.
 */
export function Money({
  centimes,
  whole = false,
  withSuffix = true,
  className,
}: {
  centimes: Centimes;
  /** Show whole dinars only, for compact list rows. */
  whole?: boolean | undefined;
  withSuffix?: boolean | undefined;
  className?: string | undefined;
}) {
  const locale = currentLanguage();
  const text = whole
    ? formatDinars(centimes, { locale, withSuffix })
    : formatDZD(centimes, { locale, withSuffix });

  return <Num className={className}>{text}</Num>;
}
