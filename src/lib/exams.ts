/**
 * Exam sittings and results.
 *
 * Transport only. Which stage a candidate is at is worked out in Rust from
 * their results and arrives here as `ExamProgress` — the frontend must never
 * derive it, or there are two answers to the same question.
 */
import type { Centimes } from "@/lib/money";
import { call } from "@/lib/tauri";

/** The three exams, in the order they are taken. */
export const EXAM_KINDS = ["theory", "maneuvers", "road"] as const;
export type ExamKind = (typeof EXAM_KINDS)[number];

/** `presented` means the batch exists and the result is not back yet. */
export const OUTCOMES = ["presented", "passed", "failed", "absent"] as const;
export type Outcome = (typeof OUTCOMES)[number];

export type Standing = "notSat" | "awaiting" | "failed" | "passed";

export interface ExamSession {
  id: number;
  kind: string;
  heldOn: string;
  /** "HH:MM", or empty — the school is not always given a time. */
  heldAt: string;
  place: string;
  notes: string;
  createdAt: string;
  updatedAt: string;
}

export interface ExamSessionInput {
  kind: string;
  heldOn: string;
  heldAt: string;
  place: string;
  notes: string;
}

/** A candidate in a sitting, with enough to print the list. */
export interface ExamEntry {
  id: number;
  enrollmentId: number;
  candidateId: number;
  lastName: string;
  firstName: string;
  nin: string;
  categoryCode: string;
  kind: string;
  satOn: string;
  outcome: string;
  notes: string;
  /** 2 means a repassage. */
  attempt: number;
  /** null when he pays nothing for this sitting. */
  feeChargeId: number | null;
  feeCentimes: Centimes;
  feePaidCentimes: Centimes;
  /** What is left to pay of the fee, worked out in Rust. */
  feeDueCentimes: Centimes;
}

/** Who has paid the exam fee in one sitting. */
export interface SessionFees {
  withFee: number;
  paid: number;
  missingCentimes: Centimes;
}

export interface ExamResult {
  id: number;
  enrollmentId: number;
  sessionId: number | null;
  kind: string;
  satOn: string;
  outcome: string;
  notes: string;
  place: string;
  attempt: number;
}

export interface ExamProgress {
  theory: Standing;
  maneuvers: Standing;
  road: Standing;
  theoryAttempts: number;
  maneuversAttempts: number;
  roadAttempts: number;
  /** What the school would normally present them for next, null when done. */
  nextDue: string | null;
  finished: boolean;
}

/** One card on the progress board. A card is an ENROLMENT, not a candidate. */
export interface BoardCard {
  enrollmentId: number;
  candidateId: number;
  lastName: string;
  firstName: string;
  phone: string;
  categoryCode: string;
  /** How a withdrawal is known — the results cannot imply one. */
  enrollmentStatus: string;
  progress: ExamProgress;
  daysSinceExam: number | null;
  /** Decided in Rust: a drag between columns records something, and what
   *  it records depends on which column the card is in. */
  column: BoardColumn;
}

export type BoardColumn =
  | "theory"
  | "maneuvers"
  | "road"
  | "awaiting"
  | "finished"
  | "withdrawn";

export const getExamBoard = () => call<BoardCard[]>("get_exam_board");

export const listExamSessions = () => call<ExamSession[]>("list_exam_sessions");

export const createExamSession = (input: ExamSessionInput) =>
  call<ExamSession>("create_exam_session", { input });

/** Correct when and where; the exam itself cannot change. Rust moves the
 * entries with the date, and refuses a new date once a result is noted. */
export const updateExamSession = (
  id: number,
  change: { heldOn: string; heldAt: string; place: string },
) => call<ExamSession>("update_exam_session", { id, change });

export const deleteExamSession = (id: number) => call<void>("delete_exam_session", { id });

/** The next sitting from today on, for the box on Accueil. */
export interface NextExam {
  session: ExamSession;
  candidates: number;
  fees: SessionFees;
}

/** `now` is the PC's local "HH:MM": a sitting today drops out once its time passes. */
export const nextExamSession = (today: string, now: string) =>
  call<NextExam | null>("next_exam_session", { today, now });

export const examSessionFees = (sessionId: number) =>
  call<SessionFees>("exam_session_fees", { sessionId });

export const listExamEntries = (sessionId: number) =>
  call<ExamEntry[]>("list_exam_entries", { sessionId });

/** `feeCentimes` is what the owner typed; 0 means he pays nothing this time. */
/** Several at once, each charged `feeCentimes`: all are added or none. */
export const addToExamSession = (
  sessionId: number,
  enrollmentIds: number[],
  feeCentimes: number,
) => call<ExamEntry[]>("add_to_exam_session", { sessionId, enrollmentIds, feeCentimes });

export const setExamOutcome = (resultId: number, outcome: string, notes: string) =>
  call<void>("set_exam_outcome", { resultId, outcome, notes });

export const removeExamResult = (resultId: number) =>
  call<void>("remove_exam_result", { resultId });

export const listExamResults = (enrollmentId: number) =>
  call<ExamResult[]>("list_exam_results", { enrollmentId });

export const getExamProgress = (enrollmentId: number) =>
  call<ExamProgress>("get_exam_progress", { enrollmentId });

/**
 * Exam fees.
 *
 * Typed when the candidate is entered for a sitting — there is no school-wide
 * amount, because there is no school-wide practice. Some schools charge the
 * repassage again and some do not. See docs/05-OPEN-QUESTIONS.md Q16.
 */
export interface ExamFeeCharge {
  id: number;
  enrollmentId: number;
  /** The entry this fee was raised for. */
  resultId: number | null;
  kind: string;
  amountCentimes: number;
  chargedOn: string;
}

export const listExamFeeCharges = (enrollmentId: number) =>
  call<ExamFeeCharge[]>("list_exam_fee_charges", { enrollmentId });

/** A fee not yet fully paid, offered by the payment window on the file and Créances. */
export interface UnpaidFee {
  chargeId: number;
  kind: string;
  chargedOn: string;
  dueCentimes: number;
}

export const listUnpaidExamFees = (enrollmentId: number) =>
  call<UnpaidFee[]>("list_unpaid_exam_fees", { enrollmentId });
