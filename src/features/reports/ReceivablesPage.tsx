import { useCallback, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { useTranslation } from "react-i18next";

import { Money, Num } from "@/components/Num";
import { currentLanguage } from "@/i18n";
import { isAppError } from "@/lib/candidates";
import { getReceivables, type Receivables, type ReceivableRow } from "@/lib/reports";
import { isDesktop } from "@/lib/tauri";

/** Rows quieter than this are worth chasing. */
const STALE_DAYS = 30;

function name(row: ReceivableRow, locale: "fr" | "ar"): string {
  const arabic = `${row.lastNameAr} ${row.firstNameAr}`.trim();
  const latin = `${row.lastNameFr} ${row.firstNameFr}`.trim();
  return locale === "ar" ? arabic || latin : latin || arabic;
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
      <h1 className="mb-4 text-xl font-semibold">{t("receivables.title")}</h1>

      {error && (
        <p className="mb-3 rounded-md border border-owed bg-white px-3 py-2 text-sm text-owed">
          {error}
        </p>
      )}

      {/* The number the owner came for. */}
      <section className="mb-5 grid gap-4 rounded-lg border border-line bg-white px-5 py-4 sm:grid-cols-2">
        <div>
          <p className="text-xs uppercase tracking-wide text-muted">
            {t("receivables.totalOwed")}
          </p>
          <Money
            centimes={data?.totalOwedCentimes ?? 0}
            className="text-3xl font-bold text-owed"
          />
        </div>
        <div className="sm:text-end">
          <p className="text-xs uppercase tracking-wide text-muted">
            {t("receivables.owingCount")}
          </p>
          <p className="text-3xl font-bold">
            <Num>{data?.enrollmentsOwing ?? 0}</Num>
          </p>
        </div>
      </section>

      <label className="mb-3 flex items-center gap-2 text-sm">
        <input
          type="checkbox"
          checked={onlyStale}
          onChange={(e) => setOnlyStale(e.target.checked)}
        />
        {t("receivables.onlyStale", { days: STALE_DAYS })}
      </label>

      {loading && <p className="text-sm text-muted">{t("common.loading")}</p>}

      <div className="overflow-hidden rounded-lg border border-line bg-white">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-line text-xs uppercase tracking-wide text-muted">
              <th className="px-4 py-2 text-start font-medium">{t("candidates.colName")}</th>
              <th className="px-4 py-2 text-start font-medium">{t("candidates.colPhone")}</th>
              <th className="px-4 py-2 text-start font-medium">{t("enrollments.category")}</th>
              <th className="px-4 py-2 text-end font-medium">{t("enrollments.totalDue")}</th>
              <th className="px-4 py-2 text-end font-medium">{t("enrollments.paid")}</th>
              <th className="px-4 py-2 text-end font-medium">{t("enrollments.balance")}</th>
              <th className="px-4 py-2 text-start font-medium">
                {t("receivables.lastPayment")}
              </th>
            </tr>
          </thead>
          <tbody>
            {!loading && rows.length === 0 && (
              <tr>
                <td colSpan={7} className="px-4 py-8 text-center text-sm text-muted">
                  {onlyStale ? t("receivables.noStale") : t("receivables.nobodyOwes")}
                </td>
              </tr>
            )}

            {rows.map((row) => {
              const stale = (row.daysSincePayment ?? 0) >= STALE_DAYS;
              return (
                <tr key={row.enrollmentId} className="border-b border-line last:border-b-0">
                  <td className="px-4 py-2">
                    <Link
                      to={`/candidates/${row.candidateId}`}
                      className="font-medium text-brand hover:underline"
                    >
                      {name(row, locale)}
                    </Link>
                  </td>
                  <td className="px-4 py-2">
                    {row.phone ? <Num>{row.phone}</Num> : <span className="text-muted">—</span>}
                  </td>
                  <td className="px-4 py-2">{row.categoryCode}</td>
                  <td className="px-4 py-2 text-end">
                    <Money centimes={row.totalDueCentimes} whole />
                  </td>
                  <td className="px-4 py-2 text-end">
                    <Money centimes={row.paidCentimes} whole />
                  </td>
                  <td className="px-4 py-2 text-end">
                    <Money centimes={row.balanceCentimes} className="font-bold text-owed" />
                  </td>
                  <td className="px-4 py-2">
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
      </div>
    </div>
  );
}
