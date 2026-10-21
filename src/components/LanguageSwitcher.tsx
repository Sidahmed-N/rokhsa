import { useTranslation } from "react-i18next";

import { SUPPORTED_LANGUAGES, currentLanguage, setLanguage, type Language } from "@/i18n";

/**
 * French / Arabic toggle.
 *
 * Changing the language also flips the document direction, so this control is
 * the fastest way to check that a screen survives RTL. Keep it reachable on
 * every page during development.
 *
 * Each label is written in its own script — "Français" and "العربية", never
 * "French" and "Arabic" — so a secretary who reads only one of them can still
 * find the one she wants.
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
      className="flex rounded-lg border border-line bg-sunken p-0.5"
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
              "flex-1 rounded-[0.3rem] px-2.5 py-1 text-xs font-medium transition-colors",
              isActive ? "bg-surface text-ink shadow-card" : "text-muted hover:text-ink",
            ].join(" ")}
          >
            {labels[language]}
          </button>
        );
      })}
    </div>
  );
}
