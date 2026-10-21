import type { ReactNode } from "react";

/**
 * A two-or-three-way view switcher, as on Calendrier: Jour / Semaine.
 *
 * A segmented control rather than underlined tabs because the choice is a
 * *view over the same data*, not navigation to somewhere else — and because
 * it reads as one object, which survives mirroring into Arabic without the
 * selected item appearing to belong to the wrong side.
 */
export default function Segmented<T extends string>({
  value,
  onChange,
  options,
  label,
}: {
  value: T;
  onChange: (next: T) => void;
  options: { id: T; label: string; icon?: ReactNode | undefined }[];
  /** Names the group for screen readers. */
  label: string;
}) {
  return (
    <div
      role="tablist"
      aria-label={label}
      className="inline-flex rounded-lg border border-line bg-sunken p-0.5"
    >
      {options.map((option) => {
        const selected = option.id === value;
        return (
          <button
            key={option.id}
            type="button"
            role="tab"
            aria-selected={selected}
            onClick={() => onChange(option.id)}
            className={[
              "inline-flex items-center gap-1.5 rounded-[0.3rem] px-3 py-1.5",
              "text-sm font-medium transition-colors",
              selected
                ? "bg-surface text-ink shadow-card"
                : "text-muted hover:text-ink",
            ].join(" ")}
          >
            {option.icon}
            {option.label}
          </button>
        );
      })}
    </div>
  );
}
