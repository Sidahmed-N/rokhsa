/**
 * Which surface theme the app is wearing.
 *
 * A per-device display preference, not business data, so it lives in
 * localStorage rather than in the school's database: two people running the
 * app on two machines may reasonably want different answers, and nothing
 * about it belongs in a backup.
 *
 * Every read and write is wrapped, because localStorage throws in a private
 * window and comes back empty with site data cleared — and a theme that
 * cannot be read must never stop the app starting.
 */
export const SURFACES = ["solid", "glass"] as const;
export type Surface = (typeof SURFACES)[number];

const KEY = "tariqi.surface";

/** Solid is the honest default: it always runs well. */
export const DEFAULT_SURFACE: Surface = "solid";

export function currentSurface(): Surface {
  try {
    const stored = window.localStorage.getItem(KEY);
    return stored === "glass" || stored === "solid" ? stored : DEFAULT_SURFACE;
  } catch {
    return DEFAULT_SURFACE;
  }
}

export function applySurface(surface: Surface): void {
  document.documentElement.dataset["surface"] = surface;
  try {
    window.localStorage.setItem(KEY, surface);
  } catch {
    // Not being able to remember the choice is not a reason to refuse it.
  }
}

/**
 * Called once at startup, before the first paint.
 *
 * `?surface=glass` forces one theme for a single load without changing the
 * stored preference — the same escape hatch `?lang=ar` provides, and for the
 * same reason: these are things you have to LOOK at to check.
 */
export function initSurface(): void {
  const asked = new URLSearchParams(window.location.search).get("surface");
  const forced = asked === "glass" || asked === "solid" ? asked : null;
  document.documentElement.dataset["surface"] = forced ?? currentSurface();
}
