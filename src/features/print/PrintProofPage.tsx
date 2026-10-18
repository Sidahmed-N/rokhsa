import { useTranslation } from "react-i18next";

import Receipt, { type ReceiptData, type ReceiptSchool } from "@/print/Receipt";

/**
 * Arabic print proof.
 *
 * This page exists to retire the single biggest technical risk in the project:
 * whether Arabic prints correctly — letters joined, text right-to-left, numbers
 * and money the right way round. See docs/09-FIRST-RELEASE.md, build step 1.
 *
 * The data below is a placeholder so the layout can be judged. It is replaced
 * by real records once the database and payment screens land.
 */

const DEMO_SCHOOL: ReceiptSchool = {
  nameFr: "Auto-École El Amel",
  nameAr: "مدرسة تعليم السياقة الأمل",
  addressFr: "12, Rue Didouche Mourad, Alger-Centre, Alger",
  addressAr: "12 شارع ديدوش مراد، الجزائر الوسطى، الجزائر",
  phone: "0551 23 45 67",
  agrementNumber: "16/2019/0482",
};

const DEMO_RECEIPT: ReceiptData = {
  receiptNumber: "2026-0147",
  paidOn: "2026-10-18",
  candidateNameFr: "BENALI Yacine",
  candidateNameAr: "بن علي ياسين",
  nin: "109 2004 1600 5482 130",
  categoryCode: "B",
  amount: 1_200_000, // 12 000,00 DA
  methodKey: "cash",
  totalDue: 3_500_000, // 35 000,00 DA
  paidTotal: 2_200_000, // 22 000,00 DA
  remaining: 1_300_000, // 13 000,00 DA
};

export default function PrintProofPage() {
  const { t } = useTranslation();

  return (
    <div>
      <div className="screen-only mb-6 max-w-2xl">
        <h1 className="text-xl font-semibold">{t("printProof.title")}</h1>
        <p className="mt-2 text-sm leading-relaxed text-muted">{t("printProof.explain")}</p>
        <p className="mt-3 rounded-md border border-line bg-brand-soft px-3 py-2 text-sm text-brand">
          {t("printProof.instruction")}
        </p>

        <button
          type="button"
          onClick={() => window.print()}
          className="mt-4 rounded-md bg-brand px-4 py-2 text-sm font-medium text-white transition-opacity hover:opacity-90"
        >
          {t("common.print")}
        </button>
      </div>

      <h2 className="screen-only mb-2 text-xs uppercase tracking-wide text-muted">
        {t("printProof.sheetLabel")}
      </h2>

      <Receipt school={DEMO_SCHOOL} data={DEMO_RECEIPT} />
    </div>
  );
}
