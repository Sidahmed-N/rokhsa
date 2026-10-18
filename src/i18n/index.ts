import i18next from "i18next";
import { initReactI18next } from "react-i18next";

import fr from "@/locales/fr.json";
import ar from "@/locales/ar.json";

export const SUPPORTED_LANGUAGES = ["fr", "ar"] as const;
export type Language = (typeof SUPPORTED_LANGUAGES)[number];

export const LANGUAGE_DIRECTION: Record<Language, "ltr" | "rtl"> = {
  fr: "ltr",
  ar: "rtl",
};

const STORAGE_KEY = "tariqi.language";

function isLanguage(value: unknown): value is Language {
  return typeof value === "string" && (SUPPORTED_LANGUAGES as readonly string[]).includes(value);
}

/**
 * Read the remembered language. Wrapped because storage can throw in a
 * restricted context, and a thrown error here would stop the app booting.
 */
function readStoredLanguage(): Language | null {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    return isLanguage(stored) ? stored : null;
  } catch {
    return null;
  }
}

function storeLanguage(language: Language): void {
  try {
    localStorage.setItem(STORAGE_KEY, language);
  } catch {
    // Not being able to remember the choice is a minor annoyance, never a failure.
  }
}

/**
 * Apply the language to the document.
 *
 * Setting `dir` here is what makes the whole UI flip, because every layout
 * rule uses CSS logical properties (ms-/me-/ps-/pe-/start-/end-) rather than
 * physical left/right. See CLAUDE.md.
 */
export function applyLanguageToDocument(language: Language): void {
  const html = document.documentElement;
  html.setAttribute("lang", language);
  html.setAttribute("dir", LANGUAGE_DIRECTION[language]);
}

/** French is the default: it is the administrative language of auto-écoles. */
export const DEFAULT_LANGUAGE: Language = "fr";

export async function setLanguage(language: Language): Promise<void> {
  await i18next.changeLanguage(language);
  storeLanguage(language);
  applyLanguageToDocument(language);
}

export function currentLanguage(): Language {
  return isLanguage(i18next.resolvedLanguage) ? i18next.resolvedLanguage : DEFAULT_LANGUAGE;
}

/**
 * `?lang=ar` forces a language for one load.
 *
 * Checking a screen in Arabic is the single most common thing that gets
 * skipped, and skipping it is how RTL bugs ship. This makes it one URL away
 * for anyone, including automated checks, without clearing stored settings.
 */
function readLanguageFromUrl(): Language | null {
  try {
    const value = new URLSearchParams(window.location.search).get("lang");
    return isLanguage(value) ? value : null;
  } catch {
    return null;
  }
}

export async function initI18n(): Promise<void> {
  const initial = readLanguageFromUrl() ?? readStoredLanguage() ?? DEFAULT_LANGUAGE;

  await i18next.use(initReactI18next).init({
    resources: {
      fr: { translation: fr },
      ar: { translation: ar },
    },
    lng: initial,
    fallbackLng: DEFAULT_LANGUAGE,
    supportedLngs: [...SUPPORTED_LANGUAGES],
    interpolation: {
      // React already escapes output.
      escapeValue: false,
    },
    returnNull: false,
    // A missing key is a bug we want to see loudly in development, not a
    // silently blank label in front of a school secretary.
    saveMissing: import.meta.env.DEV,
    missingKeyHandler: import.meta.env.DEV
      ? (_lngs, _ns, key) => {
          console.error(`[i18n] missing translation key: ${key}`);
        }
      : false,
  });

  applyLanguageToDocument(initial);
}

export default i18next;
