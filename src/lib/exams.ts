/**
 * Exam sittings and results.
 *
 * Transport only. Which stage a candidate is at is worked out in Rust from
 * their results and arrives here as `ExamProgress` — the frontend must never
 * derive it, or there are two answers to the same question.
 */
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

/** What a drag records. `kind` is the exam it is about. */
export type BoardMove = {
  action: "present" | "pass" | "resultPassed" | "resultFailed" | "undoPass";
  kind: string;
};

export const getExamBoard = () => call<BoardCard[]>("get_exam_board");

/** What dropping a card on `to` would record — nothing is written. */
export const planBoardMove = (enrollmentId: number, to: BoardColumn) =>
  call<BoardMove>("plan_board_move", { enrollmentId, to });

/** Record it. Rust plans the move again inside the write. */
export const moveOnBoard = (enrollmentId: number, to: BoardColumn, date: string) =>
  call<BoardMove>("move_on_board", { enrollmentId, to, date });

/**
 * Take back a presentation made by mistake on the board — instead of the
 * failure that dragging the card back would record. Refused for one from a
 * real sitting, which is removed on the Examens screen.
 */
export const withdrawBoardPresentation = (enrollmentId: number) =>
  call<void>("withdraw_board_presentation", { enrollmentId });

export const listExamSessions = () => call<ExamSession[]>("list_exam_sessions");

export const createExamSession = (input: ExamSessionInput) =>
  call<ExamSession>("create_exam_session", { input });

export const deleteExamSession = (id: number) => call<void>("delete_exam_session", { id });

export const listExamEntries = (sessionId: number) =>
  call<ExamEntry[]>("list_exam_entries", { sessionId });

/** `feeCentimes` is what the owner typed; 0 means he pays nothing this time. */
export const addToExamSession = (
  sessionId: number,
  enrollmentId: number,
  feeCentimes: number,
) => call<ExamEntry>("add_to_exam_session", { sessionId, enrollmentId, feeCentimes });

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
