/**
 * Receivables, cash flow and expenses.
 *
 * Every total here is computed in Rust from the same `domain::money` functions
 * the candidate screen uses. The frontend must never re-derive a balance, or
 * the two screens will eventually disagree — and the moment they disagree the
 * owner stops trusting both.
 */

import { call } from "@/lib/tauri";
import type { Centimes } from "@/lib/money";

export interface ReceivableRow {
  enrollmentId: number;
  candidateId: number;
  lastNameFr: string;
  firstNameFr: string;
  lastNameAr: string;
  firstNameAr: string;
  phone: string;
  categoryCode: string;
  enrolledOn: string;
  status: string;
  totalDueCentimes: Centimes;
  paidCentimes: Centimes;
  balanceCentimes: Centimes;
  lastPaymentOn: string | null;
  daysSincePayment: number | null;
}

export interface Receivables {
  rows: ReceivableRow[];
  totalOwedCentimes: Centimes;
  enrollmentsOwing: number;
}

export interface LineTotal {
  code: string;
  nameFr: string;
  nameAr: string;
  centimes: Centimes;
}

export interface CashFlow {
  from: string;
  to: string;
  inCentimes: Centimes;
  outCentimes: Centimes;
  netCentimes: Centimes;
  byMethod: LineTotal[];
  byExpenseCategory: LineTotal[];
  paymentCount: number;
  expenseCount: number;
}

export interface Expense {
  id: number;
  categoryCode: string;
  amountCentimes: Centimes;
  spentOn: string;
  description: string;
  supplier: string;
  vehicleLabel: string;
  createdAt: string;
}

export interface ExpenseCategory {
  code: string;
  nameFr: string;
  nameAr: string;
}

export interface ExpenseInput {
  categoryCode: string;
  amountCentimes: Centimes;
  spentOn: string;
  description: string;
  supplier: string;
  vehicleLabel: string;
}

export interface DayTotal {
  day: string;
  inCentimes: Centimes;
  outCentimes: Centimes;
}

export const getCashFlowDaily = (from: string, to: string) =>
  call<DayTotal[]>("get_cash_flow_daily", { from, to });

/**
 * The same span, one period earlier — for "vs. last month" on the dashboard.
 *
 * Returns the range only; the totals come from the same get_cash_flow the
 * current period uses, so there is no second definition of what "money in"
 * means that could drift from the first.
 */
export function previousPeriod(range: { from: string; to: string }): { from: string; to: string } {
  const start = new Date(range.from);
  const end = new Date(range.to);
  const days = Math.round((end.getTime() - start.getTime()) / 86_400_000) + 1;
  const shift = (d: Date) => {
    const out = new Date(d);
    out.setDate(out.getDate() - days);
    const pad = (n: number) => String(n).padStart(2, "0");
    return `${out.getFullYear()}-${pad(out.getMonth() + 1)}-${pad(out.getDate())}`;
  };
  return { from: shift(start), to: shift(end) };
}

export const getReceivables = () => call<Receivables>("get_receivables");

export const getCashFlow = (from: string, to: string) =>
  call<CashFlow>("get_cash_flow", { from, to });

export const listExpenseCategories = () =>
  call<ExpenseCategory[]>("list_expense_categories");

export const listExpenses = (from: string, to: string) =>
  call<Expense[]>("list_expenses", { from, to });

export const createExpense = (input: ExpenseInput) =>
  call<Expense>("create_expense", { input });

export const deleteExpense = (id: number) => call<void>("delete_expense", { id });

/* ------------------------------ date ranges ----------------------------- */

function iso(date: Date): string {
  const m = String(date.getMonth() + 1).padStart(2, "0");
  const d = String(date.getDate()).padStart(2, "0");
  return `${date.getFullYear()}-${m}-${d}`;
}

export interface DateRange {
  from: string;
  to: string;
}

export function thisMonth(): DateRange {
  const now = new Date();
  return {
    from: iso(new Date(now.getFullYear(), now.getMonth(), 1)),
    to: iso(new Date(now.getFullYear(), now.getMonth() + 1, 0)),
  };
}

export function lastMonth(): DateRange {
  const now = new Date();
  return {
    from: iso(new Date(now.getFullYear(), now.getMonth() - 1, 1)),
    to: iso(new Date(now.getFullYear(), now.getMonth(), 0)),
  };
}

export function thisYear(): DateRange {
  const now = new Date();
  return {
    from: iso(new Date(now.getFullYear(), 0, 1)),
    to: iso(new Date(now.getFullYear(), 11, 31)),
  };
}
