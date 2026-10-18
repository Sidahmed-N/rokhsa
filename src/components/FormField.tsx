import type { ReactNode } from "react";

const INPUT_CLASS =
  "w-full rounded-md border bg-white px-3 py-2 text-sm outline-none focus:border-brand focus:ring-2 focus:ring-brand-soft";

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
    <label className="block">
      <span className="mb-1 block text-xs text-muted">{label}</span>
      <input
        type={type}
        value={value}
        dir={dir}
        placeholder={placeholder}
        onChange={(e) => onChange(e.target.value)}
        className={[INPUT_CLASS, error ? "border-owed" : "border-line"].join(" ")}
      />
      {hint && !error && <span className="mt-1 block text-xs text-muted">{hint}</span>}
      {error && <span className="mt-1 block text-xs text-owed">{error}</span>}
    </label>
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
    <label className="block">
      <span className="mb-1 block text-xs text-muted">{label}</span>
      <select
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className={[INPUT_CLASS, error ? "border-owed" : "border-line"].join(" ")}
      >
        {children}
      </select>
      {error && <span className="mt-1 block text-xs text-owed">{error}</span>}
    </label>
  );
}
