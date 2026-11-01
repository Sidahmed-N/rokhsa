/**
 * The one-code lock. Checked and enforced in Rust (src-tauri/src/license);
 * the screen only asks for the code and says what state the app is in.
 */

import { call } from "@/lib/tauri";

export interface LicenseStatus {
  /** Whether this build has a lock at all (a development build may not). */
  required: boolean;
  unlocked: boolean;
}

export const licenseStatus = () => call<LicenseStatus>("license_status");
export const unlockApp = (code: string) => call<LicenseStatus>("unlock_app", { code });
