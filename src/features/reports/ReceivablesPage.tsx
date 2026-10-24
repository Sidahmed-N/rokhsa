import { useCallback, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { useTranslation } from "react-i18next";

import { Wallet } from "lucide-react";

import { Money, Num } from "@/components/Num";
import { Card, EmptyState, ErrorNote, PageHeader } from "@/components/ui/Surface";
import { currentLanguage } from "@/i18n";
import { isAppError } from "@/lib/candidates";
import { personName } from "@/lib/names";
import { getReceivables, type Receivables, type ReceivableRow } from "@/lib/reports";
import { isDesktop } from "@/lib/tauri";

/** Rows quieter than this are worth chasing. */
const STALE_DAYS = 30;

function name(row: ReceivableRow, locale: "fr" | "ar"): string {
  return personName(row, locale);
}

/**
 * Who owes money.
 *
 * This is the screen an owner opens the app for, and the one to lead a demo
 * with: a school cannot answer "how much am I owed?" from its notebooks
 * without adding up pages.
 */
export default function ReceivablesPage() {
  const { t } = useTranslation();
  const locale = currentLanguage();

  const [data, setData] = useState<Receivables | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [onlyStale, setOnlyStale] = useState(false);

  const refresh = useCallback(async () => {
    if (!isDesktop()) {
      setError(t("candidates.needsDesktop"));
      setLoading(false);
      return;
    }
    setLoading(true);
    try {
      setData(await getReceivables());
      setError(null);
    } catch (raw: unknown) {
      setError(isAppError(raw) ? t(raw.code, { defaultValue: raw.message }) : String(raw));
    } finally {
      setLoading(false);
    }
  }, [t]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const rows = (data?.rows ?? []).filter(
    (row) => !onlyStale || (row.daysSincePayment ?? 0) >= STALE_DAYS,
  );

  return (
    <div>
      <PageHeader title={t("receivables.title")} />

      {error && <ErrorNote>{error}</ErrorNote>}

      {/* The number the owner came for. */}
      <Card className="mb-4">
        <div className="grid gap-4 sm:grid-cols-2">
          <div>
            <p className="text-xs font-medium tracking-wide text-muted uppercase">
              {t("receivables.totalOwed")}
            </p>
            <Money
              centimes={data?.totalOwedCentimes ?? 0}
              className="mt-1 block text-3xl font-semibold tracking-tight text-owed"
            />
          </div>
          <div className="sm:text-end">
            <p className="text-xs font-medium tracking-wide text-muted uppercase">
              {t("receivables.owingCount")}
            </p>
            <p className="mt-1 text-3xl font-semibold tracking-tight text-ink">
              <Num>{data?.enrollmentsOwing ?? 0}</Num>
            </p>
          </div>
        </div>
      </Card>

      <label className="mb-4 inline-flex cursor-pointer items-center gap-2 text-sm text-ink-soft">
        <input
          type="checkbox"
          checked={onlyStale}
          onChange={(e) => setOnlyStale(e.target.checked)}
          className="size-4 accent-[oklch(0.52_0.16_258)]"
        />
        {t("receivables.onlyStale", { days: STALE_DAYS })}
      </label>

      {loading && <p className="text-sm text-muted">{t("common.loading")}</p>}

      {!loading && rows.length === 0 ? (
        <EmptyState
          icon={<Wallet className="size-5" aria-hidden="true" />}
          title={onlyStale ? t("receivables.noStale") : t("receivables.nobodyOwes")}
        />
      ) : (
      <Card flush className="overflow-hidden">
        <table className="data-table">
          <thead>
            <tr>
              <th>{t("candidates.colName")}</th>
              <th>{t("candidates.colPhone")}</th>
              <th className="text-end">{t("enrollments.totalDue")}</th>
              <th className="text-end">{t("enrollments.paid")}</th>
              <th className="text-end">{t("enrollments.balance")}</th>
              <th>
                {t("receivables.lastPayment")}
              </th>
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => {
              const stale = (row.daysSincePayment ?? 0) >= STALE_DAYS;
              return (
                <tr key={row.enrollmentId}>
                  <td>
                    <Link
                      to={`/candidates/${row.candidateId}`}
                      className="font-medium text-ink hover:text-brand hover:underline"
                    >
                      {name(row, locale)}
                    </Link>
                  </td>
                  <td>
                    {row.phone ? <Num>{row.phone}</Num> : <span className="text-faint">—</span>}
                  </td>
                  <td className="text-end">
                    <Money centimes={row.totalDueCentimes} whole />
                  </td>
                  <td className="text-end">
                    <Money centimes={row.paidCentimes} whole />
                  </td>
                  <td className="text-end">
                    <Money centimes={row.balanceCentimes} className="font-semibold text-owed" />
                  </td>
                  <td>
                    {row.lastPaymentOn ? (
                      <Num>{row.lastPaymentOn}</Num>
                    ) : (
                      <span className="text-muted">{t("receivables.never")}</span>
                    )}
                    {row.daysSincePayment !== null && (
                      <span className={["ms-2 text-xs", stale ? "text-owed" : "text-muted"].join(" ")}>
                        <Num>{row.daysSincePayment}</Num> {t("receivables.daysAgo")}
                      </span>
                    )}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </Card>
      )}
    </div>
  );
}
