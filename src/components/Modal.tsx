import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";

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

  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center overflow-y-auto bg-black/40 p-6">
      <div
        className={[
          "w-full rounded-lg border border-line bg-white shadow-lg",
          wide ? "max-w-2xl" : "max-w-md",
        ].join(" ")}
      >
        <header className="border-b border-line px-5 py-3">
          <h2 className="text-base font-semibold">{title}</h2>
        </header>

        <div className="max-h-[70vh] overflow-y-auto px-5 py-4">
          {error && (
            <p className="mb-3 rounded-md border border-owed bg-white px-3 py-2 text-sm text-owed">
              {error}
            </p>
          )}
          {children}
        </div>

        <footer className="flex gap-2 border-t border-line px-5 py-3">
          <button
            type="button"
            onClick={onSubmit}
            disabled={submitting}
            className="rounded-md bg-brand px-4 py-2 text-sm font-medium text-white disabled:opacity-50"
          >
            {submitting ? t("common.loading") : (submitLabel ?? t("common.save"))}
          </button>
          <button
            type="button"
            onClick={onClose}
            className="rounded-md border border-line px-4 py-2 text-sm"
          >
            {t("common.cancel")}
          </button>
        </footer>
      </div>
    </div>
  );
}
