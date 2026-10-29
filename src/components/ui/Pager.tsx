import { useEffect, useMemo, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { ChevronLeft, ChevronRight } from "lucide-react";

import { Num } from "@/components/Num";
import Button from "@/components/ui/Button";

/**
 * Previous / next, and where you are.
 *
 * One control for every long list, so paging means the same thing on every
 * screen. It renders nothing when everything fits on one page: a pager under
 * eleven rows is clutter.
 *
 * The range is ONE isolate. "1–50" is two numbers either side of a neutral
 * dash, and inside Arabic text the bidi algorithm treats both numbers as
 * right-to-left for the purpose of placing the dash — so an un-isolated range
 * renders as "50–1". The total is a second isolate for the same reason.
 */
export function Pager({
  page,
  pageSize,
  total,
  onPage,
  busy = false,
}: {
  /** Zero-based. */
  page: number;
  pageSize: number;
  total: number;
  onPage: (page: number) => void;
  busy?: boolean | undefined;
}) {
  const { t } = useTranslation();

  if (total <= pageSize) return null;

  const from = page * pageSize + 1;
  const to = Math.min((page + 1) * pageSize, total);
  const last = (page + 1) * pageSize >= total;

  return (
    <div className="mt-3 flex flex-wrap items-center gap-2">
      <Button
        size="sm"
        onClick={() => onPage(Math.max(0, page - 1))}
        disabled={page === 0 || busy}
        icon={<ChevronLeft className="size-4 rtl:rotate-180" aria-hidden="true" />}
      >
        {t("common.previous")}
      </Button>
      <Button size="sm" onClick={() => onPage(page + 1)} disabled={last || busy}>
        {t("common.next")}
        <ChevronRight className="size-4 rtl:rotate-180" aria-hidden="true" />
      </Button>
      <span className="text-xs text-muted">
        <Num>{`${from}–${to}`}</Num> {t("pager.of")} <Num>{total}</Num>
      </span>
    </div>
  );
}

/**
 * Page through a list that is already in memory.
 *
 * For lists the backend returns whole because it has to: the receivables
 * total is a sum over every debtor, so Rust computes all of them anyway, and
 * the cost a long list actually has on a 4 GB machine is rendering hundreds
 * of table rows, not carrying them.
 *
 * The page CLAMPS rather than resets when the list shrinks — marking the last
 * row of page 3 should leave you on what is now the last page, not throw you
 * back to page 1. `resetKey` is for the other case: a new filter or search is
 * a different list, and that one does start again at the top.
 */
export function usePaged<T>(items: T[], pageSize: number, resetKey?: unknown) {
  const [page, setPage] = useState(0);

  useEffect(() => {
    setPage(0);
  }, [resetKey]);

  const lastPage = Math.max(0, Math.ceil(items.length / pageSize) - 1);
  const current = Math.min(page, lastPage);

  const pageItems = useMemo(
    () => items.slice(current * pageSize, (current + 1) * pageSize),
    [items, current, pageSize],
  );

  return {
    pageItems,
    pager: { page: current, pageSize, total: items.length, onPage: setPage },
  };
}

/**
 * The same, for a list drawn inside a loop.
 *
 * A candidate file renders one card per enrolment, and a hook cannot be
 * called inside that map. This gives each card its own page state.
 */
export function Paged<T>({
  items,
  pageSize,
  children,
}: {
  items: T[];
  pageSize: number;
  children: (pageItems: T[]) => ReactNode;
}) {
  const { pageItems, pager } = usePaged(items, pageSize);
  return (
    <>
      {children(pageItems)}
      <Pager {...pager} />
    </>
  );
}
