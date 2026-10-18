import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";

import { Money, Num } from "@/components/Num";
import { currentLanguage } from "@/i18n";
import { amountInWords, type Centimes } from "@/lib/money";

export interface ReceiptSchool {
  nameFr: string;
  nameAr: string;
  addressFr: string;
  addressAr: string;
  phone: string;
  agrementNumber: string;
}

export interface ReceiptData {
  receiptNumber: string;
  /** ISO date, YYYY-MM-DD. */
  paidOn: string;
  candidateNameFr: string;
  candidateNameAr: string;
  nin: string;
  categoryCode: string;
  amount: Centimes;
  methodKey: string;
  /** All computed in Rust. The frontend never does money arithmetic. */
  totalDue: Centimes;
  paidTotal: Centimes;
  remaining: Centimes;
}

/** "2026-10-18" -> "18/10/2026". Western digits, day first, both languages. */
function formatDate(iso: string): string {
  const [year, month, day] = iso.split("-");
  if (!year || !month || !day) return iso;
  return `${day}/${month}/${year}`;
}

/**
 * One label/value line.
 *
 * `value` is a ReactNode rather than a string so that numeric values must be
 * passed through <Num> / <Money>, which isolate them from Arabic bidi
 * reordering. See src/components/Num.tsx.
 */
function Field({ label, value }: { label: string; value: ReactNode }) {
  return (
    <div className="flex gap-2 border-b border-dotted border-line py-1.5">
      <span className="shrink-0 text-[11px] uppercase tracking-wide text-muted">{label}</span>
      <span className="ms-auto text-end text-sm font-medium">{value}</span>
    </div>
  );
}

/**
 * Payment receipt (reçu de versement / وصل دفع).
 *
 * Plain HTML printed by the WebView, which handles Arabic shaping and
 * bidirectional layout on its own. That is why we are not generating PDFs in
 * Rust — see docs/03-STACK.md, Decision 4.
 */
export default function Receipt({
  school,
  data,
}: {
  school: ReceiptSchool;
  data: ReceiptData;
}) {
  const { t } = useTranslation();
  const locale = currentLanguage();
  const isArabic = locale === "ar";

  const schoolName = isArabic ? school.nameAr : school.nameFr;
  const schoolAddress = isArabic ? school.addressAr : school.addressFr;
  const candidateName =
    (isArabic ? data.candidateNameAr : data.candidateNameFr) ||
    data.candidateNameFr ||
    data.candidateNameAr;

  const remaining = data.remaining;

  return (
    <article className="print-sheet mx-auto w-full max-w-[148mm] border border-line bg-white p-6 shadow-sm">
      {/* Letterhead */}
      <header className="border-b-2 border-ink pb-3">
        <h1 className="text-base font-bold leading-tight">{schoolName}</h1>
        {schoolAddress && <p className="mt-0.5 text-xs text-muted">{schoolAddress}</p>}
        {/* Empty fields are omitted entirely rather than printed as a label
            with nothing after it — a receipt reading "Tél:  ·  Agrément n°"
            looks broken to the candidate holding it. */}
        {(school.phone || school.agrementNumber) && (
          <p className="text-xs text-muted">
            {school.phone && (
              <>
                {t("receipt.phone")}: <Num>{school.phone}</Num>
              </>
            )}
            {school.phone && school.agrementNumber && "  ·  "}
            {school.agrementNumber && (
              <>
                {t("receipt.agrement")} <Num>{school.agrementNumber}</Num>
              </>
            )}
          </p>
        )}
      </header>

      {/* Title */}
      <h2 className="my-4 text-center text-lg font-bold tracking-wide">
        {t("receipt.title")}
      </h2>

      {/* Reference line */}
      <div className="mb-3 flex items-baseline justify-between text-sm">
        <span>
          {t("receipt.number")}:{" "}
          <Num className="font-bold">{data.receiptNumber}</Num>
        </span>
        <span>
          {t("receipt.date")}: <Num className="font-bold">{formatDate(data.paidOn)}</Num>
        </span>
      </div>

      {/* Body */}
      <section className="no-break">
        <Field label={t("receipt.receivedFrom")} value={candidateName} />
        <Field label={t("receipt.nin")} value={<Num>{data.nin}</Num>} />
        <Field
          label={t("receipt.category")}
          value={t(`categories.${data.categoryCode}`, { defaultValue: data.categoryCode })}
        />
        <Field label={t("receipt.purpose")} value={t("receipt.purposeValue")} />
        <Field label={t("receipt.method")} value={t(`methods.${data.methodKey}`)} />
      </section>

      {/* Amount — the thing the candidate checks */}
      <section className="no-break my-4 rounded-md border-2 border-ink px-4 py-3">
        <div className="flex items-baseline justify-between">
          <span className="text-xs uppercase tracking-wide text-muted">
            {t("receipt.amount")}
          </span>
          <Money centimes={data.amount} className="text-xl font-bold" />
        </div>
        <p className="mt-2 border-t border-dotted border-line pt-2 text-xs leading-relaxed">
          <span className="text-muted">{t("receipt.inWords")}: </span>
          <em className="font-medium not-italic">{amountInWords(data.amount, locale)}</em>
        </p>
      </section>

      {/* Running totals */}
      <section className="no-break mb-5 text-sm">
        <Field
          label={t("receipt.totalAgreed")}
          value={<Money centimes={data.totalDue} />}
        />
        <Field label={t("receipt.totalPaid")} value={<Money centimes={data.paidTotal} />} />
        <div className="flex gap-2 py-1.5">
          <span className="shrink-0 text-[11px] uppercase tracking-wide text-muted">
            {t("receipt.remaining")}
          </span>
          <Money
            centimes={remaining}
            className={[
              "ms-auto text-end text-sm font-bold",
              remaining > 0 ? "text-owed" : "text-ok",
            ].join(" ")}
          />
        </div>
      </section>

      {/* Signature */}
      <footer className="no-break">
        <div className="flex justify-end">
          <div className="w-44 text-center">
            <div className="h-14" />
            <div className="border-t border-ink pt-1 text-xs text-muted">
              {t("receipt.signature")}
            </div>
          </div>
        </div>
        <p className="mt-4 text-center text-[10px] text-muted">{t("receipt.footer")}</p>
      </footer>
    </article>
  );
}
