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
  lastNameFr: string;
  firstNameFr: string;
  lastNameAr: string;
  firstNameAr: string;
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

export const listExamSessions = () => call<ExamSession[]>("list_exam_sessions");

export const createExamSession = (input: ExamSessionInput) =>
  call<ExamSession>("create_exam_session", { input });

export const deleteExamSession = (id: number) => call<void>("delete_exam_session", { id });

export const listExamEntries = (sessionId: number) =>
  call<ExamEntry[]>("list_exam_entries", { sessionId });

export const addToExamSession = (sessionId: number, enrollmentId: number) =>
  call<ExamEntry>("add_to_exam_session", { sessionId, enrollmentId });

export const setExamOutcome = (resultId: number, outcome: string, notes: string) =>
  call<void>("set_exam_outcome", { resultId, outcome, notes });

export const removeExamResult = (resultId: number) =>
  call<void>("remove_exam_result", { resultId });

export const listExamResults = (enrollmentId: number) =>
  call<ExamResult[]>("list_exam_results", { enrollmentId });

export const getExamProgress = (enrollmentId: number) =>
  call<ExamProgress>("get_exam_progress", { enrollmentId });
