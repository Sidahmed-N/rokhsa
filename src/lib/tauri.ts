/**
 * Bridge to the Rust layer.
 *
 * The frontend runs in two places: inside the Tauri desktop shell (the real
 * product) and in a plain browser (convenient while building screens). This
 * module is the only place that knows the difference.
 *
 * It is a TRANSPORT, not a place for logic. Business rules — balances, prices,
 * eligibility — live in Rust. See CLAUDE.md.
 */

import { invoke } from "@tauri-apps/api/core";

/**
 * True when running inside the desktop shell.
 *
 * Tauri injects `__TAURI_INTERNALS__` into the window before any app code
 * runs, so this is reliable at module scope as well as in a component.
 */
export function isDesktop(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/** Raised when a command is called in a browser, where there is no Rust side. */
export class NotInDesktopError extends Error {
  constructor(command: string) {
    super(`Command "${command}" needs the desktop app; it is not available in a browser.`);
    this.name = "NotInDesktopError";
  }
}

/**
 * Call a Rust command.
 *
 * Deliberately throws rather than returning a fake value in the browser: a
 * silent stub would let business logic drift into the frontend, which is the
 * one thing the architecture is trying to prevent.
 */
export async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isDesktop()) throw new NotInDesktopError(command);
  return invoke<T>(command, args);
}

/* -------------------------------------------------------------------------
   Commands
   ------------------------------------------------------------------------- */

/**
 * The app version reported by Rust.
 *
 * Doubles as the smoke test that the Rust↔JS bridge is really wired up: if
 * this returns, the whole chain works.
 */
export function appVersion(): Promise<string> {
  return call<string>("app_version");
}
