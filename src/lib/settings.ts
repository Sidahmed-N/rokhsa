/**
 * School settings and price list.
 *
 * The price list is the school's own, not a regulatory table. Editing it never
 * changes an existing enrolment — those snapshot price and hour targets when
 * created. See CLAUDE.md and db/enrollments.rs.
 */

import { call } from "@/lib/tauri";
import type { Centimes } from "@/lib/money";

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

export interface Category {
  code: string;
  codeAr: string;
  nameFr: string;
  nameAr: string;
  theoryHoursTarget: number;
  practicalHoursTarget: number;
  defaultPriceCentimes: Centimes;
  isOffered: number;
  sortOrder: number;
}

export interface CategoryInput {
  codeAr: string;
  nameFr: string;
  nameAr: string;
  theoryHoursTarget: number;
  practicalHoursTarget: number;
  defaultPriceCentimes: Centimes;
  isOffered: boolean;
}

export const getSchoolSettings = () => call<SchoolSettings>("get_school_settings");

export const updateSchool = (input: School) => call<School>("update_school", { input });

export const listAllCategories = () => call<Category[]>("list_all_categories");

export const updateCategory = (code: string, input: CategoryInput) =>
  call<Category>("update_category", { code, input });
