/**
 * Candidate API.
 *
 * Transport only. Validation, search folding and duplicate detection all run
 * in Rust — this file must never reimplement them, or the two will drift and
 * the browser copy will be the wrong one. See CLAUDE.md.
 */

import { call } from "@/lib/tauri";

export interface Candidate {
  id: number;
  nin: string | null;
  firstName: string;
  lastName: string;
  fatherName: string;
  motherName: string;
  birthDate: string | null;
  birthPlace: string;
  gender: string | null;
  bloodGroup: string;
  phone: string;
  phoneAlt: string;
  addressFr: string;
  addressAr: string;
  commune: string;
  wilayaCode: string;
  photoPath: string | null;
  status: string;
  notes: string;
  registeredAt: string;
  createdAt: string;
  updatedAt: string;
}

/** What the form sends. Every field is a string; Rust parses and validates. */
export interface CandidateInput {
  nin: string;
  firstName: string;
  lastName: string;
  fatherName: string;
  motherName: string;
  birthDate: string;
  birthPlace: string;
  gender: string;
  bloodGroup: string;
  phone: string;
  phoneAlt: string;
  addressFr: string;
  addressAr: string;
  commune: string;
  wilayaCode: string;
  status: string;
  notes: string;
}

export const CANDIDATE_STATUSES = [
  "prospect",
  "active",
  "inactive",
  "completed",
  "withdrawn",
] as const;

export function emptyCandidateInput(): CandidateInput {
  return {
    nin: "",
    firstName: "",
    lastName: "",
    fatherName: "",
    motherName: "",
    birthDate: "",
    birthPlace: "",
    gender: "",
    bloodGroup: "",
    phone: "",
    phoneAlt: "",
    addressFr: "",
    addressAr: "",
    commune: "",
    wilayaCode: "",
    status: "active",
    notes: "",
  };
}

export function toInput(candidate: Candidate): CandidateInput {
  return {
    nin: candidate.nin ?? "",
    firstName: candidate.firstName,
    lastName: candidate.lastName,
    fatherName: candidate.fatherName,
    motherName: candidate.motherName,
    birthDate: candidate.birthDate ?? "",
    birthPlace: candidate.birthPlace,
    gender: candidate.gender ?? "",
    bloodGroup: candidate.bloodGroup,
    phone: candidate.phone,
    phoneAlt: candidate.phoneAlt,
    addressFr: candidate.addressFr,
    addressAr: candidate.addressAr,
    commune: candidate.commune,
    wilayaCode: candidate.wilayaCode,
    status: candidate.status,
    notes: candidate.notes,
  };
}

export interface CandidatePage {
  items: Candidate[];
  total: number;
}

/** The shape Rust sends when a command fails. */
export interface AppErrorPayload {
  code: string;
  field: string | null;
  message: string;
}

export function isAppError(value: unknown): value is AppErrorPayload {
  return (
    typeof value === "object" &&
    value !== null &&
    "code" in value &&
    typeof (value as { code: unknown }).code === "string"
  );
}

export function listCandidates(query: string, limit = 50, offset = 0): Promise<CandidatePage> {
  return call<CandidatePage>("list_candidates", { query, limit, offset });
}

export function getCandidate(id: number): Promise<Candidate> {
  return call<Candidate>("get_candidate", { id });
}

export function createCandidate(input: CandidateInput): Promise<Candidate> {
  return call<Candidate>("create_candidate", { input });
}

export function updateCandidate(id: number, input: CandidateInput): Promise<Candidate> {
  return call<Candidate>("update_candidate", { id, input });
}

export function deleteCandidate(id: number): Promise<void> {
  return call<void>("delete_candidate", { id });
}

export function findCandidateDuplicates(
  input: CandidateInput,
  excludeId: number | null = null,
): Promise<Candidate[]> {
  return call<Candidate[]>("find_candidate_duplicates", { input, excludeId });
}

