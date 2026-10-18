import { useEffect, useState } from "react";
import { Link, useParams, useSearchParams } from "react-router-dom";
import { useTranslation } from "react-i18next";

import { getCandidate, isAppError, type Candidate } from "@/lib/candidates";
import { isDesktop } from "@/lib/tauri";
import {
  getEnrollment,
  getPayment,
  getSchool,
  type EnrollmentSummary,
  type Payment,
  type School,
} from "@/lib/training";
import Receipt, { type ReceiptData, type ReceiptSchool } from "@/print/Receipt";

/**
 * A receipt at its own address.
 *
 * Giving the receipt a real route rather than printing it inline means there is
 * ONE code path for "print this payment": taking a payment navigates here with
 * `?autoprint=1`, and reprinting an old one navigates here without it. It also
 * means a receipt can be reopened and re-examined, which a modal cannot.
 *
 * The running totals shown are TODAY's, not the ones at the time of payment, so
 * a reprint reflects the current balance.
 */
export default function ReceiptPage() {
  const { t } = useTranslation();
  const params = useParams();
  const [search] = useSearchParams();
  const paymentId = Number(params.paymentId);
  const autoprint = search.get("autoprint") === "1";

  const [data, setData] = useState<ReceiptData | null>(null);
  const [school, setSchool] = useState<ReceiptSchool | null>(null);
  const [backTo, setBackTo] = useState("/candidates");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!isDesktop()) {
      setError(t("candidates.needsDesktop"));
      return;
    }

    let cancelled = false;

    async function load() {
      try {
        const payment: Payment = await getPayment(paymentId);
        const summary: EnrollmentSummary = await getEnrollment(payment.enrollmentId);
        const candidate: Candidate = await getCandidate(summary.enrollment.candidateId);
        const s: School = await getSchool();
        if (cancelled) return;

        setBackTo(`/candidates/${candidate.id}`);
        setSchool({
          nameFr: s.nameFr || t("school.unnamed"),
          nameAr: s.nameAr || t("school.unnamed"),
          addressFr: s.addressFr,
          addressAr: s.addressAr,
          phone: s.phone,
          agrementNumber: s.agrementNumber,
        });
        setData({
          receiptNumber: payment.receiptNumber,
          paidOn: payment.paidOn,
          candidateNameFr: `${candidate.lastNameFr} ${candidate.firstNameFr}`.trim(),
          candidateNameAr: `${candidate.lastNameAr} ${candidate.firstNameAr}`.trim(),
          nin: candidate.nin ?? "—",
          categoryCode: summary.enrollment.categoryCode,
          amount: payment.amountCentimes,
          methodKey: payment.methodCode,
          totalDue: summary.totalDueCentimes,
          paidTotal: summary.paidCentimes,
          remaining: summary.balanceCentimes,
        });
      } catch (raw: unknown) {
        if (!cancelled) {
          setError(isAppError(raw) ? t(raw.code, { defaultValue: raw.message }) : String(raw));
        }
      }
    }

    void load();
    return () => {
      cancelled = true;
    };
  }, [paymentId, t]);

  // Print once the receipt is actually on screen, not before.
  useEffect(() => {
    if (!autoprint || !data || !school) return;
    const timer = setTimeout(() => window.print(), 150);
    return () => clearTimeout(timer);
  }, [autoprint, data, school]);

  return (
    <div>
      <div className="screen-only mb-4 flex flex-wrap items-center gap-2">
        <Link to={backTo} className="text-sm text-brand hover:underline">
          ← {t("common.back")}
        </Link>
        <button
          type="button"
          onClick={() => window.print()}
          className="ms-auto rounded-md bg-brand px-4 py-2 text-sm font-medium text-white"
        >
          {t("common.print")}
        </button>
      </div>

      {error && (
        <p className="screen-only rounded-md border border-owed bg-white px-3 py-2 text-sm text-owed">
          {error}
        </p>
      )}

      {!error && (!data || !school) && (
        <p className="screen-only text-sm text-muted">{t("common.loading")}</p>
      )}

      {data && school && <Receipt school={school} data={data} />}
    </div>
  );
}
