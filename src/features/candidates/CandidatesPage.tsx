import { useCallback, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { Columns3, List, Pencil, Plus, Search, Trash2, Users } from "lucide-react";

import { Num } from "@/components/Num";
import { Pager } from "@/components/ui/Pager";
import Button, { IconButton } from "@/components/ui/Button";
import { Badge, Card, EmptyState, ErrorNote, PageHeader, type Tone } from "@/components/ui/Surface";
import {
  deleteCandidate,
  emptyCandidateInput,
  isAppError,
  listCandidates,
  toInput,
  type Candidate,
  type CandidateInput,
} from "@/lib/candidates";
import { personName } from "@/lib/names";
import { isDesktop } from "@/lib/tauri";
import CandidateForm from "@/features/candidates/CandidateForm";
import ProgressBoard from "@/features/candidates/ProgressBoard";
import Segmented from "@/components/ui/Segmented";

const PAGE_SIZE = 50;

const STATUS_TONE: Record<string, Tone> = {
  prospect: "neutral",
  active: "brand",
  completed: "ok",
  inactive: "neutral",
  withdrawn: "warn",
};

type View = "list" | "board";

export default function CandidatesPage() {
  const { t } = useTranslation();

  // `?view=board` opens straight on the board, the same escape hatch the
  // calendar has, so it can be looked at without clicking through.
  const [view, setView] = useState<View>(() =>
    new URLSearchParams(window.location.search).get("view") === "board" ? "board" : "list",
  );

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

  const newCandidate = () => setEditing({ id: null, input: emptyCandidateInput() });

  return (
    <div>
      <PageHeader
        title={t("candidates.title")}
        actions={
          <Button
            variant="primary"
            icon={<Plus className="size-4" aria-hidden="true" />}
            onClick={newCandidate}
          >
            {t("candidates.new")}
          </Button>
        }
      >
        <Segmented<View>
          value={view}
          onChange={setView}
          label={t("candidates.title")}
          options={[
            {
              id: "list",
              label: t("candidates.viewList"),
              icon: <List className="size-4" aria-hidden="true" />,
            },
            {
              id: "board",
              label: t("candidates.viewBoard"),
              icon: <Columns3 className="size-4" aria-hidden="true" />,
            },
          ]}
        />
      </PageHeader>

      {view === "board" && <ProgressBoard />}

      {view === "list" && (
      <>
      <div className="mb-4 flex flex-wrap items-center gap-3">
        <div className="relative w-full max-w-md">
          <Search
            className="pointer-events-none absolute start-3 top-1/2 size-4 -translate-y-1/2 text-faint"
            aria-hidden="true"
          />
          <input
            type="search"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder={t("candidates.searchPlaceholder")}
            className="h-9 w-full rounded-md border border-line-strong bg-surface ps-9 pe-3 text-sm shadow-card outline-none transition-colors placeholder:text-faint focus:border-brand focus:ring-2 focus:ring-brand-soft"
          />
        </div>
        <p className="text-sm text-muted">
          {loading ? t("common.loading") : t("candidates.count", { n: total })}
        </p>
      </div>

      {error && <ErrorNote>{error}</ErrorNote>}

      {items.length === 0 && !loading ? (
        <EmptyState
          icon={<Users className="size-5" aria-hidden="true" />}
          title={debounced ? t("candidates.noMatches") : t("candidates.empty")}
          action={
            debounced ? undefined : (
              <Button
                variant="primary"
                icon={<Plus className="size-4" aria-hidden="true" />}
                onClick={newCandidate}
              >
                {t("candidates.new")}
              </Button>
            )
          }
        />
      ) : (
        <Card flush className="overflow-hidden">
          <table className="data-table">
            <thead>
              <tr>
                <th>{t("candidates.colName")}</th>
                <th>{t("candidates.colPhone")}</th>
                <th>{t("candidates.colNin")}</th>
                <th>{t("candidates.colStatus")}</th>
                <th>{t("candidates.colActions")}</th>
              </tr>
            </thead>
            <tbody>
              {items.map((candidate) => (
                <tr
                  key={candidate.id}
                  className="group"
                >
                  <td>
                    <Link
                      to={`/candidates/${candidate.id}`}
                      className="font-medium text-ink hover:text-brand hover:underline"
                    >
                      {personName(candidate)}
                    </Link>
                  </td>
                  <td className="text-ink-soft">
                    {candidate.phone ? (
                      <Num>{candidate.phone}</Num>
                    ) : (
                      <span className="text-faint">—</span>
                    )}
                  </td>
                  <td className="text-ink-soft">
                    {candidate.nin ? (
                      <Num>{candidate.nin}</Num>
                    ) : (
                      <span className="text-faint">—</span>
                    )}
                  </td>
                  <td>
                    <Badge tone={STATUS_TONE[candidate.status] ?? "neutral"}>
                      {t(`candidates.status_${candidate.status}`)}
                    </Badge>
                  </td>
                  <td>
                    {/* Row actions stay faint until the row is hovered or
                        something in it has keyboard focus, so fifty rows do
                        not read as a hundred buttons. */}
                    <div className="flex justify-end gap-1 opacity-0 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100">
                      <IconButton
                        aria-label={t("common.edit")}
                        onClick={() =>
                          setEditing({ id: candidate.id, input: toInput(candidate) })
                        }
                        className="size-8"
                      >
                        <Pencil className="size-3.5" aria-hidden="true" />
                      </IconButton>
                      <IconButton
                        aria-label={t("common.delete")}
                        onClick={() => setConfirmDelete(candidate)}
                        className="size-8 text-owed hover:bg-owed-soft"
                      >
                        <Trash2 className="size-3.5" aria-hidden="true" />
                      </IconButton>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </Card>
      )}

      <Pager
        page={page}
        pageSize={PAGE_SIZE}
        total={total}
        onPage={setPage}
        busy={loading}
      />

      </>
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
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-ink/40 p-6 backdrop-blur-[2px]">
          <div
            role="dialog"
            aria-modal="true"
            className="w-full max-w-md rounded-lg border border-line bg-surface p-5 shadow-overlay"
          >
            <h2 className="text-base font-semibold tracking-tight text-ink">
              {t("candidates.deleteTitle")}
            </h2>
            <p className="mt-2 text-sm text-ink-soft">
              {t("candidates.deleteBody", { name: personName(confirmDelete) })}
            </p>
            <p className="mt-2 text-xs text-muted">{t("candidates.deleteNote")}</p>
            <div className="mt-5 flex gap-2">
              <Button
                variant="primary"
                className="bg-owed hover:bg-owed"
                icon={<Trash2 className="size-4" aria-hidden="true" />}
                onClick={() => void doDelete(confirmDelete)}
              >
                {t("common.delete")}
              </Button>
              <Button onClick={() => setConfirmDelete(null)}>{t("common.cancel")}</Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
