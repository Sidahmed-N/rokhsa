import type { ReactNode } from "react";

/**
 * Containers, headings and status chips.
 *
 * Kept in one file because they are the vocabulary every screen is written
 * in, and splitting five twenty-line components across five files makes them
 * harder to keep consistent, not easier.
 */

/** A white panel. `flush` drops the padding when the content is a table. */
export function Card({
  children,
  flush = false,
  tone,
  className,
}: {
  children: ReactNode;
  flush?: boolean | undefined;
  /** Colours the border only — for a row that needs to stand out in a list. */
  tone?: "brand" | "ok" | "warn" | "owed" | undefined;
  className?: string | undefined;
}) {
  const border =
    tone === "brand"
      ? "border-brand-line"
      : tone === "ok"
        ? "border-ok/30"
        : tone === "warn"
          ? "border-warn/30"
          : tone === "owed"
            ? "border-owed/30"
            : "border-line";

  return (
    <section
      className={[
        // `surface-blur` does nothing in the solid theme and frosts the card
        // in the glass one — see styles/surfaces.css.
        "surface-blur rounded-lg border bg-surface shadow-card",
        border,
        flush ? "" : "p-4",
        className ?? "",
      ]
        .filter(Boolean)
        .join(" ")}
    >
      {children}
    </section>
  );
}

/**
 * The title block at the top of a screen.
 *
 * `actions` sits at the inline end, so it lands on the right in French and on
 * the left in Arabic without the caller thinking about it.
 */
export function PageHeader({
  title,
  description,
  actions,
  children,
}: {
  title: string;
  description?: string | undefined;
  actions?: ReactNode | undefined;
  /** Controls that belong with the title, such as a view switcher. */
  children?: ReactNode | undefined;
}) {
  return (
    <header className="mb-5">
      <div className="flex flex-wrap items-center gap-x-4 gap-y-2">
        <h1 className="text-[1.375rem] font-semibold tracking-tight text-ink">{title}</h1>
        {children}
        {actions && <div className="ms-auto flex items-center gap-2">{actions}</div>}
      </div>
      {description && <p className="mt-1 text-sm text-muted">{description}</p>}
    </header>
  );
}

/**
 * The one card on a screen that matters more than the others.
 *
 * Dark, so it wins without needing to be bigger, and so the figure on it can
 * be large without the page turning into a wall of competing boxes. A screen
 * has at most one — the moment there are two, neither is a hero.
 */
export function Hero({
  label,
  children,
  footer,
  action,
}: {
  label: string;
  children: ReactNode;
  footer?: ReactNode | undefined;
  action?: ReactNode | undefined;
}) {
  return (
    <section className="relative overflow-hidden rounded-xl bg-ink p-5 text-white shadow-raised">
      {/* A single soft highlight so the dark panel is not a flat rectangle. */}
      <span
        aria-hidden="true"
        className="pointer-events-none absolute -top-24 -end-16 size-56 rounded-full bg-white/10 blur-2xl"
      />
      <div className="relative flex flex-wrap items-end gap-x-6 gap-y-3">
        <div className="min-w-0">
          <p className="text-xs font-medium tracking-wide text-white/60 uppercase">{label}</p>
          <p className="mt-1.5 text-4xl font-semibold tracking-tight">{children}</p>
          {footer && <div className="mt-2 text-sm text-white/70">{footer}</div>}
        </div>
        {action && <div className="ms-auto">{action}</div>}
      </div>
    </section>
  );
}

export type Tone = "neutral" | "brand" | "ok" | "warn" | "owed";

const BADGE_TONES: Record<Tone, string> = {
  neutral: "bg-sunken text-ink-soft",
  brand: "bg-brand-soft text-brand",
  ok: "bg-ok-soft text-ok",
  warn: "bg-warn-soft text-warn",
  owed: "bg-owed-soft text-owed",
};

/** A small status pill. */
export function Badge({
  tone = "neutral",
  children,
  className,
}: {
  tone?: Tone | undefined;
  children: ReactNode;
  className?: string | undefined;
}) {
  return (
    <span
      className={[
        "inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium",
        BADGE_TONES[tone],
        className ?? "",
      ]
        .filter(Boolean)
        .join(" ")}
    >
      {children}
    </span>
  );
}

/**
 * What a screen shows when it has nothing to show.
 *
 * Always offers the action that would fill it. An empty screen that only says
 * "nothing here" makes the owner feel the app is broken; one that says
 * "nothing booked — book a session" tells him what the app is for.
 */
export function EmptyState({
  icon,
  title,
  description,
  action,
}: {
  icon?: ReactNode | undefined;
  title: string;
  description?: string | undefined;
  action?: ReactNode | undefined;
}) {
  return (
    <div className="rounded-lg border border-dashed border-line-strong bg-surface/60 px-6 py-12 text-center">
      {icon && (
        <div className="mx-auto mb-3 flex size-10 items-center justify-center rounded-full bg-sunken text-faint">
          {icon}
        </div>
      )}
      <p className="text-sm font-medium text-ink">{title}</p>
      {description && <p className="mx-auto mt-1 max-w-sm text-sm text-muted">{description}</p>}
      {action && <div className="mt-4 flex justify-center">{action}</div>}
    </div>
  );
}

/** An error strip. Errors are never raw strings — see CLAUDE.md. */
export function ErrorNote({ children }: { children: ReactNode }) {
  return (
    <p
      role="alert"
      className="mb-4 rounded-md border border-owed/30 bg-owed-soft px-3 py-2 text-sm text-owed"
    >
      {children}
    </p>
  );
}
