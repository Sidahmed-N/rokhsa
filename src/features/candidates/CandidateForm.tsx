import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import FreeText from "@/components/FreeText";
import { Num } from "@/components/Num";
import {
  CANDIDATE_STATUSES,
  createCandidate,
  findCandidateDuplicates,
  isAppError,
  updateCandidate,
  type Candidate,
  type CandidateInput,
} from "@/lib/candidates";
import { personName } from "@/lib/names";

interface Props {
  initial: CandidateInput;
  editingId: number | null;
  onSaved: (candidate: Candidate) => void;
  onCancel: () => void;
}

/** One labelled input. */
function Field({
  label,
  value,
  onChange,
  error,
  dir,
  type = "text",
  placeholder,
}: {
  label: string;
  value: string;
  onChange: (next: string) => void;
  error?: string | undefined;
  dir?: "ltr" | "rtl" | "auto" | undefined;
  type?: string;
  placeholder?: string | undefined;
}) {
  return (
    <label className="block">
      <span className="mb-1 block text-xs text-muted">{label}</span>
      <input
        type={type}
        value={value}
        dir={dir}
        placeholder={placeholder}
        onChange={(e) => onChange(e.target.value)}
        className={[
          "w-full rounded-md border bg-surface px-3 py-2 text-sm outline-none",
          "focus:border-brand focus:ring-2 focus:ring-brand-soft",
          error ? "border-owed" : "border-line",
        ].join(" ")}
      />
      {error && <span className="mt-1 block text-xs text-owed">{error}</span>}
    </label>
  );
}

export default function CandidateForm({ initial, editingId, onSaved, onCancel }: Props) {
  const { t } = useTranslation();

  const [form, setForm] = useState<CandidateInput>(initial);
  const [saving, setSaving] = useState(false);
  const [errorCode, setErrorCode] = useState<string | null>(null);
  const [errorField, setErrorField] = useState<string | null>(null);
  const [duplicates, setDuplicates] = useState<Candidate[]>([]);

  const set = <K extends keyof CandidateInput>(key: K, value: CandidateInput[K]) =>
    setForm((prev) => ({ ...prev, [key]: value }));

  // Look for likely duplicates as the name is typed. A WARNING only — two
  // people genuinely can share a name, and blocking the second one would push
  // the secretary back to paper.
  const nameKey = `${form.lastName}|${form.firstName}|${form.nin}`;
  const latestRequest = useRef(0);

  useEffect(() => {
    if (!form.lastName && !form.nin) {
      setDuplicates([]);
      return;
    }
    const token = ++latestRequest.current;
    const timer = setTimeout(() => {
      findCandidateDuplicates(form, editingId)
        .then((found) => {
          // Ignore a slow response that a newer keystroke has superseded.
          if (token === latestRequest.current) setDuplicates(found);
        })
        .catch(() => {
          if (token === latestRequest.current) setDuplicates([]);
        });
    }, 350);
    return () => clearTimeout(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [nameKey, editingId]);

  const fieldError = (field: string): string | undefined =>
    errorField === field && errorCode ? t(errorCode, { defaultValue: errorCode }) : undefined;

  async function save() {
    setSaving(true);
    setErrorCode(null);
    setErrorField(null);
    try {
      const saved = editingId
        ? await updateCandidate(editingId, form)
        : await createCandidate(form);
      onSaved(saved);
    } catch (raw: unknown) {
      if (isAppError(raw)) {
        setErrorCode(raw.code);
        setErrorField(raw.field);
      } else {
        setErrorCode("error.internal");
      }
    } finally {
      setSaving(false);
    }
  }

  const generalError =
    errorCode && !errorField ? t(errorCode, { defaultValue: errorCode }) : null;

  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center overflow-y-auto bg-black/40 p-6">
      <div className="w-full max-w-3xl rounded-lg border border-line bg-surface shadow-card shadow-lg">
        <header className="border-b border-line px-5 py-3">
          <h2 className="text-base font-semibold">
            {editingId ? t("candidates.editTitle") : t("candidates.newTitle")}
          </h2>
        </header>

        <div className="max-h-[70vh] overflow-y-auto px-5 py-4">
          {generalError && (
            <p className="mb-4 rounded-md border border-owed/30 bg-owed-soft px-3 py-2 text-sm text-owed">
              {generalError}
            </p>
          )}

          {duplicates.length > 0 && (
            <div className="mb-4 rounded-md border border-warn bg-surface px-3 py-2">
              <p className="text-sm font-medium text-warn">
                {t("candidates.duplicateWarning", { n: duplicates.length })}
              </p>
              <ul className="mt-1 text-xs text-muted">
                {duplicates.map((d) => (
                  <li key={d.id}>
                    <FreeText as="span">{personName(d)}</FreeText>
                    {d.phone && (
                      <>
                        {" — "}
                        <Num>{d.phone}</Num>
                      </>
                    )}
                  </li>
                ))}
              </ul>
              <p className="mt-1 text-xs text-muted">{t("candidates.duplicateHint")}</p>
            </div>
          )}

          <fieldset className="mb-5">
            <legend className="mb-2 text-xs uppercase tracking-wide text-muted">
              {t("candidates.sectionIdentity")}
            </legend>
            <div className="grid gap-3 sm:grid-cols-2">
              <Field
                label={t("candidates.lastName")}
                value={form.lastName}
                onChange={(v) => set("lastName", v)}
                error={fieldError("lastName")}
                dir="auto"
              />
              <Field
                label={t("candidates.firstName")}
                value={form.firstName}
                onChange={(v) => set("firstName", v)}
                dir="auto"
              />
            </div>
          </fieldset>

          <fieldset className="mb-5">
            <legend className="mb-2 text-xs uppercase tracking-wide text-muted">
              {t("candidates.sectionAdmin")}
            </legend>
            <div className="grid gap-3 sm:grid-cols-2">
              <Field
                label={t("candidates.nin")}
                value={form.nin}
                onChange={(v) => set("nin", v)}
                error={fieldError("nin")}
                dir="ltr"
                placeholder={t("candidates.ninPlaceholder")}
              />
              <Field
                label={t("candidates.birthDate")}
                value={form.birthDate}
                onChange={(v) => set("birthDate", v)}
                error={fieldError("birthDate")}
                type="date"
                dir="ltr"
              />
              <Field
                label={t("candidates.birthPlace")}
                value={form.birthPlace}
                onChange={(v) => set("birthPlace", v)}
                dir="auto"
              />

              <label className="block">
                <span className="mb-1 block text-xs text-muted">{t("candidates.gender")}</span>
                <select
                  value={form.gender}
                  onChange={(e) => set("gender", e.target.value)}
                  className="w-full rounded-md border border-line bg-surface px-3 py-2 text-sm outline-none focus:border-brand"
                >
                  <option value="">{t("common.none")}</option>
                  <option value="m">{t("candidates.genderMale")}</option>
                  <option value="f">{t("candidates.genderFemale")}</option>
                </select>
              </label>
            </div>
          </fieldset>

          <fieldset className="mb-5">
            <legend className="mb-2 text-xs uppercase tracking-wide text-muted">
              {t("candidates.sectionContact")}
            </legend>
            <div className="grid gap-3 sm:grid-cols-2">
              <Field
                label={t("candidates.phone")}
                value={form.phone}
                onChange={(v) => set("phone", v)}
                error={fieldError("phone")}
                dir="ltr"
                placeholder="0551 23 45 67"
              />
              <Field
                label={t("candidates.phoneAlt")}
                value={form.phoneAlt}
                onChange={(v) => set("phoneAlt", v)}
                error={fieldError("phoneAlt")}
                dir="ltr"
              />
              <Field
                label={t("candidates.commune")}
                value={form.commune}
                onChange={(v) => set("commune", v)}
              />
              <label className="block">
                <span className="mb-1 block text-xs text-muted">{t("candidates.status")}</span>
                <select
                  value={form.status}
                  onChange={(e) => set("status", e.target.value)}
                  className="w-full rounded-md border border-line bg-surface px-3 py-2 text-sm outline-none focus:border-brand"
                >
                  {CANDIDATE_STATUSES.map((status) => (
                    <option key={status} value={status}>
                      {t(`candidates.status_${status}`)}
                    </option>
                  ))}
                </select>
              </label>
            </div>
          </fieldset>

          <label className="block">
            <span className="mb-1 block text-xs text-muted">{t("candidates.notes")}</span>
            <textarea
              value={form.notes}
              onChange={(e) => set("notes", e.target.value)}
              rows={2}
              className="w-full rounded-md border border-line bg-surface px-3 py-2 text-sm outline-none focus:border-brand"
            />
          </label>
        </div>

        <footer className="flex items-center gap-2 border-t border-line px-5 py-3">
          <button
            type="button"
            onClick={() => void save()}
            disabled={saving}
            className="rounded-md bg-brand px-4 py-2 text-sm font-medium text-white disabled:opacity-50"
          >
            {saving ? t("common.loading") : t("common.save")}
          </button>
          <button
            type="button"
            onClick={onCancel}
            className="rounded-md border border-line px-4 py-2 text-sm"
          >
            {t("common.cancel")}
          </button>
          <span className="ms-auto text-xs text-muted">{t("candidates.requiredHint")}</span>
        </footer>
      </div>
    </div>
  );
}
