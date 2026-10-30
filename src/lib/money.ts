/**
 * Money handling.
 *
 * HARD RULE (see CLAUDE.md): money is stored and passed around as integer
 * centimes of DZD. Never a float. 1 DA = 100 centimes.
 *
 * This module is formatting and parsing only. Arithmetic that decides what a
 * candidate owes lives in the Rust domain layer, never here.
 */

/** Integer centimes of Algerian dinar. Always an integer; never a float. */
export type Centimes = number;

export const CENTIMES_PER_DINAR = 100;

/** Guard for values crossing a boundary (form input, API result). */
export function isCentimes(value: unknown): value is Centimes {
  return typeof value === "number" && Number.isSafeInteger(value);
}

export function assertCentimes(value: unknown, label = "amount"): Centimes {
  if (!isCentimes(value)) {
    throw new Error(`${label} must be an integer number of centimes, got: ${String(value)}`);
  }
  return value;
}

/**
 * Format centimes for display, e.g. 1250000 -> "12 500,00 DA".
 *
 * Uses a non-breaking thin space as the thousands separator and a comma as the
 * decimal mark, which is the French/Algerian convention. Digits stay Western
 * even in Arabic — Algerian paperwork uses Western digits for money.
 */
export function formatDZD(
  centimes: Centimes,
  options: { withSuffix?: boolean; locale?: "fr" | "ar" } = {},
): string {
  const { withSuffix = true, locale = "fr" } = options;

  const negative = centimes < 0;
  const abs = Math.abs(centimes);
  const dinars = Math.trunc(abs / CENTIMES_PER_DINAR);
  const cents = abs % CENTIMES_PER_DINAR;

  const grouped = String(dinars).replace(/\B(?=(\d{3})+(?!\d))/g, " ");
  const body = `${grouped},${String(cents).padStart(2, "0")}`;
  const signed = negative ? `-${body}` : body;

  if (!withSuffix) return signed;
  return `${signed} ${locale === "ar" ? "دج" : "DA"}`;
}

/** Whole dinars only, for compact places like list rows: 1250000 -> "12 500 DA". */
export function formatDinars(
  centimes: Centimes,
  options: { withSuffix?: boolean; locale?: "fr" | "ar" } = {},
): string {
  const { withSuffix = true, locale = "fr" } = options;
  const negative = centimes < 0;
  const dinars = Math.trunc(Math.abs(centimes) / CENTIMES_PER_DINAR);
  const grouped = String(dinars).replace(/\B(?=(\d{3})+(?!\d))/g, " ");
  const signed = negative ? `-${grouped}` : grouped;
  if (!withSuffix) return signed;
  return `${signed} ${locale === "ar" ? "دج" : "DA"}`;
}

/**
 * Parse what a secretary typed into centimes, or null if it is not clearly
 * an amount — callers then show a validation message instead of saving.
 *
 * A dot or comma followed by exactly three digits is a THOUSANDS separator,
 * because that is how amounts are written here: "12.000" and "12,000" are
 * twelve thousand dinars. The first version read every lone separator as a
 * decimal point and stored a 12 000 DA payment as 12 DA. With one or two
 * digits after it, the separator is the decimal point ("12500,50").
 *
 *   "12500"  "12 500"  "12.000"  "12,000"  "1.500.000"  -> whole dinars
 *   "12500,50"  "12500.5"  "12.500,50"  "12,500.50"     -> with centimes
 *   "12.5000"  "1.50.000"  "12,345,6"  "abc"             -> null
 *
 * Integer arithmetic on the digit strings, never `Number(x) * 100` on a
 * fraction: money is never a float here (CLAUDE.md). Arabic-Indic digits are
 * accepted, since an Arabic keyboard types them. Tested by
 * scripts/test-money.mjs, which `npm run check` runs.
 */
export function parseDZD(input: string): Centimes | null {
  let s = input
    .replace(/[٠-٩]/g, (d) => String(d.charCodeAt(0) - 0x0660))
    .replace(/[\s  ]/g, "")
    .replace(/[،٫]/g, ",")
    // A currency word typed after the figure ("12 000 DA", "12000 دج").
    .replace(/(da|dzd|dinars?|دج|دينار)$/i, "");

  let negative = false;
  if (s.startsWith("-")) {
    negative = true;
    s = s.slice(1);
  }
  if (!/^\d[\d.,]*$/.test(s) || /[.,]$/.test(s)) return null;

  let whole = s;
  let fraction = "";
  const lastSep = Math.max(s.lastIndexOf("."), s.lastIndexOf(","));

  if (lastSep >= 0) {
    const sepChar = s.charAt(lastSep);
    const tail = s.slice(lastSep + 1);
    const mixed = s.includes(".") && s.includes(",");
    const separators = s.match(/[.,]/g) ?? [];

    // Which separator, if any, is the decimal point.
    let decimalAt = -1;
    if (mixed) {
      // "12.500,50": the last kind is the decimal point, and every earlier
      // separator must be the other kind.
      if (s.slice(0, lastSep).includes(sepChar)) return null;
      decimalAt = lastSep;
    } else if (separators.length === 1 && tail.length !== 3) {
      decimalAt = lastSep;
    }

    if (decimalAt >= 0) {
      whole = s.slice(0, decimalAt);
      fraction = s.slice(decimalAt + 1);
      if (fraction.length < 1 || fraction.length > 2) return null;
    }

    // The separators left are thousands: a first group of one to three
    // digits, then groups of exactly three.
    const groups = whole.split(/[.,]/);
    if (groups.length > 1) {
      const [first, ...rest] = groups;
      if (!first || first.length > 3 || rest.some((g) => g.length !== 3)) return null;
    }
    whole = groups.join("");
  }

  if (!/^\d+$/.test(whole) || (fraction !== "" && !/^\d+$/.test(fraction))) return null;

  const centimes = Number(whole) * CENTIMES_PER_DINAR + Number(fraction.padEnd(2, "0") || "0");
  if (!Number.isSafeInteger(centimes)) return null;
  return negative ? -centimes : centimes;
}

/* ---------------------------------------------------------------------------
   Amount in words
   ---------------------------------------------------------------------------
   Algerian receipts conventionally state the amount in words as well as in
   figures ("arretee la presente quittance a la somme de ..."). This is
   presentation, not a business rule, so it belongs in the frontend.
--------------------------------------------------------------------------- */

const FR_UNITS = [
  "zéro", "un", "deux", "trois", "quatre", "cinq", "six", "sept", "huit", "neuf",
  "dix", "onze", "douze", "treize", "quatorze", "quinze", "seize",
  "dix-sept", "dix-huit", "dix-neuf",
];
const FR_TENS = [
  "", "", "vingt", "trente", "quarante", "cinquante",
  "soixante", "soixante", "quatre-vingt", "quatre-vingt",
];

function frBelowHundred(n: number): string {
  if (n < 20) return FR_UNITS[n] as string;
  const tensDigit = Math.floor(n / 10);
  const unit = n % 10;
  // French 70-79 and 90-99 build on soixante / quatre-vingt.
  const base = FR_TENS[tensDigit] as string;
  const offset = tensDigit === 7 || tensDigit === 9 ? unit + 10 : unit;

  if (offset === 0) return tensDigit === 8 ? `${base}s` : base;
  if (offset === 1 && (tensDigit === 2 || tensDigit === 3 || tensDigit === 4 || tensDigit === 5 || tensDigit === 6)) {
    return `${base}-et-un`;
  }
  return `${base}-${FR_UNITS[offset] as string}`;
}

function frBelowThousand(n: number): string {
  if (n < 100) return frBelowHundred(n);
  const hundreds = Math.floor(n / 100);
  const rest = n % 100;
  const head = hundreds === 1 ? "cent" : `${FR_UNITS[hundreds] as string} cent`;
  const headPlural = rest === 0 && hundreds > 1 ? `${head}s` : head;
  return rest === 0 ? headPlural : `${headPlural} ${frBelowHundred(rest)}`;
}

/** French words for a whole number of dinars. */
export function dinarsToWordsFr(dinars: number): string {
  if (dinars === 0) return FR_UNITS[0] as string;
  if (dinars < 0) return `moins ${dinarsToWordsFr(-dinars)}`;

  const chunks: string[] = [];
  const millions = Math.floor(dinars / 1_000_000);
  const thousands = Math.floor((dinars % 1_000_000) / 1000);
  const rest = dinars % 1000;

  if (millions > 0) {
    chunks.push(millions === 1 ? "un million" : `${frBelowThousand(millions)} millions`);
  }
  if (thousands > 0) {
    chunks.push(thousands === 1 ? "mille" : `${frBelowThousand(thousands)} mille`);
  }
  if (rest > 0) chunks.push(frBelowThousand(rest));

  return chunks.join(" ");
}

const AR_UNITS = [
  "صفر", "واحد", "اثنان", "ثلاثة", "أربعة", "خمسة", "ستة", "سبعة", "ثمانية", "تسعة",
  "عشرة", "أحد عشر", "اثنا عشر", "ثلاثة عشر", "أربعة عشر", "خمسة عشر",
  "ستة عشر", "سبعة عشر", "ثمانية عشر", "تسعة عشر",
];
const AR_TENS = ["", "", "عشرون", "ثلاثون", "أربعون", "خمسون", "ستون", "سبعون", "ثمانون", "تسعون"];
const AR_HUNDREDS = ["", "مئة", "مئتان", "ثلاث مئة", "أربع مئة", "خمس مئة", "ست مئة", "سبع مئة", "ثمان مئة", "تسع مئة"];

function arBelowHundred(n: number): string {
  if (n < 20) return AR_UNITS[n] as string;
  const tensDigit = Math.floor(n / 10);
  const unit = n % 10;
  if (unit === 0) return AR_TENS[tensDigit] as string;
  // Arabic puts the unit before the ten: "واحد وعشرون".
  return `${AR_UNITS[unit] as string} و${AR_TENS[tensDigit] as string}`;
}

function arBelowThousand(n: number): string {
  if (n < 100) return arBelowHundred(n);
  const hundreds = Math.floor(n / 100);
  const rest = n % 100;
  const head = AR_HUNDREDS[hundreds] as string;
  return rest === 0 ? head : `${head} و${arBelowHundred(rest)}`;
}

/**
 * Arabic words for a whole number of dinars.
 *
 * Simplified: it does not implement full case endings or every dual/plural
 * agreement rule of formal Arabic. It is readable and unambiguous on a
 * receipt, which is what the document needs.
 * TODO: have a native Arabic speaker review the output before release.
 */
export function dinarsToWordsAr(dinars: number): string {
  if (dinars === 0) return AR_UNITS[0] as string;
  if (dinars < 0) return `سالب ${dinarsToWordsAr(-dinars)}`;

  const chunks: string[] = [];
  const millions = Math.floor(dinars / 1_000_000);
  const thousands = Math.floor((dinars % 1_000_000) / 1000);
  const rest = dinars % 1000;

  if (millions > 0) {
    if (millions === 1) chunks.push("مليون");
    else if (millions === 2) chunks.push("مليونان");
    else chunks.push(`${arBelowThousand(millions)} مليون`);
  }
  if (thousands > 0) {
    if (thousands === 1) chunks.push("ألف");
    else if (thousands === 2) chunks.push("ألفان");
    else chunks.push(`${arBelowThousand(thousands)} ألف`);
  }
  if (rest > 0) chunks.push(arBelowThousand(rest));

  return chunks.join(" و");
}

/** Amount in words for a receipt, in the given language. */
export function amountInWords(centimes: Centimes, locale: "fr" | "ar"): string {
  const dinars = Math.trunc(Math.abs(centimes) / CENTIMES_PER_DINAR);
  const cents = Math.abs(centimes) % CENTIMES_PER_DINAR;

  if (locale === "ar") {
    const head = `${dinarsToWordsAr(dinars)} دينار جزائري`;
    return cents === 0 ? head : `${head} و${dinarsToWordsAr(cents)} سنتيم`;
  }

  const head = `${dinarsToWordsFr(dinars)} dinars algériens`;
  return cents === 0 ? head : `${head} et ${dinarsToWordsFr(cents)} centimes`;
}
