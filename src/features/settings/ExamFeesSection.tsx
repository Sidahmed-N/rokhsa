import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { ErrorNote } from "@/components/ui/Surface";
import { isAppError } from "@/lib/candidates";
import { listExamFees, setExamFee, type ExamFee } from "@/lib/exams";
import { formatDZD, parseDZD } from "@/lib/money";
import { isDesktop } from "@/lib/tauri";

/**
 * What the school collects for each exam.
 *
 * Ships at zero on purpose. The amount is set by the administration, not by
 * us, and a number we invented would be read as authoritative the moment a
 * school saw it on screen. Zero shows nothing on a candidate's file and
 * charges nobody until the owner types what he actually collects.
 *
 * Changing an amount only affects candidates entered for an exam afterwards —
 * every charge keeps the price of the day it was made, like the agreed price
 * on an enrolment.
 */

function FeeRow({
  fee,
  onSaved,
  onError,
}: {
  fee: ExamFee;
  onSaved: () => void;
  onError: (message: string) => void;
}) {
  const { t } = useTranslation();

  const [amount, setAmount] = useState(
    fee.amountCentimes > 0 ? formatDZD(fee.amountCentimes, { withSuffix: false }) : "",
  );
  const [saving, setSaving] = useState(false);

  const dirty = (parseDZD(amount) ?? 0) !== fee.amountCentimes;

  async function save() {
    setSaving(true);
    try {
      await setExamFee(fee.kind, parseDZD(amount) ?? 0);
      onSaved();
    } catch (raw: unknown) {
      onError(isAppError(raw) ? t(raw.code, { defaultValue: raw.message }) : String(raw));
    } finally {
      setSaving(false);
    }
  }

  return (
    <tr className="border-b border-line last:border-b-0">
      <td className="px-3 py-2 font-medium">{t(`lessons.kind_${fee.kind}`)}</td>
      <td className="px-3 py-2">
        <input
          type="text"
          value={amount}
          onChange={(e) => setAmount(e.target.value)}
          dir="ltr"
          placeholder="0"
          aria-label={t("fees.amount")}
          className="w-28 rounded-md border border-line-strong bg-surface px-2 py-1 text-sm outline-none transition-colors focus:border-brand focus:ring-2 focus:ring-brand-soft"
        />
      </td>
      <td className="px-3 py-2 text-end">
        {dirty && (
          <button
            type="button"
            onClick={() => void save()}
            disabled={saving}
            className="inline-flex h-8 shrink-0 items-center justify-center gap-1.5 rounded-md bg-brand px-2.5 text-xs font-medium text-white shadow-card transition-colors hover:bg-brand-hover disabled:cursor-not-allowed disabled:bg-faint"
          >
            {saving ? t("common.loading") : t("common.save")}
          </button>
        )}
      </td>
    </tr>
  );
}

export default function ExamFeesSection() {
  const { t } = useTranslation();

  const [fees, setFees] = useState<ExamFee[]>([]);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    if (!isDesktop()) return;
    try {
      setFees(await listExamFees());
      setError(null);
    } catch (raw: unknown) {
      setError(isAppError(raw) ? t(raw.code, { defaultValue: raw.message }) : String(raw));
    }
  }, [t]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return (
    <section className="mb-6 rounded-lg border border-line bg-surface shadow-card">
      <h2 className="border-b border-line px-4 py-3 text-xs tracking-wide text-muted uppercase">
        {t("fees.section")}
      </h2>
      <p className="border-b border-line px-4 py-2 text-xs text-muted">
        {t("fees.note")}
      </p>

      {error && (
        <div className="px-4 pt-3">
          <ErrorNote>{error}</ErrorNote>
        </div>
      )}

      <table className="w-full text-sm">
        <thead>
          <tr className="border-b border-line text-xs tracking-wide text-muted uppercase">
            <th className="px-3 py-2 text-start font-medium">{t("fees.exam")}</th>
            <th className="px-3 py-2 text-start font-medium">{t("fees.amount")}</th>
            <th className="px-3 py-2 text-end font-medium" />
          </tr>
        </thead>
        <tbody>
          {fees.map((fee) => (
            <FeeRow
              key={fee.kind}
              fee={fee}
              onSaved={() => void refresh()}
              onError={setError}
            />
          ))}
        </tbody>
      </table>
    </section>
  );
}
