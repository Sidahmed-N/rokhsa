import { useEffect, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { X } from "lucide-react";

import Button, { IconButton } from "@/components/ui/Button";
import { ErrorNote } from "@/components/ui/Surface";

/**
 * A dialog.
 *
 * Escape closes it and Enter submits from any single-line field, because the
 * people using this are typing a candidate's details with someone waiting at
 * the counter, and reaching for the mouse to confirm costs more than it
 * sounds. Both are handled here so no individual dialog has to remember.
 */
export default function Modal({
  title,
  children,
  onClose,
  onSubmit,
  submitLabel,
  submitting = false,
  error,
  wide = false,
}: {
  title: string;
  children: ReactNode;
  onClose: () => void;
  onSubmit: () => void;
  submitLabel?: string | undefined;
  submitting?: boolean | undefined;
  error?: string | null | undefined;
  wide?: boolean | undefined;
}) {
  const { t } = useTranslation();

  useEffect(() => {
    function onKey(event: KeyboardEvent) {
      if (event.key === "Escape") onClose();
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center overflow-y-auto bg-ink/40 p-6 backdrop-blur-[2px]">
      <div
        role="dialog"
        aria-modal="true"
        aria-label={title}
        className={[
          "mt-[6vh] w-full rounded-lg border border-line bg-surface shadow-overlay",
          wide ? "max-w-2xl" : "max-w-md",
        ].join(" ")}
      >
        <header className="flex items-center gap-3 border-b border-line px-5 py-3.5">
          <h2 className="text-base font-semibold tracking-tight text-ink">{title}</h2>
          <IconButton
            aria-label={t("common.cancel")}
            onClick={onClose}
            className="ms-auto -me-1.5"
          >
            <X className="size-4" aria-hidden="true" />
          </IconButton>
        </header>

        {/* Enter submits, except from a textarea where it means a new line. */}
        <form
          onSubmit={(event) => {
            event.preventDefault();
            if (!submitting) onSubmit();
          }}
        >
          <div className="max-h-[68vh] overflow-y-auto px-5 py-4">
            {error && <ErrorNote>{error}</ErrorNote>}
            {children}
          </div>

          <footer className="flex gap-2 border-t border-line bg-sunken/60 px-5 py-3">
            <Button type="submit" variant="primary" disabled={submitting}>
              {submitting ? t("common.loading") : (submitLabel ?? t("common.save"))}
            </Button>
            <Button onClick={onClose}>{t("common.cancel")}</Button>
          </footer>
        </form>
      </div>
    </div>
  );
}
