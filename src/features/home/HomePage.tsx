import { useTranslation } from "react-i18next";

type FoundationItem = {
  labelKey: string;
  done: boolean;
};

/**
 * Placeholder dashboard.
 *
 * It shows the state of the technical foundation while there are no business
 * screens yet. It is replaced by the three money screens — candidate balance,
 * receivables, money in/out — from docs/09-FIRST-RELEASE.md.
 */
export default function HomePage() {
  const { t } = useTranslation();

  const foundation: FoundationItem[] = [
    { labelKey: "home.checkI18n", done: true },
    { labelKey: "home.checkMoney", done: true },
    { labelKey: "home.checkPrint", done: true },
    { labelKey: "home.checkDb", done: false },
    { labelKey: "home.checkBackup", done: false },
  ];

  return (
    <div className="max-w-2xl">
      <h1 className="text-xl font-semibold">{t("home.title")}</h1>
      <p className="mt-1 text-sm text-muted">{t("home.subtitle")}</p>

      <p className="mt-4 rounded-md border border-line bg-white px-3 py-2 text-sm text-muted">
        {t("home.foundationNotice")}
      </p>

      <section className="mt-6 rounded-lg border border-line bg-white">
        <h2 className="border-b border-line px-4 py-3 text-xs uppercase tracking-wide text-muted">
          {t("home.checklistTitle")}
        </h2>
        <ul>
          {foundation.map((item) => (
            <li
              key={item.labelKey}
              className="flex items-center gap-3 border-b border-line px-4 py-2.5 last:border-b-0"
            >
              <span
                aria-hidden="true"
                className={[
                  "flex size-5 shrink-0 items-center justify-center rounded-full text-[11px] font-bold text-white",
                  item.done ? "bg-ok" : "bg-line",
                ].join(" ")}
              >
                {item.done ? "✓" : ""}
              </span>
              <span className="text-sm">{t(item.labelKey)}</span>
              <span
                className={[
                  "ms-auto shrink-0 text-[11px] uppercase tracking-wide",
                  item.done ? "text-ok" : "text-muted",
                ].join(" ")}
              >
                {item.done ? t("home.statusDone") : t("home.statusPending")}
              </span>
            </li>
          ))}
        </ul>
      </section>
    </div>
  );
}
