import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { SelectField, TextField } from "@/components/FormField";
import { Num } from "@/components/Num";
import { currentLanguage } from "@/i18n";
import { isAppError } from "@/lib/candidates";
import { formatDZD, parseDZD } from "@/lib/money";
import {
  getSchoolSettings,
  listAllCategories,
  updateCategory,
  updateSchool,
  type Category,
  type SchoolSettings,
} from "@/lib/settings";
import { isDesktop } from "@/lib/tauri";

/** One editable row of the price list. */
function CategoryRow({
  category,
  onSaved,
  onError,
}: {
  category: Category;
  onSaved: () => void;
  onError: (message: string) => void;
}) {
  const { t } = useTranslation();
  const locale = currentLanguage();

  const [theory, setTheory] = useState(String(category.theoryHoursTarget));
  const [practical, setPractical] = useState(String(category.practicalHoursTarget));
  const [price, setPrice] = useState(
    category.defaultPriceCentimes > 0
      ? formatDZD(category.defaultPriceCentimes, { withSuffix: false })
      : "",
  );
  const [offered, setOffered] = useState(category.isOffered === 1);
  const [saving, setSaving] = useState(false);

  const dirty =
    theory !== String(category.theoryHoursTarget) ||
    practical !== String(category.practicalHoursTarget) ||
    offered !== (category.isOffered === 1) ||
    (parseDZD(price) ?? 0) !== category.defaultPriceCentimes;

  async function save() {
    setSaving(true);
    try {
      await updateCategory(category.code, {
        codeAr: category.codeAr,
        nameFr: category.nameFr,
        nameAr: category.nameAr,
        theoryHoursTarget: Number(theory) || 0,
        practicalHoursTarget: Number(practical) || 0,
        defaultPriceCentimes: parseDZD(price) ?? 0,
        isOffered: offered,
      });
      onSaved();
    } catch (raw: unknown) {
      onError(isAppError(raw) ? t(raw.code, { defaultValue: raw.message }) : String(raw));
    } finally {
      setSaving(false);
    }
  }

  return (
    <tr className="border-b border-line last:border-b-0">
      <td className="px-3 py-2">
        <span className="font-medium">{category.code}</span>
        <span className="ms-2 text-xs text-muted">
          {locale === "ar" ? category.nameAr : category.nameFr}
        </span>
      </td>
      <td className="px-3 py-2">
        <input
          type="checkbox"
          checked={offered}
          onChange={(e) => setOffered(e.target.checked)}
          aria-label={t("settings.offered")}
        />
      </td>
      <td className="px-3 py-2">
        <input
          type="number"
          value={theory}
          onChange={(e) => setTheory(e.target.value)}
          dir="ltr"
          className="w-20 rounded-md border border-line px-2 py-1 text-sm"
        />
      </td>
      <td className="px-3 py-2">
        <input
          type="number"
          value={practical}
          onChange={(e) => setPractical(e.target.value)}
          dir="ltr"
          className="w-20 rounded-md border border-line px-2 py-1 text-sm"
        />
      </td>
      <td className="px-3 py-2">
        <input
          type="text"
          value={price}
          onChange={(e) => setPrice(e.target.value)}
          dir="ltr"
          placeholder="35000"
          className="w-28 rounded-md border border-line px-2 py-1 text-sm"
        />
      </td>
      <td className="px-3 py-2 text-end">
        {dirty && (
          <button
            type="button"
            onClick={() => void save()}
            disabled={saving}
            className="rounded-md bg-brand px-3 py-1 text-xs font-medium text-white disabled:opacity-50"
          >
            {saving ? t("common.loading") : t("common.save")}
          </button>
        )}
      </td>
    </tr>
  );
}

/**
 * School settings.
 *
 * Without this screen a school prints somebody else's name on every receipt,
 * so it is the last thing standing between the app and a real school using it.
 */
export default function SettingsPage() {
  const { t } = useTranslation();

  const [school, setSchool] = useState<SchoolSettings | null>(null);
  const [categories, setCategories] = useState<Category[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const [saving, setSaving] = useState(false);

  const refresh = useCallback(async () => {
    if (!isDesktop()) {
      setError(t("candidates.needsDesktop"));
      return;
    }
    try {
      const [s, c] = await Promise.all([getSchoolSettings(), listAllCategories()]);
      setSchool(s);
      setCategories(c);
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
      <h1 className="mb-4 text-xl font-semibold">{t("settings.title")}</h1>

      {error && (
        <p className="mb-3 rounded-md border border-owed bg-white px-3 py-2 text-sm text-owed">
          {error}
        </p>
      )}
      {saved && (
        <p className="mb-3 rounded-md border border-ok bg-white px-3 py-2 text-sm text-ok">
          {t("settings.saved")}
        </p>
      )}

      {school && (
        <>
          <section className="mb-6 rounded-lg border border-line bg-white">
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

          <section className="mb-6 rounded-lg border border-line bg-white">
            <h2 className="border-b border-line px-4 py-3 text-xs uppercase tracking-wide text-muted">
              {t("settings.receiptSection")}
            </h2>
            <div className="grid gap-3 px-4 py-4 sm:grid-cols-2">
              <TextField
                label={t("settings.receiptPrefix")}
                value={school.receiptPrefix}
                onChange={(v) => set("receiptPrefix", v)}
                dir="ltr"
                placeholder="2026-"
                hint={t("settings.receiptPrefixHint", {
                  example: `${school.receiptPrefix}${String(school.nextReceiptNumber).padStart(4, "0")}`,
                })}
              />
              {school.receiptCounterLocked ? (
                <div>
                  <span className="mb-1 block text-xs text-muted">
                    {t("settings.nextReceiptNumber")}
                  </span>
                  <p className="rounded-md border border-line bg-brand-soft px-3 py-2 text-sm">
                    <Num>{school.nextReceiptNumber}</Num>
                  </p>
                  <span className="mt-1 block text-xs text-muted">
                    {t("settings.receiptCounterLocked")}
                  </span>
                </div>
              ) : (
                <TextField
                  label={t("settings.nextReceiptNumber")}
                  value={String(school.nextReceiptNumber)}
                  onChange={(v) => set("nextReceiptNumber", Number(v) || 1)}
                  type="number"
                  dir="ltr"
                  hint={t("settings.receiptCounterHint")}
                />
              )}
            </div>
          </section>

          <button
            type="button"
            onClick={() => void save()}
            disabled={saving}
            className="mb-8 rounded-md bg-brand px-4 py-2 text-sm font-medium text-white disabled:opacity-50"
          >
            {saving ? t("common.loading") : t("settings.saveSchool")}
          </button>
        </>
      )}

      <section className="rounded-lg border border-line bg-white">
        <h2 className="border-b border-line px-4 py-3 text-xs uppercase tracking-wide text-muted">
          {t("settings.priceListSection")}
        </h2>
        <p className="border-b border-line px-4 py-2 text-xs text-muted">
          {t("settings.priceListNote")}
        </p>
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-line text-xs uppercase tracking-wide text-muted">
              <th className="px-3 py-2 text-start font-medium">{t("settings.category")}</th>
              <th className="px-3 py-2 text-start font-medium">{t("settings.offered")}</th>
              <th className="px-3 py-2 text-start font-medium">{t("settings.theoryHours")}</th>
              <th className="px-3 py-2 text-start font-medium">
                {t("settings.practicalHours")}
              </th>
              <th className="px-3 py-2 text-start font-medium">{t("settings.price")}</th>
              <th className="px-3 py-2 text-end font-medium" />
            </tr>
          </thead>
          <tbody>
            {categories.map((category) => (
              <CategoryRow
                key={category.code}
                category={category}
                onSaved={() => void refresh()}
                onError={setError}
              />
            ))}
          </tbody>
        </table>
      </section>
    </div>
  );
}
