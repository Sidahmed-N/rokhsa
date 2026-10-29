/**
 * Enrolment, hours and payment API.
 *
 * Transport and types only. Every money figure here — total due, paid,
 * balance, extra-hour charge — is computed in Rust and sent whole. The
 * frontend must never add, subtract or round money. See CLAUDE.md.
 */

import { call } from "@/lib/tauri";
import type { Centimes } from "@/lib/money";

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
  /**
   * Exam fees — collected for the examiner, one per stage he has been
   * entered for. Kept apart from the school's own price because it was
   * never the school's money.
   */
  examFees: Centimes;
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
  /** Booked together with others when set; null means on their own. */
  sessionId: number | null;
  id: number;
  enrollmentId: number;
  instructorStaffId: number | null;
  kind: string;
  /** planned | done | missed | cancelled. Only `done` counts as hours. */
  status: string;
  scheduledOn: string | null;
  scheduledTime: string | null;
  occurredOn: string;
  minutes: number;
  /** What to work on, written when booking. */
  planNote: string;
  /** How it went, written when completing. */
  notes: string;
  createdAt: string;
}

/** A row of the Today screen. */
export interface DayEntry {
  id: number;
  /** The day this sits on, YYYY-MM-DD. Used to group the week view. */
  day: string;
  enrollmentId: number;
  candidateId: number;
  lastName: string;
  firstName: string;
  phone: string;
  categoryCode: string;
  kind: string;
  status: string;
  scheduledTime: string | null;
  minutes: number;
  planNote: string;
  notes: string;
  /**
   * Set when this candidate was booked together with others. Rows sharing a
   * value are one session on the calendar.
   */
  sessionId: number | null;
}

export interface ScheduleInput {
  kind: string;
  scheduledOn: string;
  scheduledTime: string;
  minutes: number;
  instructorStaffId: number | null;
  planNote: string;
}

export interface CompleteInput {
  /** Blank means the day it was planned for. */
  occurredOn: string;
  /** Zero means the duration it was planned for. */
  minutes: number;
  notes: string;
}

export interface NextUp {
  next: Lesson | null;
  lastNote: string;
}

export const LESSON_STATUSES = ["planned", "done", "missed", "cancelled"] as const;

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

export const listPaymentMethods = () => call<PaymentMethod[]>("list_payment_methods");
export const getSchool = () => call<School>("get_school");

export interface BookableEnrollment {
  enrollmentId: number;
  candidateId: number;
  lastName: string;
  firstName: string;
  categoryCode: string;
  status: string;
  /** Normalised name and phone — filter against this, never the display name. */
  searchText: string;
  /**
   * Passed all three exams. These are left out of the list the picker opens
   * on and only appear when their name is searched, so when one does appear
   * it has to say why.
   */
  finished: boolean;
}

export const listBookableEnrollments = (query: string) =>
  call<BookableEnrollment[]>("list_bookable_enrollments", { query });

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

/**
 * Book one slot for several candidates.
 *
 * A list of one is an ordinary booking and gets no session id — see
 * migration 0009 for why a group is not its own object.
 */
export const scheduleGroup = (enrollmentIds: number[], input: ScheduleInput) =>
  call<Lesson[]>("schedule_group", { enrollmentIds, input });

export const completeLesson = (id: number, input: CompleteInput) =>
  call<Lesson>("complete_lesson", { id, input });

export const rescheduleLesson = (
  id: number,
  scheduledOn: string,
  scheduledTime: string,
  minutes: number,
) => call<Lesson>("reschedule_lesson", { id, scheduledOn, scheduledTime, minutes });

export const setLessonNotHeld = (id: number, status: "missed" | "cancelled") =>
  call<Lesson>("set_lesson_not_held", { id, status });

/**
 * Sessions whose day has passed that nobody ever marked.
 *
 * Oldest first. These are the ones quietly holding a candidate’s hours - and
 * so their balance - at the wrong number.
 */
export const listOverdue = (today: string) =>
  call<DayEntry[]>("list_overdue_lessons", { today });

export const listDay = (date: string) => call<DayEntry[]>("list_day", { date });

export const listRange = (from: string, to: string) =>
  call<DayEntry[]>("list_range", { from, to });

export const nextUp = (enrollmentId: number) => call<NextUp>("next_up", { enrollmentId });

/** Today as YYYY-MM-DD, in the computer's own timezone. */
export function todayIso(): string {
  const d = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

/** Shift an ISO date by whole days, for the Today screen's arrows. */
export function shiftIso(iso: string, days: number): string {
  const [y, m, d] = iso.split("-").map(Number);
  const date = new Date(y ?? 1970, (m ?? 1) - 1, d ?? 1);
  date.setDate(date.getDate() + days);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

/**
 * The Saturday that opens the week `iso` falls in.
 *
 * Algerian auto-écoles work Saturday to Thursday and rest on Friday, so a week
 * that starts on Saturday puts the quiet day at the end where it belongs. A
 * Monday-first week would split the working week across two screens.
 */
export function startOfWeekIso(iso: string): string {
  const [y, m, d] = iso.split("-").map(Number);
  const date = new Date(y ?? 1970, (m ?? 1) - 1, d ?? 1);
  // getDay(): 0 = Sunday … 6 = Saturday. Days since the most recent Saturday.
  const since = (date.getDay() + 1) % 7;
  return shiftIso(iso, -since);
}

/** The seven ISO days of the week containing `iso`, Saturday first. */
export function weekDays(iso: string): string[] {
  const start = startOfWeekIso(iso);
  return Array.from({ length: 7 }, (_, i) => shiftIso(start, i));
}

export const listPayments = (enrollmentId: number) =>
  call<Payment[]>("list_payments", { enrollmentId });

export const getPayment = (id: number) => call<Payment>("get_payment", { id });

export const createPayment = (enrollmentId: number, input: PaymentInput) =>
  call<Payment>("create_payment", { enrollmentId, input });

export const reversePayment = (id: number, reason: string) =>
  call<Payment>("reverse_payment", { id, reason });

/**
 * Minutes to "2 h 30" / "45 min". Formatting only — no arithmetic on money.
 *
 * Prefer the `<Duration>` component over calling this directly.
 *
 * The Arabic form is deliberately different, not a translation of the French
 * one. "1 سا 30" puts a digit run on BOTH sides of a right-to-left word, and
 * the bidi algorithm then renders it right-to-left as 1, 30, سا — the hour
 * marker is stranded at the end and "1 h 30" reads as "1 30 h". Measured in
 * the WebView, not guessed. A single numeric run ("1:30") has nothing to
 * reorder, so it survives in either direction.
 */
export function formatDuration(
  minutes: number,
  hourLabel: string,
  minuteLabel: string,
  locale = "fr",
): string {
  const h = Math.trunc(minutes / 60);
  const m = minutes % 60;
  if (h === 0) return `${m} ${minuteLabel}`;
  if (m === 0) return `${h} ${hourLabel}`;
  if (locale === "ar") return `${h}:${String(m).padStart(2, "0")} ${hourLabel}`;
  return `${h} ${hourLabel} ${m}`;
}
