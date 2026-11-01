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
const STEPS: readonly Step[] = [
  { target: "/", route: "/", titleKey: "nav.home", textKey: "tour.step.home" },
  { target: "/calendar", route: "/calendar", titleKey: "nav.calendar", textKey: "tour.step.calendar" },
  { target: "/candidates", route: "/candidates", titleKey: "nav.candidates", textKey: "tour.step.candidates" },
  { target: "/exams", route: "/exams", titleKey: "nav.exams", textKey: "tour.step.exams" },
  { target: "nav.groupMoney", route: "/receivables", titleKey: "nav.groupMoney", textKey: "tour.step.money" },
  { target: "/backup", route: "/backup", titleKey: "nav.backup", textKey: "tour.step.backup" },
  { target: "help", route: null, titleKey: "tour.replay", textKey: "tour.step.help" },
];

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
  if (phase === "tour") return <Tour steps={STEPS} onFinish={finish} beside />;
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

/** One spotlight step. `target` is a `data-tour` attribute on the screen. */
type Step = {
  target: string;
  /** Page to open behind the overlay, for the sidebar tour. */
  route?: string | null | undefined;
  titleKey: string;
  textKey: string;
};

type Box = { top: number; left: number; width: number; height: number };

/** Space between the highlighted item and its card. */
const GAP = 16;
const CARD_WIDTH = 300;
/** How far the highlight ring sits outside the item. */
const PAD = 4;
/** Keep the card this far from the window's edges. */
const MARGIN = 16;

function findTarget(target: string): HTMLElement | null {
  return document.querySelector<HTMLElement>(`[data-tour="${target}"]`);
}

/** On screen and with a size: an element that exists but is empty is not a step. */
function isShown(target: string): boolean {
  const el = findTarget(target);
  if (!el) return false;
  const r = el.getBoundingClientRect();
  return r.width > 0 && r.height > 0;
}

/**
 * The spotlight itself, for both tours.
 *
 * `beside` is the sidebar tour: the card sits next to the highlighted item,
 * on the side away from the rail, with a pointer. Inside a page there is no
 * such side to count on, so the card goes BELOW the highlighted part, or
 * above it when there is no room, lined up with its start edge — and the
 * page is scrolled so the part is in view first.
 */
function Tour({
  steps,
  onFinish,
  beside,
}: {
  steps: readonly Step[];
  onFinish: () => void;
  beside: boolean;
}) {
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const card = useRef<HTMLDivElement>(null);

  const [index, setIndex] = useState(0);
  const [box, setBox] = useState<Box | null>(null);
  const [cardHeight, setCardHeight] = useState(0);

  const step = steps[index] ?? steps[0];
  const last = index === steps.length - 1;
  const rtl = i18n.dir() === "rtl";

  // Open the page the step talks about, behind the overlay.
  useEffect(() => {
    if (step?.route) navigate(step.route);
  }, [step, navigate]);

  // Where the highlighted item is. Measured, not assumed: the rail may be
  // collapsed, wider on a large screen, or on the right in Arabic.
  const measure = useCallback(() => {
    const element = step ? findTarget(step.target) : null;
    if (!element) {
      setBox(null);
      return;
    }
    const r = element.getBoundingClientRect();
    setBox({ top: r.top - PAD, left: r.left - PAD, width: r.width + PAD * 2, height: r.height + PAD * 2 });
  }, [step]);

  useLayoutEffect(() => {
    // A part of a page may be below the fold: bring it into view first.
    if (!beside && step) findTarget(step.target)?.scrollIntoView({ block: "nearest" });
    measure();
    window.addEventListener("resize", measure);
    window.addEventListener("scroll", measure, true);
    return () => {
      window.removeEventListener("resize", measure);
      window.removeEventListener("scroll", measure, true);
    };
  }, [measure, beside, step]);

  useLayoutEffect(() => {
    if (card.current) setCardHeight(card.current.offsetHeight);
  }, [index, box, i18n.language]);

  // Escape leaves; Enter goes on, so the tour can be read without reaching
  // for the mouse. Through a ref, because `next` is redefined every render.
  const nextRef = useRef<() => void>(() => {});
  useEffect(() => {
    function onKey(event: KeyboardEvent) {
      if (event.key === "Escape") onFinish();
      if (event.key === "Enter") {
        event.preventDefault();
        nextRef.current();
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onFinish]);

  if (!step) return null;

  nextRef.current = next;
  function next() {
    if (last) {
      // The sidebar tour ends where the day starts.
      if (beside) navigate("/");
      onFinish();
    } else {
      setIndex((i) => i + 1);
    }
  }

  // PHYSICAL pixel positions on purpose: they come from
  // getBoundingClientRect, which measures the screen, not the text direction.
  const viewportWidth = typeof window === "undefined" ? 1200 : window.innerWidth;
  const viewportHeight = typeof window === "undefined" ? 800 : window.innerHeight;
  const clamp = (v: number, min: number, max: number) => Math.min(Math.max(v, min), Math.max(min, max));

  let cardLeft = 0;
  let cardTop = 0;
  const centre = box ? box.top + box.height / 2 : viewportHeight / 2;
  if (beside) {
    // To the right of the rail in French, to its left in Arabic.
    cardLeft = box ? (rtl ? box.left - GAP - CARD_WIDTH : box.left + box.width + GAP) : 0;
    cardTop = clamp(centre - cardHeight / 2, MARGIN, viewportHeight - cardHeight - MARGIN);
  } else if (box) {
    const below = box.top + box.height + GAP;
    cardTop =
      below + cardHeight <= viewportHeight - MARGIN
        ? below
        : clamp(box.top - GAP - cardHeight, MARGIN, viewportHeight - cardHeight - MARGIN);
    // Lined up with the part's start edge: its left in French, right in Arabic.
    const start = rtl ? box.left + box.width - CARD_WIDTH : box.left;
    cardLeft = clamp(start, MARGIN, viewportWidth - CARD_WIDTH - MARGIN);
  } else {
    cardLeft = (viewportWidth - CARD_WIDTH) / 2;
    cardTop = (viewportHeight - cardHeight) / 2;
  }

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
            so the card's start edge is the one facing it in both languages.
            Inside a page the card sits below or above, and needs none. */}
        {beside && box && (
          <span
            aria-hidden="true"
            className="absolute -start-1.5 size-3 rotate-45 border-b border-s border-line bg-surface rtl:-rotate-45"
            style={{ top: Math.min(Math.max(12, centre - cardTop - 6), cardHeight - 24) }}
          />
        )}

        <div className="flex items-center gap-2">
          <h2 className="text-sm font-semibold text-ink">{t(step.titleKey)}</h2>
          <span className="ms-auto text-xs text-faint">
            <Num>{`${index + 1} / ${steps.length}`}</Num>
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

/* ------------------------------ page tours ------------------------------ */

/**
 * A short tour of what is INSIDE each daily page, shown the first time it is
 * opened: two to four steps, one sentence each.
 *
 * Unlike the sidebar tour, these point at parts of a page that may not be
 * there — no overdue sessions, no exam sitting yet on a fresh install — so a
 * step whose part is not on screen is left out and kept for later: it is
 * shown on the first visit after that part appears.
 */
export type PageTourId = "home" | "calendar" | "candidates" | "exams";

const PAGE_STEPS: Record<PageTourId, Step[]> = {
  home: [
    { target: "home-owed", titleKey: "receivables.totalOwed", textKey: "tour.page.home.owed" },
    { target: "home-today", titleKey: "home.todayTitle", textKey: "tour.page.home.today" },
    { target: "home-chase", titleKey: "home.chaseTitle", textKey: "tour.page.home.chase" },
    { target: "home-overdue", titleKey: "home.overdueTitle", textKey: "tour.page.home.overdue" },
  ],
  calendar: [
    { target: "calendar-views", titleKey: "tour.page.calendar.viewsTitle", textKey: "tour.page.calendar.views" },
    { target: "calendar-book", titleKey: "calendar.book", textKey: "tour.page.calendar.book" },
    { target: "calendar-day", titleKey: "tour.page.calendar.dayTitle", textKey: "tour.page.calendar.day" },
  ],
  candidates: [
    { target: "candidates-new", titleKey: "candidates.new", textKey: "tour.page.candidates.new" },
    { target: "candidates-search", titleKey: "tour.page.candidates.searchTitle", textKey: "tour.page.candidates.search" },
    { target: "candidates-views", titleKey: "tour.page.candidates.viewsTitle", textKey: "tour.page.candidates.views" },
  ],
  exams: [
    { target: "exams-new", titleKey: "exams.newSession", textKey: "tour.page.exams.new" },
    { target: "exams-sittings", titleKey: "tour.page.exams.sittingsTitle", textKey: "tour.page.exams.sittings" },
    { target: "exams-entries", titleKey: "tour.page.exams.entriesTitle", textKey: "tour.page.exams.entries" },
  ],
};

const PAGE_TOUR_IDS = Object.keys(PAGE_STEPS) as PageTourId[];

/** The page tour for a route, if that route has one. Exact paths only. */
export function pageTourFor(pathname: string): PageTourId | null {
  const map: Record<string, PageTourId> = {
    "/": "home",
    "/calendar": "calendar",
    "/candidates": "candidates",
    "/exams": "exams",
  };
  return map[pathname] ?? null;
}

const pageKey = (page: PageTourId) => `tariqi.tour.${page}`;

/**
 * The steps of a page already shown. Remembered per step, not per page: on
 * an empty install the exams page has only its "new sitting" button, and
 * marking the whole page seen then meant sittings and results were never
 * explained. "done" is what an earlier version stored for a finished page.
 */
function seenSteps(page: PageTourId): Set<string> {
  try {
    const raw = localStorage.getItem(pageKey(page));
    if (raw === "done") return new Set(PAGE_STEPS[page].map((s) => s.target));
    const list: unknown = raw ? JSON.parse(raw) : [];
    return new Set(Array.isArray(list) ? list.filter((x): x is string => typeof x === "string") : []);
  } catch {
    // Storage blocked: count everything as seen rather than show it each time.
    return new Set(PAGE_STEPS[page].map((s) => s.target));
  }
}

/** Replaying the sidebar tour brings the page tours back too. */
export function resetPageTours(): void {
  try {
    for (const page of PAGE_TOUR_IDS) localStorage.removeItem(pageKey(page));
  } catch {
    // Nothing to reset when storage is blocked.
  }
}

/**
 * Waits for the page to draw (its data arrives a moment after it opens),
 * keeps the steps whose part is on screen, and runs them once.
 */
export function PageTour({ page }: { page: PageTourId }) {
  const [steps, setSteps] = useState<Step[] | null>(null);

  useEffect(() => {
    setSteps(null);
    const seen = seenSteps(page);
    const wanted = PAGE_STEPS[page].filter((s) => !seen.has(s.target));
    if (wanted.length === 0) return;
    let tries = 0;
    let last = -1;
    let steady = 0;
    // Every 200 ms: start once the parts on screen have stayed the same for
    // 0.6 s (the page has finished drawing), or after three seconds. An
    // empty day or an unopened sitting leaves a part missing for good, and
    // waiting the full three seconds for it meant the tour came too late to
    // be seen.
    const timer = window.setInterval(() => {
      tries += 1;
      const shown = wanted.filter((s) => isShown(s.target));
      steady = shown.length === last ? steady + 1 : 0;
      last = shown.length;
      if ((shown.length > 0 && steady >= 3) || tries >= 15) {
        window.clearInterval(timer);
        if (shown.length > 0) setSteps(shown);
      }
    }, 200);
    return () => window.clearInterval(timer);
  }, [page]);

  // Skipping counts as seen too: what was on screen is not offered again,
  // only a part that appears later.
  const finish = useCallback(() => {
    if (steps) {
      const seen = seenSteps(page);
      for (const s of steps) seen.add(s.target);
      try {
        localStorage.setItem(pageKey(page), JSON.stringify([...seen]));
      } catch {
        // Not remembering is harmless: it shows again next time.
      }
    }
    setSteps(null);
  }, [page, steps]);

  if (!steps) return null;
  return <Tour steps={steps} onFinish={finish} beside={false} />;
}
