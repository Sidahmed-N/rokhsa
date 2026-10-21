import { useRef } from "react";
import { CalendarDays } from "lucide-react";

import { currentLanguage } from "@/i18n";

/**
 * A date field that reads in the user's language.
 *
 * `<input type="date">` renders its text in the *operating system's* locale,
 * not the page's. On a Windows machine set up in English — which is most of
 * them, because that is how they arrive — the calendar showed "10/20/2026"
 * inside a French interface, and inside an Arabic one. The owner reads the
 * month as the day.
 *
 * There is no way to restyle the native text, so the native control is kept
 * for what it is genuinely good at (a real calendar popup, arrow keys, and
 * the OS date picker) and laid transparently over our own label, which is
 * formatted with Intl in the app's language. Clicking anywhere on the field
 * opens the picker; tabbing to it and typing still works, because the real
 * input is the thing receiving focus.
 */
export default function DateField({
  value,
  onChange,
  className,
  "aria-label": ariaLabel,
}: {
  /** ISO YYYY-MM-DD. */
  value: string;
  onChange: (iso: string) => void;
  className?: string | undefined;
  "aria-label"?: string | undefined;
}) {
  const input = useRef<HTMLInputElement>(null);
  const locale = currentLanguage();

  const [y, m, d] = value.split("-").map(Number);
  const label = Number.isFinite(y)
    ? new Intl.DateTimeFormat(locale === "ar" ? "ar-DZ" : "fr-DZ", {
        weekday: "short",
        day: "numeric",
        month: "long",
        year: "numeric",
      }).format(new Date(y ?? 1970, (m ?? 1) - 1, d ?? 1))
    : value;

  return (
    <div
      className={[
        "relative inline-flex h-9 items-center gap-2 rounded-md border border-line-strong",
        "bg-surface px-3 text-sm shadow-card focus-within:border-brand",
        className ?? "",
      ]
        .filter(Boolean)
        .join(" ")}
    >
      <CalendarDays className="size-4 shrink-0 text-faint" aria-hidden="true" />
      {/* The label the user reads. `num` keeps the digits from being reordered
          by the bidi algorithm when the month name is Arabic. */}
      <span className="num whitespace-nowrap font-medium text-ink">{label}</span>

      <input
        ref={input}
        type="date"
        value={value}
        aria-label={ariaLabel ?? ""}
        onChange={(e) => onChange(e.target.value)}
        onClick={() => input.current?.showPicker?.()}
        /* Transparent, but still the real focusable control, so keyboard and
           screen readers get the native behaviour. */
        className="absolute inset-0 size-full cursor-pointer opacity-0"
      />
    </div>
  );
}
