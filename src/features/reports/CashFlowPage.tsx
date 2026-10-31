import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { SelectField, TextField } from "@/components/FormField";
import Modal from "@/components/Modal";
import { Pager, usePaged } from "@/components/ui/Pager";
import { Money, Num } from "@/components/Num";
import { currentLanguage } from "@/i18n";
import { isAppError } from "@/lib/candidates";
import { parseDZD } from "@/lib/money";
import {
  createExpense,
  deleteExpense,
  getCashFlow,
  lastMonth,
  listExpenseCategories,
  listExpenses,
  thisMonth,
  thisYear,
  type CashFlow,
  type Expense,
  type ExpenseCategory,
  type LineTotal,
} from "@/lib/reports";
import { isDesktop } from "@/lib/tauri";
import { todayIso } from "@/lib/training";

function Breakdown({
  title,
  lines,
  empty,
}: {
  title: string;
  lines: LineTotal[];
  empty: string;
}) {
  const locale = currentLanguage();
  return (
    <section className="rounded-lg border border-line bg-surface shadow-card">
      <h2 className="border-b border-line px-4 py-2 text-xs uppercase tracking-wide text-muted">
        {title}
      </h2>
      {lines.length === 0 ? (
        <p className="px-4 py-3 text-sm text-muted">{empty}</p>
      ) : (
        <table className="w-full text-sm">
          <tbody>
            {lines.map((line) => (
              <tr key={line.code} className="border-b border-line last:border-b-0">
                <td className="px-4 py-2">
                  {(locale === "ar" ? line.nameAr : line.nameFr) || line.code}
                </td>
                <td className="px-4 py-2 text-end">
                  <Money centimes={line.centimes} />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}

function AddExpenseDialog({
  defaultDate,
  onDone,
  onClose,
}: {
  defaultDate: string;
  onDone: () => void;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const locale = currentLanguage();

  const [categories, setCategories] = useState<ExpenseCategory[]>([]);
  const [categoryCode, setCategoryCode] = useState("fuel");
  const [amount, setAmount] = useState("");
  const [spentOn, setSpentOn] = useState(defaultDate);
  const [description, setDescription] = useState("");
  const [supplier, setSupplier] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    listExpenseCategories()
      .then(setCategories)
      .catch(() => setCategories([]));
  }, []);

  async function save() {
    const parsed = parseDZD(amount);
    if (parsed === null || parsed <= 0) {
      setError(t("expenses.error.amountPositive"));
      return;
    }
    setSaving(true);
    setError(null);
    try {
      await createExpense({
        categoryCode,
        amountCentimes: parsed,
        spentOn,
        description,
        supplier,
        vehicleLabel: "",
      });
      onDone();
    } catch (raw: unknown) {
      setError(isAppError(raw) ? t(raw.code, { defaultValue: raw.message }) : String(raw));
    } finally {
      setSaving(false);
    }
  }

  return (
    <Modal
      title={t("expenses.addTitle")}
      onClose={onClose}
      onSubmit={() => void save()}
      submitting={saving}
      error={error}
    >
      <div className="grid gap-3">
        <SelectField
          label={t("expenses.category")}
          value={categoryCode}
          onChange={setCategoryCode}
        >
          {categories.map((c) => (
            <option key={c.code} value={c.code}>
              {locale === "ar" ? c.nameAr : c.nameFr}
            </option>
          ))}
        </SelectField>
        <TextField
          label={t("expenses.amount")}
          value={amount}
          onChange={setAmount}
          dir="ltr"
          placeholder="3000"
        />
        <TextField
          label={t("expenses.date")}
          value={spentOn}
          onChange={setSpentOn}
          type="date"
          dir="ltr"
        />
        <TextField
          label={t("expenses.description")}
          value={description}
          onChange={setDescription}
        />
        <TextField label={t("expenses.supplier")} value={supplier} onChange={setSupplier} />
      </div>
    </Modal>
  );
}

/**
 * Money in and out over a period, with the expenses that make up "out".
 *
 * Kept on one screen on purpose: an owner wants to see the total and then
 * immediately record the fuel he just paid for, without navigating.
 */
/** Expenses per page. A busy month can hold more than fit on one screen. */
const EXPENSE_PAGE_SIZE = 50;

export default function CashFlowPage() {
  const { t } = useTranslation();
  const locale = currentLanguage();

  const [range, setRange] = useState(thisMonth());
  const [flow, setFlow] = useState<CashFlow | null>(null);
  const [expenses, setExpenses] = useState<Expense[]>([]);
  // A new period is a different list, so it opens on its first page. The
  // totals above are sums over the whole period and do not page.
  const { pageItems: expensePage, pager: expensePager } = usePaged(
    expenses,
    EXPENSE_PAGE_SIZE,
    `${range.from}|${range.to}`,
  );
  const [categories, setCategories] = useState<ExpenseCategory[]>([]);
  const [adding, setAdding] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    if (!isDesktop()) {
      setError(t("candidates.needsDesktop"));
      setLoading(false);
      return;
    }
    setLoading(true);
    try {
      const [f, e, c] = await Promise.all([
        getCashFlow(range.from, range.to),
        listExpenses(range.from, range.to),
        listExpenseCategories(),
      ]);
      setFlow(f);
      setExpenses(e);
      setCategories(c);
      setError(null);
    } catch (raw: unknown) {
      setError(isAppError(raw) ? t(raw.code, { defaultValue: raw.message }) : String(raw));
    } finally {
      setLoading(false);
    }
  }, [range, t]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const categoryName = (code: string) => {
    const found = categories.find((c) => c.code === code);
    if (!found) return code;
    return locale === "ar" ? found.nameAr : found.nameFr;
  };

  const positive = (flow?.netCentimes ?? 0) >= 0;

  return (
    <div>
      <h1 className="mb-5 text-[1.375rem] font-semibold tracking-tight text-ink">{t("cashFlow.title")}</h1>

      {error && (
        <p className="mb-3 rounded-md border border-owed/30 bg-owed-soft px-3 py-2 text-sm text-owed">
          {error}
        </p>
      )}

      <div className="mb-4 flex flex-wrap items-end gap-2">
        {[
          { label: t("cashFlow.thisMonth"), value: thisMonth() },
          { label: t("cashFlow.lastMonth"), value: lastMonth() },
          { label: t("cashFlow.thisYear"), value: thisYear() },
        ].map((preset) => (
          <button
            key={preset.label}
            type="button"
            onClick={() => setRange(preset.value)}
            className={[
              "rounded-md border px-3 py-1.5 text-sm",
              range.from === preset.value.from && range.to === preset.value.to
                ? "border-brand bg-brand-soft text-brand"
                : "border-line",
            ].join(" ")}
          >
            {preset.label}
          </button>
        ))}

        <div className="flex items-end gap-2">
          <TextField
            label={t("cashFlow.from")}
            value={range.from}
            onChange={(from) => setRange((r) => ({ ...r, from }))}
            type="date"
            dir="ltr"
          />
          <TextField
            label={t("cashFlow.to")}
            value={range.to}
            onChange={(to) => setRange((r) => ({ ...r, to }))}
            type="date"
            dir="ltr"
          />
        </div>
      </div>

      <section className="mb-5 grid gap-4 rounded-lg border border-line bg-surface shadow-card px-5 py-4 sm:grid-cols-3">
        <div>
          <p className="text-xs uppercase tracking-wide text-muted">{t("cashFlow.in")}</p>
          <Money centimes={flow?.inCentimes ?? 0} className="text-2xl font-bold text-ok" />
          <p className="mt-0.5 text-xs text-muted">
            <Num>{flow?.paymentCount ?? 0}</Num> {t("cashFlow.payments")}
          </p>
        </div>
        <div>
          <p className="text-xs uppercase tracking-wide text-muted">{t("cashFlow.out")}</p>
          <Money centimes={flow?.outCentimes ?? 0} className="text-2xl font-bold text-owed" />
          <p className="mt-0.5 text-xs text-muted">
            <Num>{flow?.expenseCount ?? 0}</Num> {t("cashFlow.expenses")}
          </p>
        </div>
        <div>
          <p className="text-xs uppercase tracking-wide text-muted">{t("cashFlow.net")}</p>
          <Money
            centimes={flow?.netCentimes ?? 0}
            className={["text-2xl font-bold", positive ? "text-ok" : "text-owed"].join(" ")}
          />
        </div>
      </section>

      {loading && <p className="mb-3 text-sm text-muted">{t("common.loading")}</p>}

      <div className="mb-5 grid gap-4 sm:grid-cols-2">
        <Breakdown
          title={t("cashFlow.byMethod")}
          lines={flow?.byMethod ?? []}
          empty={t("cashFlow.noPayments")}
        />
        <Breakdown
          title={t("cashFlow.byCategory")}
          lines={flow?.byExpenseCategory ?? []}
          empty={t("cashFlow.noExpenses")}
        />
      </div>

      <div className="mb-3 flex items-center gap-3">
        <h2 className="text-base font-semibold">{t("expenses.title")}</h2>
        <button
          type="button"
          onClick={() => setAdding(true)}
          className="inline-flex h-9 shrink-0 items-center justify-center gap-2 rounded-md bg-brand px-3.5 text-sm font-medium text-white shadow-card transition-colors hover:bg-brand-hover disabled:cursor-not-allowed disabled:bg-faint"
        >
          {t("expenses.add")}
        </button>
      </div>

      <div className="overflow-hidden rounded-lg border border-line bg-surface shadow-card">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-line text-xs uppercase tracking-wide text-muted">
              <th className="px-4 py-2 text-start font-medium">{t("expenses.date")}</th>
              <th className="px-4 py-2 text-start font-medium">{t("expenses.category")}</th>
              <th className="px-4 py-2 text-start font-medium">{t("expenses.description")}</th>
              <th className="px-4 py-2 text-end font-medium">{t("expenses.amount")}</th>
              <th className="px-4 py-2 text-end font-medium" />
            </tr>
          </thead>
          <tbody>
            {expenses.length === 0 && !loading && (
              <tr>
                <td colSpan={5} className="px-4 py-6 text-center text-sm text-muted">
                  {t("expenses.none")}
                </td>
              </tr>
            )}
            {expensePage.map((expense) => (
              <tr key={expense.id} className="border-b border-line last:border-b-0">
                <td className="px-4 py-2">
                  <Num>{expense.spentOn}</Num>
                </td>
                <td className="px-4 py-2">{categoryName(expense.categoryCode)}</td>
                <td className="px-4 py-2">
                  {expense.description || <span className="text-muted">—</span>}
                  {expense.supplier && (
                    <span className="ms-2 text-xs text-muted">{expense.supplier}</span>
                  )}
                </td>
                <td className="px-4 py-2 text-end">
                  <Money centimes={expense.amountCentimes} />
                </td>
                <td className="px-4 py-2 text-end">
                  <button
                    type="button"
                    onClick={() =>
                      void deleteExpense(expense.id)
                        .then(refresh)
                        .catch((e: unknown) =>
                          setError(
                            isAppError(e) ? t(e.code, { defaultValue: e.message }) : String(e),
                          ),
                        )
                    }
                    className="text-xs text-owed hover:underline"
                  >
                    {t("common.delete")}
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <Pager {...expensePager} />

      {adding && (
        <AddExpenseDialog
          // Today, unless the period shown is entirely in the past. The end of
          // the period opened on the last day of the month, which the date
          // check refused as "in the future" for most of the month.
          defaultDate={range.to < todayIso() ? range.to : todayIso()}
          onClose={() => setAdding(false)}
          onDone={() => {
            setAdding(false);
            void refresh();
          }}
        />
      )}
    </div>
  );
}
