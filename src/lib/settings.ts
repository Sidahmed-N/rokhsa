/**
 * School settings and price list.
 *
 * The price list is the school's own, not a regulatory table. Editing it never
 * changes an existing enrolment — those snapshot price and hour targets when
 * created. See CLAUDE.md and db/enrollments.rs.
 */

import { call } from "@/lib/tauri";

export interface School {
  nameFr: string;
  nameAr: string;
  addressFr: string;
  addressAr: string;
  commune: string;
  wilayaCode: string;
  phone: string;
  email: string;
  agrementNumber: string;
  defaultLanguage: string;
  receiptPrefix: string;
  nextReceiptNumber: number;
}

export interface SchoolSettings extends School {
  /** True once any payment exists; the counter cannot move after that. */
  receiptCounterLocked: boolean;
}

export const getSchoolSettings = () => call<SchoolSettings>("get_school_settings");

export const updateSchool = (input: School) => call<School>("update_school", { input });

