import { useCallback, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { useTranslation } from "react-i18next";

import { Num } from "@/components/Num";
import { currentLanguage } from "@/i18n";
import {
  deleteCandidate,
  displayName,
  emptyCandidateInput,
  isAppError,
  listCandidates,
  toInput,
  type Candidate,
  type CandidateInput,
} from "@/lib/candidates";
import { isDesktop } from "@/lib/tauri";
import CandidateForm from "@/features/candidates/CandidateForm";

const PAGE_SIZE = 50;

export default function CandidatesPage() {
  const { t } = useTranslation();
  const locale = currentLanguage();

  const [query, setQuery] = useState("");
  const [debounced, setDebounced] = useState("");
  const [page, setPage] = useState(0);
  const [items, setItems] = useState<Candidate[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [editing, setEditing] = useState<{ id: number | null; input: CandidateInput } | null>(
    null,
  );
  const [confirmDelete, setConfirmDelete] = useState<Candidate | null>(null);

  // Debounce so a fast typist doesn't fire a query per keystroke on a weak PC.
  useEffect(() => {
    const timer = setTimeout(() => setDebounced(query), 250);
    return () => clearTimeout(timer);
  }, [query]);

  // A new search starts at the first page, otherwise searching from page 4
  // shows an empty list and looks like "no results".
  useEffect(() => {
    setPage(0);
  }, [debounced]);

  const refresh = useCallback(async () => {
    if (!isDesktop()) {
      setError(t("candidates.needsDesktop"));
      return;
    }
    setLoading(true);
    setError(null);
    try {
      // Named `result`, not `page`: that would shadow the page state and
      // silently pin the offset to zero.
      const result = await listCandidates(debounced, PAGE_SIZE, page * PAGE_SIZE);
      setItems(result.items);
      setTotal(result.total);
    } catch (raw: unknown) {
      setError(isAppError(raw) ? t(raw.code, { defaultValue: raw.message }) : String(raw));
    } finally {
      setLoading(false);
    }
  }, [debounced, page, t]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  async function doDelete(candidate: Candidate) {
    try {
      await deleteCandidate(candidate.id);
      setConfirmDelete(null);
      void refresh();
    } catch (raw: unknown) {
      setError(isAppError(raw) ? t(raw.code, { defaultValue: raw.message }) : String(raw));
    }
  }

  return (
    <div>
      <div className="mb-4 flex flex-wrap items-center gap-3">
        <h1 className="text-xl font-semibold">{t("candidates.title")}</h1>
        <button
          type="button"
          onClick={() => setEditing({ id: null, input: emptyCandidateInput() })}
          className="rounded-md bg-brand px-3 py-1.5 text-sm font-medium text-white"
        >
          {t("candidates.new")}
        </button>
      </div>

      <input
        type="search"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder={t("candidates.searchPlaceholder")}
        className="mb-3 w-full max-w-md rounded-md border border-line bg-white px-3 py-2 text-sm outline-none focus:border-brand focus:ring-2 focus:ring-brand-soft"
      />

      {error && (
        <p className="mb-3 rounded-md border border-owed bg-white px-3 py-2 text-sm text-owed">
          {error}
        </p>
      )}

      <p className="mb-2 text-xs text-muted">
        {loading ? t("common.loading") : t("candidates.count", { n: total })}
      </p>

      <div className="overflow-hidden rounded-lg border border-line bg-white">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-line text-xs uppercase tracking-wide text-muted">
              <th className="px-4 py-2 text-start font-medium">{t("candidates.colName")}</th>
              <th className="px-4 py-2 text-start font-medium">{t("candidates.colPhone")}</th>
              <th className="px-4 py-2 text-start font-medium">{t("candidates.colNin")}</th>
              <th className="px-4 py-2 text-start font-medium">{t("candidates.colStatus")}</th>
              <th className="px-4 py-2 text-end font-medium">{t("candidates.colActions")}</th>
            </tr>
          </thead>
          <tbody>
            {items.length === 0 && !loading && (
              <tr>
                <td colSpan={5} className="px-4 py-8 text-center text-sm text-muted">
                  {debounced ? t("candidates.noMatches") : t("candidates.empty")}
                </td>
              </tr>
            )}

            {items.map((candidate) => (
              <tr key={candidate.id} className="border-b border-line last:border-b-0">
                <td className="px-4 py-2">
                  <Link
                    to={`/candidates/${candidate.id}`}
                    className="font-medium text-brand hover:underline"
                  >
                    {displayName(candidate, locale)}
                  </Link>
                  {locale === "fr" && candidate.lastNameAr && (
                    // The spacing lives on an OUTER span that inherits the row's
                    // direction. Putting ms-* on the same element that carries
                    // dir="rtl" makes margin-inline-start resolve against *that*
                    // element, so the gap appears on the wrong visual side and
                    // the two names touch.
                    <span className="ms-2 text-xs text-muted">
                      <span dir="rtl">
                        {candidate.lastNameAr} {candidate.firstNameAr}
                      </span>
                    </span>
                  )}
                </td>
                <td className="px-4 py-2">
                  {candidate.phone ? <Num>{candidate.phone}</Num> : <span className="text-muted">—</span>}
                </td>
                <td className="px-4 py-2">
                  {candidate.nin ? <Num>{candidate.nin}</Num> : <span className="text-muted">—</span>}
                </td>
                <td className="px-4 py-2">
                  <span className="rounded-full bg-brand-soft px-2 py-0.5 text-xs text-brand">
                    {t(`candidates.status_${candidate.status}`)}
                  </span>
                </td>
                <td className="px-4 py-2 text-end">
                  <button
                    type="button"
                    onClick={() => setEditing({ id: candidate.id, input: toInput(candidate) })}
                    className="rounded-md px-2 py-1 text-xs text-brand hover:bg-brand-soft"
                  >
                    {t("common.edit")}
                  </button>
                  <button
                    type="button"
                    onClick={() => setConfirmDelete(candidate)}
                    className="ms-1 rounded-md px-2 py-1 text-xs text-owed hover:bg-brand-soft"
                  >
                    {t("common.delete")}
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {total > PAGE_SIZE && (
        <div className="mt-3 flex flex-wrap items-center gap-2">
          <button
            type="button"
            onClick={() => setPage((p) => Math.max(0, p - 1))}
            disabled={page === 0 || loading}
            className="rounded-md border border-line px-3 py-1.5 text-sm disabled:opacity-40"
          >
            ← {t("common.previous")}
          </button>
          <button
            type="button"
            onClick={() => setPage((p) => p + 1)}
            disabled={(page + 1) * PAGE_SIZE >= total || loading}
            className="rounded-md border border-line px-3 py-1.5 text-sm disabled:opacity-40"
          >
            {t("common.next")} →
          </button>
          <span className="text-xs text-muted">
            {t("candidates.showingRange", {
              from: page * PAGE_SIZE + 1,
              to: Math.min((page + 1) * PAGE_SIZE, total),
              total,
            })}
          </span>
        </div>
      )}

      {editing && (
        <CandidateForm
          initial={editing.input}
          editingId={editing.id}
          onSaved={() => {
            setEditing(null);
            void refresh();
          }}
          onCancel={() => setEditing(null)}
        />
      )}

      {confirmDelete && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-6">
          <div className="w-full max-w-md rounded-lg border border-line bg-white p-5 shadow-lg">
            <h2 className="text-base font-semibold">{t("candidates.deleteTitle")}</h2>
            <p className="mt-2 text-sm text-muted">
              {t("candidates.deleteBody", { name: displayName(confirmDelete, locale) })}
            </p>
            <p className="mt-2 text-xs text-muted">{t("candidates.deleteNote")}</p>
            <div className="mt-4 flex gap-2">
              <button
                type="button"
                onClick={() => void doDelete(confirmDelete)}
                className="rounded-md bg-owed px-4 py-2 text-sm font-medium text-white"
              >
                {t("common.delete")}
              </button>
              <button
                type="button"
                onClick={() => setConfirmDelete(null)}
                className="rounded-md border border-line px-4 py-2 text-sm"
              >
                {t("common.cancel")}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
