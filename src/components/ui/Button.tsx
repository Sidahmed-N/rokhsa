import type { ButtonHTMLAttributes, ReactNode } from "react";

export type ButtonVariant = "primary" | "secondary" | "ghost" | "danger";
export type ButtonSize = "sm" | "md" | "lg";

/**
 * The one button in the app.
 *
 * Variants exist so that a screen can only ever have one obvious action.
 * "Fait" on the calendar is primary; "Absent" and "Annulée" beside it are
 * secondary, because an owner closing the day presses Fait fifty times and
 * the other two rarely. Before this, all three looked the same and the
 * common action was no easier to hit than the exceptional ones.
 */
const VARIANTS: Record<ButtonVariant, string> = {
  primary:
    "bg-brand text-white shadow-card hover:bg-brand-hover active:bg-brand-hover disabled:bg-faint",
  secondary:
    "bg-surface text-ink border border-line-strong shadow-card hover:bg-sunken hover:border-line-strong disabled:text-faint",
  ghost: "text-ink-soft hover:bg-sunken hover:text-ink disabled:text-faint",
  danger:
    "bg-surface text-owed border border-owed/30 hover:bg-owed-soft disabled:text-faint",
};

const SIZES: Record<ButtonSize, string> = {
  sm: "h-8 gap-1.5 px-2.5 text-xs",
  md: "h-9 gap-2 px-3.5 text-sm",
  lg: "h-11 gap-2 px-5 text-sm",
};

export default function Button({
  variant = "secondary",
  size = "md",
  icon,
  className,
  children,
  ...rest
}: {
  variant?: ButtonVariant | undefined;
  size?: ButtonSize | undefined;
  /** A lucide icon element, placed before the label in reading order. */
  icon?: ReactNode | undefined;
  children?: ReactNode | undefined;
} & Omit<ButtonHTMLAttributes<HTMLButtonElement>, "className" | "children"> & {
    className?: string | undefined;
  }) {
  return (
    <button
      type="button"
      {...rest}
      className={[
        "app-button inline-flex shrink-0 items-center justify-center rounded-md font-medium",
        "transition-colors disabled:cursor-not-allowed",
        VARIANTS[variant],
        SIZES[size],
        className ?? "",
      ]
        .filter(Boolean)
        .join(" ")}
    >
      {icon}
      {children}
    </button>
  );
}

/**
 * A square button that is only an icon.
 *
 * Always takes an `aria-label`, because there is no visible text to read out
 * and half the users are driving the app by keyboard.
 */
export function IconButton({
  variant = "ghost",
  className,
  children,
  ...rest
}: {
  variant?: ButtonVariant | undefined;
  "aria-label": string;
  children?: ReactNode | undefined;
} & Omit<ButtonHTMLAttributes<HTMLButtonElement>, "className" | "children"> & {
    className?: string | undefined;
  }) {
  return (
    <button
      type="button"
      {...rest}
      className={[
        "app-button inline-flex size-9 shrink-0 items-center justify-center rounded-md",
        "transition-colors disabled:cursor-not-allowed",
        VARIANTS[variant],
        className ?? "",
      ]
        .filter(Boolean)
        .join(" ")}
    >
      {children}
    </button>
  );
}
