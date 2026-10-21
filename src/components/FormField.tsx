import type { ReactNode } from "react";
import { ChevronDown } from "lucide-react";

/**
 * Form controls.
 *
 * One input style for the whole app, including the same focus treatment, so
 * that a form reads as a single object rather than a stack of boxes. The
 * focus state moves the border to the accent colour and adds a soft ring —
 * visible across the room on a cheap office monitor, which is the actual
 * test.
 */
const INPUT_CLASS = [
  "w-full rounded-md border bg-surface px-3 py-2 text-sm text-ink",
  "outline-none transition-colors placeholder:text-faint",
  "focus:border-brand focus:ring-2 focus:ring-brand-soft",
].join(" ");

function Shell({
  label,
  hint,
  error,
  children,
}: {
  label: string;
  hint?: string | undefined;
  error?: string | undefined;
  children: ReactNode;
}) {
  return (
    <label className="block">
      <span className="mb-1.5 block text-xs font-medium text-ink-soft">{label}</span>
      {children}
      {hint && !error && <span className="mt-1 block text-xs text-muted">{hint}</span>}
      {error && <span className="mt-1 block text-xs font-medium text-owed">{error}</span>}
    </label>
  );
}

export function TextField({
  label,
  value,
  onChange,
  error,
  type = "text",
  dir,
  placeholder,
  hint,
}: {
  label: string;
  value: string;
  onChange: (next: string) => void;
  error?: string | undefined;
  type?: string;
  dir?: "ltr" | "rtl" | undefined;
  placeholder?: string | undefined;
  hint?: string | undefined;
}) {
  return (
    <Shell label={label} hint={hint} error={error}>
      <input
        type={type}
        value={value}
        dir={dir}
        placeholder={placeholder}
        onChange={(e) => onChange(e.target.value)}
        className={[INPUT_CLASS, error ? "border-owed" : "border-line-strong"].join(" ")}
      />
    </Shell>
  );
}

export function SelectField({
  label,
  value,
  onChange,
  children,
  error,
}: {
  label: string;
  value: string;
  onChange: (next: string) => void;
  children: ReactNode;
  error?: string | undefined;
}) {
  return (
    <Shell label={label} error={error}>
      <div className="relative">
        <select
          value={value}
          onChange={(e) => onChange(e.target.value)}
          className={[
            INPUT_CLASS,
            "appearance-none pe-9",
            error ? "border-owed" : "border-line-strong",
          ].join(" ")}
        >
          {children}
        </select>
        {/* The native arrow is hidden above so the control matches the text
            fields; this one sits at the inline end and mirrors for Arabic. */}
        <ChevronDown
          className="pointer-events-none absolute end-3 top-1/2 size-4 -translate-y-1/2 text-faint"
          aria-hidden="true"
        />
      </div>
    </Shell>
  );
}
