import { useCallback, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { Check, GraduationCap, Plus, Trash2, UserMinus, UserPlus, X } from "lucide-react";

import FreeText from "@/components/FreeText";
import Modal from "@/components/Modal";
import { Num } from "@/components/Num";
import { SelectField, TextField } from "@/components/FormField";
import Button, { IconButton } from "@/components/ui/Button";
import { Badge, Card, EmptyState, ErrorNote, PageHeader, type Tone } from "@/components/ui/Surface";
import CandidatePicker from "@/features/training/CandidatePicker";
import { currentLanguage } from "@/i18n";
import { isAppError } from "@/lib/candidates";
import {
  addToExamSession,
  createExamSession,
  deleteExamSession,
  listExamEntries,
  listExamSessions,
  removeExamResult,
  setExamOutcome,
  EXAM_KINDS,
  type ExamEntry,
  type ExamSession,
} from "@/lib/exams";
import { kindStyle } from "@/lib/kinds";
import { parseDZD } from "@/lib/money";
import { personName } from "@/lib/names";
import { isDesktop } from "@/lib/tauri";
import { todayIso } from "@/lib/training";

const OUTCOME_TONE: Record<string, Tone> = {
  presented: "brand",
  passed: "ok",
  failed: "owed",
  absent: "warn",
};

/**
 * Exam sittings.
 *
 * The school is told "these three, tomorrow 08:00, this centre, for the
 * code". So this screen is built around the BATCH: create the sitting,
 * assemble who is going, print the list, then come back afterwards and mark
 * who passed.
 *
 * It does not stop anyone being presented for an exam they are not due. The
 * app records what the school does — see CLAUDE.md — so the picker shows
 * everyone and the owner decides.
 */
export default function ExamsPage() {
  const { t } = useTranslation();
  const locale = currentLanguage();

  const [sessions, setSessions] = useState<ExamSession[]>([]);
  const [selected, setSelected] = useState<number | null>(null);
  const [entries, setEntries] = useState<ExamEntry[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const [adding, setAdding] = useState(false);

  const errorText = useCallback(
    (raw: unknown): string =>
      isAppError(raw) ? t(raw.code, { defaultValue: raw.message }) : String(raw),
    [t],
  );

  const loadSessions = useCallback(async () => {
    if (!isDesktop()) {
      setError(t("candidates.needsDesktop"));
      return;
    }
    try {
      const list = await listExamSessions();
      setSessions(list);
      setError(null);
      // Land on the most recent sitting so the screen is never empty-handed.
      setSelected((current) => current ?? list[0]?.id ?? null);
    } catch (raw: unknown) {
      setError(errorText(raw));
    }
  }, [errorText, t]);

  const loadEntries = useCallback(async () => {
    if (selected === null || !isDesktop()) {
      setEntries([]);
      return;
    }
    try {
      setEntries(await listExamEntries(selected));
    } catch (raw: unknown) {
      setError(errorText(raw));
    }
  }, [selected, errorText]);

  useEffect(() => {
    void loadSessions();
  }, [loadSessions]);

  useEffect(() => {
    void loadEntries();
  }, [loadEntries]);

  async function run(action: () => Promise<unknown>) {
    try {
      await action();
      await loadEntries();
      await loadSessions();
    } catch (raw: unknown) {
      setError(errorText(raw));
    }
  }

  const sitting = sessions.find((s) => s.id === selected) ?? null;
  const settled = entries.filter((e) => e.outcome !== "presented").length;

  return (
    <div>
      <PageHeader
        title={t("exams.title")}
        description={t("exams.subtitle")}
        actions={
          <Button
            variant="primary"
            icon={<Plus className="size-4" aria-hidden="true" />}
            onClick={() => setCreating(true)}
          >
            {t("exams.newSession")}
          </Button>
        }
      />

      {error && <ErrorNote>{error}</ErrorNote>}

      {sessions.length === 0 ? (
        <EmptyState
          icon={<GraduationCap className="size-5" aria-hidden="true" />}
          title={t("exams.none")}
          description={t("exams.noneHint")}
          action={
            <Button
              variant="primary"
              icon={<Plus className="size-4" aria-hidden="true" />}
              onClick={() => setCreating(true)}
            >
              {t("exams.newSession")}
            </Button>
          }
        />
      ) : (
        <div className="grid gap-4 lg:grid-cols-[18rem_1fr]">
          {/* The sittings themselves */}
          <Card flush className="h-fit overflow-hidden">
            <ul className="divide-y divide-line">
              {sessions.map((s) => {
                const active = s.id === selected;
                return (
                  <li key={s.id}>
                    <button
                      type="button"
                      onClick={() => setSelected(s.id)}
                      className={[
                        "flex w-full flex-col gap-1 px-4 py-3 text-start transition-colors",
                        active ? "bg-brand-soft" : "hover:bg-sunken",
                      ].join(" ")}
                    >
                      <span className="flex items-center gap-2">
                        <span
                          className={[
                            "rounded px-1.5 py-0.5 text-xs font-semibold",
                            kindStyle(s.kind).chip,
                          ].join(" ")}
                        >
                          {t(`lessons.kind_${s.kind}`)}
                        </span>
                        <Num className="text-sm font-semibold text-ink">{s.heldOn}</Num>
                        {s.heldAt && <Num className="text-xs text-muted">{s.heldAt}</Num>}
                      </span>
                      {s.place && (
                        <FreeText as="span" className="truncate text-xs text-muted">
                          {s.place}
                        </FreeText>
                      )}
                    </button>
                  </li>
                );
              })}
            </ul>
          </Card>

          {/* Who is in the selected one */}
          {sitting && (
            <Card flush>
              <header className="flex flex-wrap items-center gap-3 border-b border-line px-4 py-3">
                <h2 className="text-sm font-semibold text-ink">
                  {t(`lessons.kind_${sitting.kind}`)}
                  <span className="text-faint"> · </span>
                  <Num>{sitting.heldOn}</Num>
                </h2>
                <span className="text-xs text-muted">
                  {t("exams.settledCount", { settled, total: entries.length })}
                </span>
                <div className="ms-auto flex gap-2">
                  <Button
                    size="sm"
                    icon={<UserPlus className="size-3.5" aria-hidden="true" />}
                    onClick={() => setAdding(true)}
                  >
                    {t("exams.addCandidate")}
                  </Button>
                  <IconButton
                    aria-label={t("common.delete")}
                    className="size-8 text-owed hover:bg-owed-soft"
                    onClick={() =>
                      void run(async () => {
                        await deleteExamSession(sitting.id);
                        setSelected(null);
                      })
                    }
                  >
                    <Trash2 className="size-3.5" aria-hidden="true" />
                  </IconButton>
                </div>
              </header>

              {entries.length === 0 ? (
                <p className="px-4 py-10 text-center text-sm text-muted">
                  {t("exams.nobodyYet")}
                </p>
              ) : (
                <ul className="divide-y divide-line">
                  {entries.map((entry) => (
                    <li
                      key={entry.id}
                      className="flex flex-wrap items-center gap-x-3 gap-y-2 px-4 py-2.5"
                    >
                      <Link
                        to={`/candidates/${entry.candidateId}`}
                        className="text-sm font-medium text-ink hover:text-brand hover:underline"
                      >
                        {personName(entry, locale)}
                      </Link>
                      {/* A second go is worth seeing at a glance. */}
                      {entry.attempt > 1 && (
                        <Badge tone="warn">
                          {t("exams.attempt", { n: entry.attempt })}
                        </Badge>
                      )}

                      <div className="ms-auto flex flex-wrap items-center gap-1.5">
                        <Badge tone={OUTCOME_TONE[entry.outcome] ?? "neutral"}>
                          {t(`exams.outcome_${entry.outcome}`)}
                        </Badge>
                        <Button
                          size="sm"
                          variant={entry.outcome === "passed" ? "primary" : "secondary"}
                          icon={<Check className="size-3.5" aria-hidden="true" />}
                          onClick={() => void run(() => setExamOutcome(entry.id, "passed", ""))}
                        >
                          {t("exams.outcome_passed")}
                        </Button>
                        <Button
                          size="sm"
                          onClick={() => void run(() => setExamOutcome(entry.id, "failed", ""))}
                        >
                          {t("exams.outcome_failed")}
                        </Button>
                        <Button
                          size="sm"
                          variant="ghost"
                          icon={<X className="size-3.5" aria-hidden="true" />}
                          onClick={() => void run(() => setExamOutcome(entry.id, "absent", ""))}
                        >
                          {t("exams.outcome_absent")}
                        </Button>
                        <IconButton
                          aria-label={t("exams.removeFromSession")}
                          className="size-8 text-owed hover:bg-owed-soft"
                          onClick={() => void run(() => removeExamResult(entry.id))}
                        >
                          <UserMinus className="size-3.5" aria-hidden="true" />
                        </IconButton>
                      </div>
                    </li>
                  ))}
                </ul>
              )}
            </Card>
          )}
        </div>
      )}

      {creating && (
        <NewSessionDialog
          onClose={() => setCreating(false)}
          onDone={(created) => {
            setCreating(false);
            setSelected(created.id);
            void loadSessions();
          }}
        />
      )}

      {adding && sitting && (
        <AddToSessionDialog
          sessionId={sitting.id}
          onClose={() => setAdding(false)}
          onDone={() => {
            setAdding(false);
            void run(async () => {});
          }}
        />
      )}
    </div>
  );
}

/** Creating the batch: one exam, one date, one place. */
function NewSessionDialog({
  onClose,
  onDone,
}: {
  onClose: () => void;
  onDone: (created: ExamSession) => void;
}) {
  const { t } = useTranslation();

  const [kind, setKind] = useState<string>("theory");
  const [heldOn, setHeldOn] = useState(todayIso());
  const [heldAt, setHeldAt] = useState("08:00");
  const [place, setPlace] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function save() {
    setSaving(true);
    setError(null);
    try {
      onDone(await createExamSession({ kind, heldOn, heldAt, place, notes: "" }));
    } catch (e: unknown) {
      setError(isAppError(e) ? t(e.code, { defaultValue: e.message }) : String(e));
      setSaving(false);
    }
  }

  return (
    <Modal
      title={t("exams.newSession")}
      onClose={onClose}
      onSubmit={() => void save()}
      submitting={saving}
      error={error}
      submitLabel={t("common.save")}
    >
      <div className="grid gap-3">
        <SelectField label={t("exams.whichExam")} value={kind} onChange={setKind}>
          {EXAM_KINDS.map((k) => (
            <option key={k} value={k}>
              {t(`lessons.kind_${k}`)}
            </option>
          ))}
        </SelectField>
        <div className="grid gap-3 sm:grid-cols-2">
          <TextField
            label={t("exams.heldOn")}
            value={heldOn}
            onChange={setHeldOn}
            type="date"
            dir="ltr"
          />
          <TextField
            label={t("exams.heldAt")}
            value={heldAt}
            onChange={setHeldAt}
            type="time"
            dir="ltr"
            hint={t("exams.heldAtHint")}
          />
        </div>
        <TextField label={t("exams.place")} value={place} onChange={setPlace} />
      </div>
    </Modal>
  );
}

/** Adding one candidate to the batch. */
function AddToSessionDialog({
  sessionId,
  onClose,
  onDone,
}: {
  sessionId: number;
  onClose: () => void;
  onDone: () => void;
}) {
  const { t } = useTranslation();

  const [chosen, setChosen] = useState<number | null>(null);
  /**
   * What this candidate is paying for this sitting.
   *
   * Starts empty, and empty means nothing — which is how a school that gives
   * the repassage free says so. Nothing is remembered between candidates and
   * nothing is pre-filled: the amount is the owner's to state every time,
   * because some schools charge the retake again and some do not.
   */
  const [fee, setFee] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [pickError, setPickError] = useState<string | undefined>(undefined);

  async function save() {
    if (chosen === null) {
      setPickError(t("calendar.pickCandidate"));
      return;
    }
    setSaving(true);
    setError(null);
    try {
      await addToExamSession(sessionId, chosen, parseDZD(fee) ?? 0);
      onDone();
    } catch (e: unknown) {
      setError(isAppError(e) ? t(e.code, { defaultValue: e.message }) : String(e));
      setSaving(false);
    }
  }

  return (
    <Modal
      title={t("exams.addCandidate")}
      onClose={onClose}
      onSubmit={() => void save()}
      submitting={saving}
      error={error}
      submitLabel={t("exams.addCandidate")}
    >
      <CandidatePicker
        value={chosen}
        error={pickError}
        onChange={(id) => {
          setChosen(id);
          setPickError(undefined);
        }}
      />

      <div className="mt-4">
        <label
          htmlFor="exam-fee"
          className="mb-1 block text-xs font-medium text-ink-soft"
        >
          {t("fees.amount")}
        </label>
        <input
          id="exam-fee"
          type="text"
          inputMode="numeric"
          value={fee}
          onChange={(e) => setFee(e.target.value)}
          dir="ltr"
          placeholder="0"
          className="w-36 rounded-md border border-line-strong bg-surface px-2.5 py-1.5 text-sm outline-none transition-colors focus:border-brand focus:ring-2 focus:ring-brand-soft"
        />
        <p className="mt-1 text-xs text-muted">{t("fees.entryHint")}</p>
      </div>
    </Modal>
  );
}
