import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Check, Search } from "lucide-react";

import { Badge } from "@/components/ui/Surface";
import { currentLanguage } from "@/i18n";
import { isAppError } from "@/lib/candidates";
import { personName } from "@/lib/names";
import { isDesktop } from "@/lib/tauri";
import { listBookableEnrollments, type BookableEnrollment } from "@/lib/training";

/**
 * Choosing whose session this is.
 *
 * A list with a search box rather than a `<select>`: a school with three
 * hundred candidates makes a dropdown useless, and the secretary knows the
 * name but not the position in an alphabetical list.
 *
 * Filtering is done by the BACKEND, not here. `fold_query` in Rust already
 * knows that "Benaïssa" and "benaissa" are the same person, and that the
 * several Arabic alef forms, tashkeel and tatweel all fold together. A
 * second implementation in TypeScript would agree with it right up until
 * someone typed an Arabic name with a different alef.
 *
 * Every enrolment is offered, not just the ones in training: a candidate who
 * was ajourné needs more hours before a repassage, and booking that session
 * is exactly what someone is trying to do.
 */
export default function CandidatePicker({
  value,
  onChange,
  error,
}: {
  value: number | null;
  onChange: (enrollmentId: number, row: BookableEnrollment) => void;
  error?: string | undefined;
}) {
  const { t } = useTranslation();
  const locale = currentLanguage();

  const [query, setQuery] = useState("");
  const [debounced, setDebounced] = useState("");
  const [rows, setRows] = useState<BookableEnrollment[]>([]);
  const [loadError, setLoadError] = useState<string | null>(null);

  // Same 250ms as the candidates screen: a fast typist should not fire a
  // query per keystroke on a machine with an HDD.
  useEffect(() => {
    const timer = setTimeout(() => setDebounced(query), 250);
    return () => clearTimeout(timer);
  }, [query]);

  const load = useCallback(async () => {
    if (!isDesktop()) return;
    try {
      setRows(await listBookableEnrollments(debounced));
      setLoadError(null);
    } catch (raw: unknown) {
      setLoadError(isAppError(raw) ? t(raw.code, { defaultValue: raw.message }) : String(raw));
    }
  }, [debounced, t]);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <div>
      <span className="mb-1.5 block text-xs font-medium text-ink-soft">
        {t("calendar.forCandidate")}
      </span>

      <div className="relative">
        <Search
          className="pointer-events-none absolute start-3 top-1/2 size-4 -translate-y-1/2 text-faint"
          aria-hidden="true"
        />
        <input
          type="search"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder={t("candidates.searchPlaceholder")}
          className={[
            "h-9 w-full rounded-md border bg-surface ps-9 pe-3 text-sm outline-none",
            "transition-colors placeholder:text-faint focus:border-brand focus:ring-2 focus:ring-brand-soft",
            error ? "border-owed" : "border-line-strong",
          ].join(" ")}
        />
      </div>

      <div className="mt-2 max-h-48 overflow-y-auto rounded-md border border-line">
        {loadError && <p className="px-3 py-2 text-xs text-owed">{loadError}</p>}

        {!loadError && rows.length === 0 && (
          <p className="px-3 py-3 text-center text-xs text-muted">
            {t("candidates.noMatches")}
          </p>
        )}

        <ul className="divide-y divide-line">
          {rows.map((row) => {
            const chosen = row.enrollmentId === value;
            return (
              <li key={row.enrollmentId}>
                <button
                  type="button"
                  onClick={() => onChange(row.enrollmentId, row)}
                  className={[
                    "flex w-full items-center gap-2 px-3 py-2 text-start text-sm transition-colors",
                    chosen ? "bg-brand-soft text-brand" : "hover:bg-sunken",
                  ].join(" ")}
                >
                  {chosen ? (
                    <Check className="size-3.5 shrink-0" aria-hidden="true" />
                  ) : (
                    <span className="size-3.5 shrink-0" aria-hidden="true" />
                  )}
                  <span className="min-w-0 flex-1 truncate font-medium">
                    {personName(row, locale)}
                  </span>
                  {/* A candidate who is not simply "in training" is worth
                      flagging here — booking for someone who abandoned is
                      allowed, but it should be a deliberate choice. */}
                  {row.status !== "forming" && (
                    <Badge tone={row.status === "withdrawn" ? "warn" : "neutral"}>
                      {t(`enrollments.status_${row.status}`)}
                    </Badge>
                  )}
                </button>
              </li>
            );
          })}
        </ul>
      </div>

      {error && <span className="mt-1 block text-xs font-medium text-owed">{error}</span>}
    </div>
  );
}
