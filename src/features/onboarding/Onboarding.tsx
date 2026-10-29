import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";

import Logo from "@/components/Logo";
import { Num } from "@/components/Num";
import Button from "@/components/ui/Button";
import i18next, { LANGUAGE_DIRECTION, setLanguage, type Language } from "@/i18n";

/**
 * First launch: pick a language, then a one-minute tour of the sidebar.
 *
 * The language comes FIRST because nothing after it can be read until it is
 * chosen — so the welcome card is the one screen in the app written in both
 * languages at once, each line in its own script.
 *
 * The tour is a spotlight on the sidebar, one item per step, with one
 * sentence each. It walks the sidebar rather than buttons inside pages
 * because the sidebar is always there: a step pointing at a button that a
 * page only shows with data would point at nothing on a fresh install. Each
 * step opens its page behind the dimmed overlay, so the sentence has the
 * screen it describes underneath it.
 *
 * Kept short on purpose (CLAUDE.md, and the owner): a tour people skip after
 * step three teaches nothing. Every step here must earn its place.
 */

/** Remembered per device, like the language. Internal key: keeps the old name. */
const DONE_KEY = "tariqi.onboarding";

export function onboardingDone(): boolean {
  try {
    return localStorage.getItem(DONE_KEY) === "done";
  } catch {
    // Storage blocked: do not trap the user in a welcome they cannot dismiss
    // for good. They can still replay the tour from the sidebar.
    return true;
  }
}

function markDone(): void {
  try {
    localStorage.setItem(DONE_KEY, "done");
  } catch {
    // Not remembering is harmless: the welcome shows again next time.
  }
}

export type OnboardingPhase = "welcome" | "tour" | null;

/**
 * The steps, in the sidebar's own order so the highlight only ever moves
 * down. `target` is a `data-tour` attribute in the shell; `route` is the
 * page opened behind it.
 */
const STEPS = [
  { target: "/", route: "/", titleKey: "nav.home", textKey: "tour.step.home" },
  { target: "/calendar", route: "/calendar", titleKey: "nav.calendar", textKey: "tour.step.calendar" },
  { target: "/candidates", route: "/candidates", titleKey: "nav.candidates", textKey: "tour.step.candidates" },
  { target: "/exams", route: "/exams", titleKey: "nav.exams", textKey: "tour.step.exams" },
  { target: "nav.groupMoney", route: "/receivables", titleKey: "nav.groupMoney", textKey: "tour.step.money" },
  { target: "/backup", route: "/backup", titleKey: "nav.backup", textKey: "tour.step.backup" },
  { target: "help", route: null, titleKey: "tour.replay", textKey: "tour.step.help" },
] as const;

export default function Onboarding({
  phase,
  onPhase,
}: {
  phase: OnboardingPhase;
  onPhase: (next: OnboardingPhase) => void;
}) {
  const finish = useCallback(() => {
    markDone();
    onPhase(null);
  }, [onPhase]);

  if (phase === "welcome") {
    return (
      <Welcome
        onChoose={(language) => {
          void setLanguage(language).then(() => onPhase("tour"));
        }}
      />
    );
  }
  if (phase === "tour") return <Tour onFinish={finish} />;
  return null;
}

/* -------------------------------- welcome ------------------------------- */

function Welcome({ onChoose }: { onChoose: (language: Language) => void }) {
  // Each language's own words, whatever the app is currently set to: the
  // person reading this has not told us which one they read yet.
  const fr = i18next.getFixedT("fr");
  const ar = i18next.getFixedT("ar");

  return (
    <div className="fixed inset-0 z-[60] flex items-center justify-center bg-ink/50 p-6 backdrop-blur-[2px]">
      <div
        role="dialog"
        aria-modal="true"
        aria-label={`${fr("tour.welcome")} · ${ar("tour.welcome")}`}
        className="w-full max-w-md rounded-2xl border border-line bg-surface px-8 py-9 text-center shadow-overlay"
      >
        <Logo className="mx-auto size-16" />

        <h1 className="mt-5 text-xl font-semibold tracking-tight text-ink" dir="ltr">
          {fr("tour.welcome")}
        </h1>
        <p className="mt-1 text-xl font-semibold text-ink" dir="rtl" lang="ar">
          {ar("tour.welcome")}
        </p>

        <p className="mt-5 text-sm text-muted" dir="ltr">
          {fr("tour.chooseLanguage")}
        </p>
        <p className="text-sm text-muted" dir="rtl" lang="ar">
          {ar("tour.chooseLanguage")}
        </p>

        <div className="mt-6 grid grid-cols-2 gap-3">
          {(["fr", "ar"] as const).map((language) => (
            <button
              key={language}
              type="button"
              onClick={() => onChoose(language)}
              dir={LANGUAGE_DIRECTION[language]}
              lang={language}
              className="rounded-xl border border-line-strong bg-surface px-4 py-4 text-base font-semibold text-ink shadow-card transition-colors hover:border-brand hover:bg-brand-soft"
            >
              {(language === "fr" ? fr : ar)(language === "fr" ? "common.french" : "common.arabic")}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}

/* --------------------------------- tour --------------------------------- */

type Box = { top: number; left: number; width: number; height: number };

/** Space between the highlighted item and its card. */
const GAP = 16;
const CARD_WIDTH = 300;
/** How far the highlight ring sits outside the item. */
const PAD = 4;

function Tour({ onFinish }: { onFinish: () => void }) {
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const card = useRef<HTMLDivElement>(null);

  const [index, setIndex] = useState(0);
  const [box, setBox] = useState<Box | null>(null);
  const [cardHeight, setCardHeight] = useState(0);

  const step = STEPS[index] ?? STEPS[0];
  const last = index === STEPS.length - 1;
  const rtl = i18n.dir() === "rtl";

  // Open the page the step talks about, behind the overlay.
  useEffect(() => {
    if (step.route) navigate(step.route);
  }, [step, navigate]);

  // Where the highlighted item is. Measured, not assumed: the rail may be
  // collapsed, wider on a large screen, or on the right in Arabic.
  const measure = useCallback(() => {
    const element = document.querySelector<HTMLElement>(`[data-tour="${step.target}"]`);
    if (!element) {
      setBox(null);
      return;
    }
    const r = element.getBoundingClientRect();
    setBox({ top: r.top - PAD, left: r.left - PAD, width: r.width + PAD * 2, height: r.height + PAD * 2 });
  }, [step]);

  useLayoutEffect(() => {
    measure();
    window.addEventListener("resize", measure);
    return () => window.removeEventListener("resize", measure);
  }, [measure]);

  useLayoutEffect(() => {
    if (card.current) setCardHeight(card.current.offsetHeight);
  }, [index, box, i18n.language]);

  useEffect(() => {
    function onKey(event: KeyboardEvent) {
      if (event.key === "Escape") onFinish();
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onFinish]);

  function next() {
    if (last) {
      navigate("/");
      onFinish();
    } else {
      setIndex((i) => i + 1);
    }
  }

  // The card sits beside the highlight, on the side away from the rail —
  // to its right in French, to its left in Arabic. These are PHYSICAL
  // pixel positions on purpose: they come from getBoundingClientRect, which
  // measures the screen, not the text direction.
  const viewportHeight = typeof window === "undefined" ? 800 : window.innerHeight;
  const cardLeft = box ? (rtl ? box.left - GAP - CARD_WIDTH : box.left + box.width + GAP) : 0;
  const centre = box ? box.top + box.height / 2 : viewportHeight / 2;
  const cardTop = Math.min(
    Math.max(16, centre - cardHeight / 2),
    Math.max(16, viewportHeight - cardHeight - 16),
  );

  return (
    <div className="fixed inset-0 z-[60]" role="dialog" aria-modal="true" aria-label={t(step.titleKey)}>
      {/* The dim is the highlight's own shadow, so the item is the one
          bright hole in it. Clicks outside do nothing: the tour is left with
          its own buttons or Escape, never by a stray click. */}
      {box ? (
        <div
          className="pointer-events-none absolute rounded-xl ring-2 ring-white"
          style={{
            top: box.top,
            left: box.left,
            width: box.width,
            height: box.height,
            boxShadow: "0 0 0 9999px rgb(15 23 42 / 0.55)",
          }}
        />
      ) : (
        <div className="absolute inset-0 bg-ink/55" />
      )}

      <div
        ref={card}
        className="absolute rounded-xl border border-line bg-surface p-4 shadow-overlay"
        style={{ top: cardTop, left: cardLeft, width: CARD_WIDTH }}
      >
        {/* Pointer toward the item. The rail is always at the inline START,
            so the card's start edge is the one facing it in both languages. */}
        {box && (
          <span
            aria-hidden="true"
            className="absolute -start-1.5 size-3 rotate-45 border-b border-s border-line bg-surface rtl:-rotate-45"
            style={{ top: Math.min(Math.max(12, centre - cardTop - 6), cardHeight - 24) }}
          />
        )}

        <div className="flex items-center gap-2">
          <h2 className="text-sm font-semibold text-ink">{t(step.titleKey)}</h2>
          <span className="ms-auto text-xs text-faint">
            <Num>{`${index + 1} / ${STEPS.length}`}</Num>
          </span>
        </div>
        <p className="mt-1.5 text-sm leading-relaxed text-ink-soft">{t(step.textKey)}</p>

        <div className="mt-4 flex items-center gap-2">
          <button
            type="button"
            onClick={onFinish}
            className="text-xs text-muted transition-colors hover:text-ink"
          >
            {t("tour.skip")}
          </button>
          <span className="ms-auto flex gap-2">
            {index > 0 && (
              <Button size="sm" onClick={() => setIndex((i) => i - 1)}>
                {t("common.previous")}
              </Button>
            )}
            <Button size="sm" variant="primary" onClick={next}>
              {last ? t("tour.done") : t("common.next")}
            </Button>
          </span>
        </div>
      </div>
    </div>
  );
}
