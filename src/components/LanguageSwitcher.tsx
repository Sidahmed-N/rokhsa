import { useTranslation } from "react-i18next";

import { SUPPORTED_LANGUAGES, currentLanguage, setLanguage, type Language } from "@/i18n";

/**
 * French / Arabic toggle.
 *
 * Changing the language also flips the document direction, so this control is
 * the fastest way to check that a screen survives RTL. Keep it reachable on
 * every page during development.
 */
export default function LanguageSwitcher() {
  const { t } = useTranslation();
  const active = currentLanguage();

  const labels: Record<Language, string> = {
    fr: t("common.french"),
    ar: t("common.arabic"),
  };

  return (
    <div
      className="flex items-center gap-1 rounded-lg border border-line bg-white p-1"
      role="group"
      aria-label={t("common.language")}
    >
      {SUPPORTED_LANGUAGES.map((language) => {
        const isActive = language === active;
        return (
          <button
            key={language}
            type="button"
            onClick={() => void setLanguage(language)}
            aria-pressed={isActive}
            className={[
              "rounded-md px-3 py-1 text-sm transition-colors",
              isActive
                ? "bg-brand text-white"
                : "text-muted hover:bg-brand-soft hover:text-brand",
            ].join(" ")}
          >
            {labels[language]}
          </button>
        );
      })}
    </div>
  );
}
