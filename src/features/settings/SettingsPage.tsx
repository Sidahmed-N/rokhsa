import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { SelectField, TextField } from "@/components/FormField";
import { isAppError } from "@/lib/candidates";
import {
  getSchoolSettings,
  updateSchool,
  type SchoolSettings,
} from "@/lib/settings";
import { isDesktop } from "@/lib/tauri";
import BackupSection from "@/features/settings/BackupSection";

/**
 * School settings.
 *
 * Identity and backups. Nothing else: a school that has to choose how the
 * software behaves is a school being asked to do our job. The price list went
 * when the school turned out to teach one licence — the price is agreed per
 * candidate and typed on the enrolment.
 */
export default function SettingsPage() {
  const { t } = useTranslation();

  const [school, setSchool] = useState<SchoolSettings | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const [saving, setSaving] = useState(false);

  const refresh = useCallback(async () => {
    if (!isDesktop()) {
      setError(t("candidates.needsDesktop"));
      return;
    }
    try {
      setSchool(await getSchoolSettings());
      setError(null);
    } catch (raw: unknown) {
      setError(isAppError(raw) ? t(raw.code, { defaultValue: raw.message }) : String(raw));
    }
  }, [t]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const set = <K extends keyof SchoolSettings>(key: K, value: SchoolSettings[K]) =>
    setSchool((prev) => (prev ? { ...prev, [key]: value } : prev));

  async function save() {
    if (!school) return;
    setSaving(true);
    setSaved(false);
    setError(null);
    try {
      await updateSchool(school);
      setSaved(true);
      await refresh();
    } catch (raw: unknown) {
      setError(isAppError(raw) ? t(raw.code, { defaultValue: raw.message }) : String(raw));
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="max-w-4xl">
      <h1 className="mb-5 text-[1.375rem] font-semibold tracking-tight text-ink">{t("settings.title")}</h1>

      {error && (
        <p className="mb-3 rounded-md border border-owed/30 bg-owed-soft px-3 py-2 text-sm text-owed">
          {error}
        </p>
      )}
      {saved && (
        <p className="mb-3 rounded-md border border-ok bg-surface px-3 py-2 text-sm text-ok">
          {t("settings.saved")}
        </p>
      )}

      {school && (
        <>
          <section className="mb-6 rounded-lg border border-line bg-surface shadow-card">
            <h2 className="border-b border-line px-4 py-3 text-xs uppercase tracking-wide text-muted">
              {t("settings.schoolSection")}
            </h2>
            <div className="grid gap-3 px-4 py-4 sm:grid-cols-2">
              <TextField
                label={t("settings.nameFr")}
                value={school.nameFr}
                onChange={(v) => set("nameFr", v)}
                dir="ltr"
              />
              <TextField
                label={t("settings.nameAr")}
                value={school.nameAr}
                onChange={(v) => set("nameAr", v)}
                dir="rtl"
              />
              <TextField
                label={t("settings.addressFr")}
                value={school.addressFr}
                onChange={(v) => set("addressFr", v)}
                dir="ltr"
              />
              <TextField
                label={t("settings.addressAr")}
                value={school.addressAr}
                onChange={(v) => set("addressAr", v)}
                dir="rtl"
              />
              <TextField
                label={t("settings.commune")}
                value={school.commune}
                onChange={(v) => set("commune", v)}
              />
              <TextField
                label={t("settings.wilayaCode")}
                value={school.wilayaCode}
                onChange={(v) => set("wilayaCode", v)}
                dir="ltr"
              />
              <TextField
                label={t("settings.phone")}
                value={school.phone}
                onChange={(v) => set("phone", v)}
                dir="ltr"
                placeholder="0551 23 45 67"
              />
              <TextField
                label={t("settings.email")}
                value={school.email}
                onChange={(v) => set("email", v)}
                dir="ltr"
              />
              <TextField
                label={t("settings.agrement")}
                value={school.agrementNumber}
                onChange={(v) => set("agrementNumber", v)}
                dir="ltr"
                hint={t("settings.agrementHint")}
              />
              <SelectField
                label={t("settings.defaultLanguage")}
                value={school.defaultLanguage}
                onChange={(v) => set("defaultLanguage", v)}
              >
                <option value="fr">{t("common.french")}</option>
                <option value="ar">{t("common.arabic")}</option>
              </SelectField>
            </div>
          </section>

          <button
            type="button"
            onClick={() => void save()}
            disabled={saving}
            className="mb-8 inline-flex h-9 shrink-0 items-center justify-center gap-2 rounded-md bg-brand px-3.5 text-sm font-medium text-white shadow-card transition-colors hover:bg-brand-hover disabled:cursor-not-allowed disabled:bg-faint"
          >
            {saving ? t("common.loading") : t("settings.saveSchool")}
          </button>
        </>
      )}

      <BackupSection />

    </div>
  );
}
