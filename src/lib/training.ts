/**
 * Enrolment, hours and payment API.
 *
 * Transport and types only. Every money figure here — total due, paid,
 * balance, extra-hour charge — is computed in Rust and sent whole. The
 * frontend must never add, subtract or round money. See CLAUDE.md.
 */

import { call } from "@/lib/tauri";
import type { Centimes } from "@/lib/money";

export interface LicenceCategory {
  code: string;
  codeAr: string;
  nameFr: string;
  nameAr: string;
  theoryHoursTarget: number;
  practicalHoursTarget: number;
  defaultPriceCentimes: Centimes;
}

export interface PaymentMethod {
  code: string;
  nameFr: string;
  nameAr: string;
  isCash: number;
  requiresReference: number;
}

export interface Enrollment {
  id: number;
  candidateId: number;
  categoryCode: string;
  enrolledOn: string;
  agreedPriceCentimes: Centimes;
  extraHourPriceCentimes: Centimes;
  theoryHoursTarget: number;
  practicalHoursTarget: number;
  discountCentimes: Centimes;
  discountReason: string;
  status: string;
  notes: string;
  createdAt: string;
  updatedAt: string;
}

export interface Progress {
  doneMinutes: number;
  targetMinutes: number;
  remainingMinutes: number;
  /** null when no target is set — showing 100% of nothing would be a lie. */
  percent: number | null;
}

export interface Charges {
  agreedPrice: Centimes;
  discount: Centimes;
  extraHours: Centimes;
}

export interface MinutesByKind {
  theory: number;
  maneuvers: number;
  road: number;
}

export interface EnrollmentSummary {
  enrollment: Enrollment;
  categoryNameFr: string;
  categoryNameAr: string;
  theory: Progress;
  practical: Progress;
  minutes: MinutesByKind;
  charges: Charges;
  totalDueCentimes: Centimes;
  paidCentimes: Centimes;
  balanceCentimes: Centimes;
}

export interface Lesson {
  id: number;
  enrollmentId: number;
  instructorStaffId: number | null;
  kind: string;
  occurredOn: string;
  minutes: number;
  notes: string;
  createdAt: string;
}

export interface Payment {
  id: number;
  enrollmentId: number;
  amountCentimes: Centimes;
  paidOn: string;
  methodCode: string;
  reference: string;
  receiptNumber: string;
  reversesPaymentId: number | null;
  notes: string;
  createdAt: string;
}

export interface School {
  nameFr: string;
  nameAr: string;
  addressFr: string;
  addressAr: string;
  phone: string;
  agrementNumber: string;
}

export const LESSON_KINDS = ["theory", "maneuvers", "road"] as const;
export const ENROLLMENT_STATUSES = [
  "forming",
  "ready",
  "presented",
  "passed",
  "failed",
  "withdrawn",
] as const;

export interface EnrollmentInput {
  categoryCode: string;
  enrolledOn: string;
  agreedPriceCentimes: Centimes;
  extraHourPriceCentimes: Centimes;
  theoryHoursTarget: number;
  practicalHoursTarget: number;
  discountCentimes: Centimes;
  discountReason: string;
  status: string;
  notes: string;
}

export interface LessonInput {
  kind: string;
  occurredOn: string;
  minutes: number;
  instructorStaffId: number | null;
  notes: string;
}

export interface PaymentInput {
  amountCentimes: Centimes;
  paidOn: string;
  methodCode: string;
  reference: string;
  notes: string;
}

export const listCategories = () => call<LicenceCategory[]>("list_categories");
export const listPaymentMethods = () => call<PaymentMethod[]>("list_payment_methods");
export const getSchool = () => call<School>("get_school");

export const listEnrollments = (candidateId: number) =>
  call<EnrollmentSummary[]>("list_enrollments", { candidateId });

export const getEnrollment = (id: number) =>
  call<EnrollmentSummary>("get_enrollment", { id });

export const createEnrollment = (candidateId: number, input: EnrollmentInput) =>
  call<Enrollment>("create_enrollment", { candidateId, input });

export const deleteEnrollment = (id: number) => call<void>("delete_enrollment", { id });

export const listLessons = (enrollmentId: number) =>
  call<Lesson[]>("list_lessons", { enrollmentId });

export const createLesson = (enrollmentId: number, input: LessonInput) =>
  call<Lesson>("create_lesson", { enrollmentId, input });

export const deleteLesson = (id: number) => call<void>("delete_lesson", { id });

export const listPayments = (enrollmentId: number) =>
  call<Payment[]>("list_payments", { enrollmentId });

export const getPayment = (id: number) => call<Payment>("get_payment", { id });

export const createPayment = (enrollmentId: number, input: PaymentInput) =>
  call<Payment>("create_payment", { enrollmentId, input });

export const reversePayment = (id: number, reason: string) =>
  call<Payment>("reverse_payment", { id, reason });

/** Minutes to "2 h 30" / "45 min". Formatting only — no arithmetic on money. */
export function formatDuration(minutes: number, hourLabel: string, minuteLabel: string): string {
  const h = Math.trunc(minutes / 60);
  const m = minutes % 60;
  if (h === 0) return `${m} ${minuteLabel}`;
  if (m === 0) return `${h} ${hourLabel}`;
  return `${h} ${hourLabel} ${m}`;
}
