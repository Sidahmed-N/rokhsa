import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { SelectField, TextField } from "@/components/FormField";
import Modal from "@/components/Modal";
import { Money } from "@/components/Num";
import { currentLanguage } from "@/i18n";
import { isAppError } from "@/lib/candidates";
import { formatDZD, parseDZD } from "@/lib/money";
import {
  LESSON_KINDS,
  createEnrollment,
  createLesson,
  createPayment,
  listCategories,
  listPaymentMethods,
  nextUp,
  scheduleLesson,
  type LicenceCategory,
  type Payment,
  type PaymentMethod,
} from "@/lib/training";

function today(): string {
  const now = new Date();
  const m = String(now.getMonth() + 1).padStart(2, "0");
  const d = String(now.getDate()).padStart(2, "0");
  return `${now.getFullYear()}-${m}-${d}`;
}

/** Turn a Rust error into a translated message. */
function useErrorText() {
  const { t } = useTranslation();
  return (raw: unknown): string =>
    isAppError(raw) ? t(raw.code, { defaultValue: raw.message }) : String(raw);
}

/* ------------------------------ enrolment ------------------------------ */

export function NewEnrollmentDialog({
  candidateId,
  onDone,
  onClose,
}: {
  candidateId: number;
  onDone: () => void;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const locale = currentLanguage();
  const errorText = useErrorText();

  const [categories, setCategories] = useState<LicenceCategory[]>([]);
  const [code, setCode] = useState("");
  const [price, setPrice] = useState("");
  const [extraHourPrice, setExtraHourPrice] = useState("");
  const [theoryTarget, setTheoryTarget] = useState("");
  const [practicalTarget, setPracticalTarget] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    listCategories()
      .then((list) => {
        setCategories(list);
        const first = list[0];
        if (first) applyCategory(first);
      })
      .catch((e: unknown) => setError(errorText(e)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  function applyCategory(category: LicenceCategory) {
    setCode(category.code);
    // Prefill from the price list. These become a SNAPSHOT on save — editing
    // the price list later never changes an existing enrolment.
    setPrice(
      category.defaultPriceCentimes > 0
        ? formatDZD(category.defaultPriceCentimes, { withSuffix: false })
        : "",
    );
    setTheoryTarget(String(category.theoryHoursTarget || ""));
    setPracticalTarget(String(category.practicalHoursTarget || ""));
  }

  async function save() {
    setSaving(true);
    setError(null);
    try {
      await createEnrollment(candidateId, {
        categoryCode: code,
        enrolledOn: today(),
        agreedPriceCentimes: parseDZD(price) ?? 0,
        extraHourPriceCentimes: parseDZD(extraHourPrice) ?? 0,
        theoryHoursTarget: Number(theoryTarget) || 0,
        practicalHoursTarget: Number(practicalTarget) || 0,
        discountCentimes: 0,
        discountReason: "",
        status: "forming",
        notes: "",
      });
      onDone();
    } catch (e: unknown) {
      setError(errorText(e));
    } finally {
      setSaving(false);
    }
  }

  return (
    <Modal
      title={t("enrollments.newTitle")}
      onClose={onClose}
      onSubmit={() => void save()}
      submitting={saving}
      error={error}
    >
      <div className="grid gap-3">
        <SelectField
          label={t("enrollments.category")}
          value={code}
          onChange={(next) => {
            const found = categories.find((c) => c.code === next);
            if (found) applyCategory(found);
          }}
        >
          {categories.map((c) => (
            <option key={c.code} value={c.code}>
              {c.code} — {locale === "ar" ? c.nameAr : c.nameFr}
            </option>
          ))}
        </SelectField>

        <TextField
          label={t("enrollments.agreedPrice")}
          value={price}
          onChange={setPrice}
          dir="ltr"
          placeholder="35000"
          hint={t("enrollments.priceHint")}
        />
        <div className="grid gap-3 sm:grid-cols-2">
          <TextField
            label={t("enrollments.theoryTarget")}
            value={theoryTarget}
            onChange={setTheoryTarget}
            dir="ltr"
            type="number"
          />
          <TextField
            label={t("enrollments.practicalTarget")}
            value={practicalTarget}
            onChange={setPracticalTarget}
            dir="ltr"
            type="number"
          />
        </div>
        <TextField
          label={t("enrollments.extraHourPrice")}
          value={extraHourPrice}
          onChange={setExtraHourPrice}
          dir="ltr"
          placeholder="0"
          hint={t("enrollments.extraHourHint")}
        />
      </div>
    </Modal>
  );
}

/* -------------------------------- hours -------------------------------- */

export function AddLessonDialog({
  enrollmentId,
  onDone,
  onClose,
}: {
  enrollmentId: number;
  onDone: () => void;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const errorText = useErrorText();

  const [kind, setKind] = useState<string>("road");
  const [occurredOn, setOccurredOn] = useState(today());
  const [minutes, setMinutes] = useState("60");
  const [notes, setNotes] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function save() {
    setSaving(true);
    setError(null);
    try {
      await createLesson(enrollmentId, {
        kind,
        occurredOn,
        minutes: Number(minutes) || 0,
        instructorStaffId: null,
        notes,
      });
      onDone();
    } catch (e: unknown) {
      setError(errorText(e));
    } finally {
      setSaving(false);
    }
  }

  return (
    <Modal
      title={t("lessons.addTitle")}
      onClose={onClose}
      onSubmit={() => void save()}
      submitting={saving}
      error={error}
    >
      <div className="grid gap-3">
        <SelectField label={t("lessons.kind")} value={kind} onChange={setKind}>
          {LESSON_KINDS.map((k) => (
            <option key={k} value={k}>
              {t(`lessons.kind_${k}`)}
            </option>
          ))}
        </SelectField>
        <TextField
          label={t("lessons.date")}
          value={occurredOn}
          onChange={setOccurredOn}
          type="date"
          dir="ltr"
        />
        <TextField
          label={t("lessons.minutes")}
          value={minutes}
          onChange={setMinutes}
          type="number"
          dir="ltr"
          hint={t("lessons.minutesHint")}
        />
        <TextField label={t("lessons.notes")} value={notes} onChange={setNotes} />
      </div>
    </Modal>
  );
}

/* ------------------------------ book ahead ----------------------------- */

export function ScheduleSessionDialog({
  enrollmentId,
  onDone,
  onClose,
}: {
  enrollmentId: number;
  onDone: () => void;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const errorText = useErrorText();

  const [kind, setKind] = useState<string>("road");
  const [scheduledOn, setScheduledOn] = useState(today());
  const [scheduledTime, setScheduledTime] = useState("09:00");
  const [minutes, setMinutes] = useState("60");
  const [planNote, setPlanNote] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Pre-fill what to work on from the last session's note, so the instructor
  // does not have to go and find it.
  useEffect(() => {
    nextUp(enrollmentId)
      .then((info) => {
        if (info.lastNote) setPlanNote(info.lastNote);
      })
      .catch(() => {});
  }, [enrollmentId]);

  async function save() {
    setSaving(true);
    setError(null);
    try {
      await scheduleLesson(enrollmentId, {
        kind,
        scheduledOn,
        scheduledTime,
        minutes: Number(minutes) || 0,
        instructorStaffId: null,
        planNote,
      });
      onDone();
    } catch (e: unknown) {
      setError(errorText(e));
    } finally {
      setSaving(false);
    }
  }

  return (
    <Modal
      title={t("calendar.bookTitle")}
      onClose={onClose}
      onSubmit={() => void save()}
      submitting={saving}
      error={error}
      submitLabel={t("calendar.book")}
    >
      <div className="grid gap-3">
        <SelectField label={t("lessons.kind")} value={kind} onChange={setKind}>
          {LESSON_KINDS.map((k) => (
            <option key={k} value={k}>
              {t(`lessons.kind_${k}`)}
            </option>
          ))}
        </SelectField>
        <div className="grid gap-3 sm:grid-cols-2">
          <TextField
            label={t("calendar.day")}
            value={scheduledOn}
            onChange={setScheduledOn}
            type="date"
            dir="ltr"
          />
          <TextField
            label={t("calendar.time")}
            value={scheduledTime}
            onChange={setScheduledTime}
            type="time"
            dir="ltr"
            hint={t("calendar.timeOptional")}
          />
        </div>
        <TextField
          label={t("lessons.minutes")}
          value={minutes}
          onChange={setMinutes}
          type="number"
          dir="ltr"
        />
        <TextField
          label={t("calendar.planNote")}
          value={planNote}
          onChange={setPlanNote}
          hint={t("calendar.planNoteHint")}
        />
      </div>
    </Modal>
  );
}

/* ------------------------------- payment ------------------------------- */

export function TakePaymentDialog({
  enrollmentId,
  balanceCentimes,
  onDone,
  onClose,
}: {
  enrollmentId: number;
  balanceCentimes: number;
  onDone: (payment: Payment) => void;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const locale = currentLanguage();
  const errorText = useErrorText();

  const [methods, setMethods] = useState<PaymentMethod[]>([]);
  const [amount, setAmount] = useState("");
  const [paidOn, setPaidOn] = useState(today());
  const [methodCode, setMethodCode] = useState("cash");
  const [reference, setReference] = useState("");
  const [notes, setNotes] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    listPaymentMethods()
      .then(setMethods)
      .catch((e: unknown) => setError(errorText(e)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const parsed = parseDZD(amount);
  const method = methods.find((m) => m.code === methodCode);

  async function save() {
    if (parsed === null || parsed <= 0) {
      setError(t("payments.error.amountPositive"));
      return;
    }
    setSaving(true);
    setError(null);
    try {
      const payment = await createPayment(enrollmentId, {
        amountCentimes: parsed,
        paidOn,
        methodCode,
        reference,
        notes,
      });
      onDone(payment);
    } catch (e: unknown) {
      setError(errorText(e));
    } finally {
      setSaving(false);
    }
  }

  return (
    <Modal
      title={t("payments.takeTitle")}
      onClose={onClose}
      onSubmit={() => void save()}
      submitting={saving}
      error={error}
      submitLabel={t("payments.saveAndPrint")}
    >
      <div className="grid gap-3">
        <p className="rounded-md border border-line bg-brand-soft px-3 py-2 text-sm">
          {t("payments.currentBalance")}:{" "}
          <Money centimes={balanceCentimes} className="font-bold" />
        </p>

        <TextField
          label={t("payments.amount")}
          value={amount}
          onChange={setAmount}
          dir="ltr"
          placeholder="12000"
          hint={t("payments.amountHint")}
        />
        <TextField
          label={t("payments.date")}
          value={paidOn}
          onChange={setPaidOn}
          type="date"
          dir="ltr"
        />
        <SelectField
          label={t("payments.method")}
          value={methodCode}
          onChange={setMethodCode}
        >
          {methods.map((m) => (
            <option key={m.code} value={m.code}>
              {locale === "ar" ? m.nameAr : m.nameFr}
            </option>
          ))}
        </SelectField>

        {method && method.requiresReference === 1 && (
          <TextField
            label={t("payments.reference")}
            value={reference}
            onChange={setReference}
            dir="ltr"
          />
        )}

        <TextField label={t("payments.notes")} value={notes} onChange={setNotes} />
      </div>
    </Modal>
  );
}
