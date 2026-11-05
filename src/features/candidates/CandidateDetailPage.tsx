import { useCallback, useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { useTranslation } from "react-i18next";
import {
  Archive,
  ArchiveRestore,
  BadgeCheck,
  Banknote,
  CalendarCheck,
  CalendarPlus,
  Check,
  ChevronLeft,
  CreditCard,
  IdCard,
  Pencil,
  Phone,
  Plus,
  Trash2,
  Undo2,
} from "lucide-react";

import FreeText from "@/components/FreeText";
import { Paged } from "@/components/ui/Pager";
import ExamHistory from "@/features/candidates/ExamHistory";
import { DateText, Duration, Money, Num } from "@/components/Num";
import Button, { IconButton } from "@/components/ui/Button";
import { Badge, Card, EmptyState, ErrorNote, type Tone } from "@/components/ui/Surface";
import {
  getCandidate,
  isAppError,
  setCandidatesArchived,
  type Candidate,
} from "@/lib/candidates";
import { personName } from "@/lib/names";
import { isDesktop } from "@/lib/tauri";
import {
  completeLesson,
  deleteLesson,
  listEnrollments,
  listLessons,
  listPayments,
  localDate,
  reopenLesson,
  reversePayment,
  todayIso,
  type Enrollment,
  type EnrollmentSummary,
  type Lesson,
  type Payment,
} from "@/lib/training";
import {
  getExamProgress,
  listExamResults,
  type ExamProgress,
  type ExamResult,
} from "@/lib/exams";
import {
  AddLessonDialog,
  EditEnrollmentDialog,
  NewEnrollmentDialog,
  ScheduleSessionDialog,
  TakePaymentDialog,
} from "@/features/training/TrainingDialogs";

const STATUS_TONE: Record<string, Tone> = {
  planned: "brand",
  done: "ok",
  missed: "warn",
  cancelled: "neutral",
};

function Figure({
  label,
  children,
  strong = false,
  tone,
  extra,
}: {
  label: string;
  children: React.ReactNode;
  strong?: boolean | undefined;
  tone?: "owed" | "ok" | undefined;
  extra?: React.ReactNode | undefined;
}) {
  return (
    <div>
      <p className="text-xs font-medium tracking-wide text-muted uppercase">{label}</p>
      <p
        className={[
          "mt-1 tracking-tight",
          strong ? "text-xl font-semibold" : "text-sm font-medium",
          tone === "owed" ? "text-owed" : tone === "ok" ? "text-ok" : "text-ink",
        ].join(" ")}
      >
        {children}
      </p>
      {extra}
    </div>
  );
}

/** Séances per page on a candidate file. */
const LESSON_PAGE_SIZE = 15;
/** Versements per page on a candidate file. */
const PAYMENT_PAGE_SIZE = 10;

export default function CandidateDetailPage() {
  const { t } = useTranslation();
  const params = useParams();
  const candidateId = Number(params.id);

  const [candidate, setCandidate] = useState<Candidate | null>(null);
  const [summaries, setSummaries] = useState<EnrollmentSummary[]>([]);
  const [lessonsBy, setLessonsBy] = useState<Record<number, Lesson[]>>({});
  const [paymentsBy, setPaymentsBy] = useState<Record<number, Payment[]>>({});
  const [examsBy, setExamsBy] = useState<Record<number, ExamResult[]>>({});
  const [examProgressBy, setExamProgressBy] = useState<Record<number, ExamProgress>>({});
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const [newEnrollment, setNewEnrollment] = useState(false);
  const [addingHoursTo, setAddingHoursTo] = useState<number | null>(null);
  const [bookingFor, setBookingFor] = useState<number | null>(null);
  /** The kind of a missed session being made up, so the dialog opens on it. */
  const [makeUpKind, setMakeUpKind] = useState<string | null>(null);
  const [payingFor, setPayingFor] = useState<EnrollmentSummary | null>(null);
  const [editing, setEditing] = useState<Enrollment | null>(null);

  const errorText = useCallback(
    (raw: unknown): string =>
      isAppError(raw) ? t(raw.code, { defaultValue: raw.message }) : String(raw),
    [t],
  );

  const refresh = useCallback(async () => {
    if (!isDesktop()) {
      setError(t("candidates.needsDesktop"));
      setLoading(false);
      return;
    }
    setLoading(true);
    try {
      const [c, list] = await Promise.all([
        getCandidate(candidateId),
        listEnrollments(candidateId),
      ]);
      setCandidate(c);
      setSummaries(list);

      const lessonEntries = await Promise.all(
        list.map(async (e) => [e.enrollment.id, await listLessons(e.enrollment.id)] as const),
      );
      const paymentEntries = await Promise.all(
        list.map(async (e) => [e.enrollment.id, await listPayments(e.enrollment.id)] as const),
      );
      const examEntries = await Promise.all(
        list.map(async (e) => [e.enrollment.id, await listExamResults(e.enrollment.id)] as const),
      );
      const progressEntries = await Promise.all(
        list.map(async (e) => [e.enrollment.id, await getExamProgress(e.enrollment.id)] as const),
      );
      setLessonsBy(Object.fromEntries(lessonEntries));
      setPaymentsBy(Object.fromEntries(paymentEntries));
      setExamsBy(Object.fromEntries(examEntries));
      setExamProgressBy(Object.fromEntries(progressEntries));
      setError(null);
    } catch (raw: unknown) {
      setError(errorText(raw));
    } finally {
      setLoading(false);
    }
  }, [candidateId, errorText, t]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  /**
   * Take payment and stay put.
   *
   * No receipt is printed: schools write the candidate's own card by hand and
   * do not want the app duplicating it. The payment still carries a hidden
   * sequential number so a deleted one leaves a gap in the books.
   */
  function onPaymentTaken(_payment: Payment) {
    setPayingFor(null);
    void refresh();
  }

  // Asked first: the reversal row can never be taken back (money rows are
  // append-only), and the button sits one row away from the next payment.
  const [reversing, setReversing] = useState<Payment | null>(null);

  // No confirmation: archiving takes nothing away and Restaurer undoes it.
  async function setArchived(archived: boolean) {
    try {
      await setCandidatesArchived([candidateId], archived);
      await refresh();
    } catch (raw: unknown) {
      setError(errorText(raw));
    }
  }

  async function doReverse(payment: Payment) {
    const reason = t("payments.reversedByUser");
    try {
      await reversePayment(payment.id, reason);
      setReversing(null);
      await refresh();
    } catch (raw: unknown) {
      setReversing(null);
      setError(errorText(raw));
    }
  }

  return (
    <>
      <div>
        <Link
          to="/candidates"
          className="inline-flex items-center gap-1 text-sm text-muted transition-colors hover:text-brand"
        >
          <ChevronLeft className="size-4 rtl:rotate-180" aria-hidden="true" />
          {t("candidates.title")}
        </Link>

        {error && (
          <div className="mt-3">
            <ErrorNote>{error}</ErrorNote>
          </div>
        )}

        {loading && <p className="mt-4 text-sm text-muted">{t("common.loading")}</p>}

        {candidate && (
          <>
            {candidate.archivedAt && (
              <div className="mt-3 flex flex-wrap items-center gap-x-3 gap-y-2 rounded-lg border border-line bg-sunken px-4 py-3">
                <Archive className="size-4 shrink-0 text-muted" aria-hidden="true" />
                <div className="min-w-0 flex-1">
                  <p className="text-sm font-medium text-ink">
                    {t("candidates.archivedBanner")}
                    <span className="ms-2 font-normal text-muted">
                      <DateText value={localDate(new Date(candidate.archivedAt))} />
                    </span>
                  </p>
                  <p className="mt-0.5 text-xs text-muted">{t("candidates.archivedHint")}</p>
                </div>
                <Button
                  icon={<ArchiveRestore className="size-4" aria-hidden="true" />}
                  onClick={() => void setArchived(false)}
                >
                  {t("candidates.unarchive")}
                </Button>
              </div>
            )}

            <header className="mt-3 mb-5 flex flex-wrap items-center gap-x-4 gap-y-2">
              <div>
                <h1 className="text-[1.375rem] font-semibold tracking-tight text-ink">
                  {personName(candidate)}
                </h1>
                <div className="mt-1.5 flex flex-wrap items-center gap-x-4 gap-y-1 text-sm text-muted">
                  {candidate.phone && (
                    <span className="inline-flex items-center gap-1.5">
                      <Phone className="size-3.5 text-faint" aria-hidden="true" />
                      <Num>{candidate.phone}</Num>
                    </span>
                  )}
                  {candidate.nin && (
                    <span className="inline-flex items-center gap-1.5">
                      <IdCard className="size-3.5 text-faint" aria-hidden="true" />
                      <Num>{candidate.nin}</Num>
                    </span>
                  )}
                </div>
              </div>

              <div className="ms-auto flex flex-wrap gap-2">
                {!candidate.archivedAt && (
                  <Button
                    icon={<Archive className="size-4" aria-hidden="true" />}
                    onClick={() => void setArchived(true)}
                  >
                    {t("candidates.archive")}
                  </Button>
                )}
                <Button
                  variant="primary"
                  icon={<Plus className="size-4" aria-hidden="true" />}
                  onClick={() => setNewEnrollment(true)}
                >
                  {t("enrollments.new")}
                </Button>
              </div>
            </header>

            {summaries.length === 0 && !loading && (
              <EmptyState
                icon={<BadgeCheck className="size-5" aria-hidden="true" />}
                title={t("enrollments.none")}
                action={
                  <Button
                    variant="primary"
                    icon={<Plus className="size-4" aria-hidden="true" />}
                    onClick={() => setNewEnrollment(true)}
                  >
                    {t("enrollments.new")}
                  </Button>
                }
              />
            )}

            <div className="grid gap-5">
              {summaries.map((summary) => {
                const id = summary.enrollment.id;
                const lessons = lessonsBy[id] ?? [];
                const payments = paymentsBy[id] ?? [];
                const owes = summary.balanceCentimes > 0;

                // One enrolment is four cards: the inscription (money and the
                // actions), then exams, séances and payments, each on its own.
                // In one frame with thin rules between them they read as one
                // long block, and the exam dates as part of the séance log.
                return (
                  <div key={id} className="grid gap-4">
                  <Card flush>
                    <header className="flex flex-wrap items-center gap-3 border-b border-line px-4 py-3">
                      <h2 className="text-base font-semibold tracking-tight text-ink">
                        {t("enrollments.heading")}
                      </h2>
                      <Badge tone={summary.enrollment.status === "withdrawn" ? "warn" : "brand"}>
                        {t(`enrollments.status_${summary.enrollment.status}`)}
                      </Badge>
                      <span className="ms-auto text-xs text-muted">
                        {t("enrollments.enrolledOn")} <DateText value={summary.enrollment.enrolledOn} />
                      </span>
                      <Button
                        size="sm"
                        icon={<Pencil className="size-3.5" aria-hidden="true" />}
                        onClick={() => setEditing(summary.enrollment)}
                      >
                        {t("common.edit")}
                      </Button>
                    </header>

                    {/* Money — the reason an owner opens this screen. The
                        balance is the biggest thing on the card because it is
                        the one number he is looking for. */}
                    <div className="grid gap-4 border-b border-line bg-sunken/40 px-4 py-4 sm:grid-cols-3">
                      <Figure
                        label={t("enrollments.totalDue")}
                        extra={
                          <>
                            {/* Called out separately because it is not the
                                school's money — it is collected on the day
                                and handed to the examiner. An owner reading
                                his own total should not count it as takings. */}
                            {summary.charges.examFees > 0 && (
                              <p className="mt-1 text-xs text-muted">
                                {t("enrollments.includesExamFees")}{" "}
                                <Money centimes={summary.charges.examFees} />
                              </p>
                            )}
                            {summary.charges.extraHours > 0 && (
                              <p className="mt-1 text-xs text-warn">
                                {t("enrollments.includesExtraHours")}{" "}
                                <Money centimes={summary.charges.extraHours} />
                              </p>
                            )}
                            {summary.charges.discount > 0 && (
                              <p className="mt-1 text-xs text-muted">
                                {t("enrollments.discount")}{" "}
                                <Money centimes={summary.charges.discount} />
                              </p>
                            )}
                          </>
                        }
                      >
                        <Money centimes={summary.totalDueCentimes} />
                      </Figure>

                      <Figure label={t("enrollments.paid")}>
                        <Money centimes={summary.paidCentimes} />
                      </Figure>

                      {/* Paid more than the price (an advance, or a price
                          corrected downwards): said in words, not as a
                          green "-5 000 DA" that reads like a typo. */}
                      {summary.balanceCentimes < 0 ? (
                        <Figure
                          label={t("enrollments.overpaid")}
                          strong
                          extra={
                            <p className="mt-1 text-xs text-muted">
                              {t("enrollments.overpaidHint")}
                            </p>
                          }
                        >
                          <Money centimes={-summary.balanceCentimes} />
                        </Figure>
                      ) : (
                        <Figure
                          label={t("enrollments.balance")}
                          strong
                          // Not red once he has dropped out: nobody is chasing it.
                          tone={summary.enrollment.status === "withdrawn" ? undefined : owes ? "owed" : "ok"}
                          extra={
                            // Where the debt comes from: the registration, or
                            // an exam fee not yet paid from the sitting. Only
                            // the parts that are owed; Rust makes them add up
                            // to the figure above.
                            owes && (
                              <p className="mt-1 text-xs text-muted">
                                {summary.owed.training > 0 && (
                                  <span className="inline-block me-3">
                                    {t("enrollments.owedTraining")}{" "}
                                    <Money centimes={summary.owed.training} className="font-medium text-ink-soft" />
                                  </span>
                                )}
                                {summary.owed.exams > 0 && (
                                  <span className="inline-block">
                                    {t("enrollments.owedExams")}{" "}
                                    <Money centimes={summary.owed.exams} className="font-medium text-owed" />
                                  </span>
                                )}
                              </p>
                            )
                          }
                        >
                          <Money centimes={summary.balanceCentimes} />
                        </Figure>
                      )}
                    </div>

                    {/* The actions. The hour bars that used to head this
                        block are gone: they measured progress against a
                        30-hour allowance, and this school does not sell
                        hours. It trains a group week after week until the
                        instructor judges them ready for the exam, so a
                        percentage of a quota was a confident answer to a
                        question nobody asks. What the candidate has actually
                        done is the séance log below. */}
                    <div className="px-4 py-4">
                      <div className="flex flex-wrap gap-2">
                        <Button
                          variant="primary"
                          size="sm"
                          icon={<Banknote className="size-3.5" aria-hidden="true" />}
                          onClick={() => setPayingFor(summary)}
                        >
                          {t("payments.take")}
                        </Button>
                        <Button
                          size="sm"
                          icon={<CalendarPlus className="size-3.5" aria-hidden="true" />}
                          onClick={() => setBookingFor(id)}
                        >
                          {t("calendar.book")}
                        </Button>
                        <Button
                          size="sm"
                          icon={<Plus className="size-3.5" aria-hidden="true" />}
                          onClick={() => setAddingHoursTo(id)}
                        >
                          {t("lessons.add")}
                        </Button>
                      </div>

                    </div>
                  </Card>

                  {/* Exams, above the lesson log on purpose: "where is this
                      candidate up to" is the question, and twenty rows of
                      séances is the answer to a different one. */}
                  <Card>
                    <ExamHistory
                      results={examsBy[id] ?? []}
                      progress={examProgressBy[id]}
                    />
                  </Card>

                  {/* Séances. Both lists can say "Code", so they are kept
                      apart and titled. */}
                  <Card>
                      <h3 className="mb-2.5 flex items-center gap-2 text-xs font-medium tracking-wide text-muted uppercase">
                        <CalendarCheck className="size-3.5" aria-hidden="true" />
                        {t("lessons.title")}
                        {lessons.length > 0 && (
                          <span className="rounded-full bg-sunken px-2 py-0.5 tracking-normal">
                            <Num>{lessons.length}</Num>
                          </span>
                        )}
                      </h3>

                      {/* A candidate trained over months has dozens of
                          séances; past a page they push the payments off
                          the screen. Upcoming ones sort first, so page one
                          is always what is next. */}
                      {lessons.length === 0 ? (
                        <p className="text-xs text-muted">{t("lessons.none")}</p>
                      ) : (
                        <Paged items={lessons} pageSize={LESSON_PAGE_SIZE}>
                          {(pageLessons) => (
                        <table className="w-full text-xs">
                          <thead>
                            <tr className="text-muted">
                              <th className="py-1 text-start font-medium">{t("lessons.colDate")}</th>
                              <th className="py-1 text-start font-medium">{t("lessons.colKind")}</th>
                              <th className="py-1 text-start font-medium">{t("lessons.colLength")}</th>
                              <th className="py-1 text-start font-medium">{t("lessons.colStatus")}</th>
                              <th className="py-1 font-medium" />
                            </tr>
                          </thead>
                          <tbody>
                            {pageLessons.map((lesson) => {
                              const upcoming = lesson.status === "planned";
                              return (
                                <tr
                                  key={lesson.id}
                                  className={[
                                    "group border-t border-line align-middle",
                                    upcoming ? "bg-brand-soft/40" : "",
                                  ].join(" ")}
                                >
                                  <td className="py-2">
                                    <DateText
                                      className="font-medium text-ink"
                                      value={
                                        upcoming
                                          ? (lesson.scheduledOn ?? lesson.occurredOn)
                                          : lesson.occurredOn
                                      }
                                    />
                                    {upcoming && lesson.scheduledTime && (
                                      <span className="ms-1.5 text-muted">
                                        <Num>{lesson.scheduledTime}</Num>
                                      </span>
                                    )}
                                  </td>
                                  <td className="py-2 text-ink-soft">
                                    {t(`lessons.kind_${lesson.kind}`)}
                                  </td>
                                  <td className="py-2 text-ink-soft">
                                    <Duration minutes={lesson.minutes} />
                                  </td>
                                  <td className="py-2">
                                    <Badge tone={STATUS_TONE[lesson.status] ?? "neutral"}>
                                      {t(`calendar.status_${lesson.status}`)}
                                    </Badge>
                                    {/* The note used to be written and never shown. */}
                                    {(lesson.notes || lesson.planNote) && (
                                      <span className="ms-2">
                                        <FreeText as="span" className="text-muted">
                                          {lesson.notes || lesson.planNote}
                                        </FreeText>
                                      </span>
                                    )}
                                  </td>
                                  <td className="py-2">
                                    <div className="flex items-center justify-end gap-1 opacity-0 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100">
                                      {/* A make-up, for somebody who did not
                                          turn up. Offered here as well as on
                                          the day view because a session
                                          missed last week is only reachable
                                          there by walking the calendar back
                                          to the day it was missed - which is
                                          the same reason the dashboard card
                                          exists. */}
                                      {lesson.status === "missed" && (
                                        <Button
                                          size="sm"
                                          icon={
                                            <CalendarPlus
                                              className="size-3.5"
                                              aria-hidden="true"
                                            />
                                          }
                                          onClick={() => {
                                            setMakeUpKind(lesson.kind);
                                            setBookingFor(summary.enrollment.id);
                                          }}
                                        >
                                          {t("calendar.rebook")}
                                        </Button>
                                      )}

                                      {/* Absent and Annulé can be taken back,
                                          and so can a booked session's Fait. */}
                                      {(lesson.status === "missed" ||
                                        lesson.status === "cancelled" ||
                                        (lesson.status === "done" &&
                                          lesson.scheduledOn !== null)) && (
                                        <Button
                                          size="sm"
                                          variant="ghost"
                                          icon={<Undo2 className="size-3.5" aria-hidden="true" />}
                                          onClick={() =>
                                            void reopenLesson(lesson.id)
                                              .then(refresh)
                                              .catch((e: unknown) => setError(errorText(e)))
                                          }
                                        >
                                          {t("calendar.reopen")}
                                        </Button>
                                      )}

                                      {/* Fait only once the day has come: on
                                          next week's row it credited the
                                          hours in advance. */}
                                      {upcoming &&
                                        (lesson.scheduledOn ?? lesson.occurredOn) <= todayIso() && (
                                        <Button
                                          size="sm"
                                          icon={<Check className="size-3.5" aria-hidden="true" />}
                                          onClick={() =>
                                            void completeLesson(lesson.id, {
                                              occurredOn: "",
                                              minutes: 0,
                                              notes: "",
                                            })
                                              .then(refresh)
                                              .catch((e: unknown) => setError(errorText(e)))
                                          }
                                        >
                                          {t("calendar.markDone")}
                                        </Button>
                                      )}
                                      <IconButton
                                        aria-label={t("common.delete")}
                                        className="size-8 text-owed hover:bg-owed-soft"
                                        onClick={() =>
                                          void deleteLesson(lesson.id)
                                            .then(refresh)
                                            .catch((e: unknown) => setError(errorText(e)))
                                        }
                                      >
                                        <Trash2 className="size-3.5" aria-hidden="true" />
                                      </IconButton>
                                    </div>
                                  </td>
                                </tr>
                              );
                            })}
                          </tbody>
                        </table>
                          )}
                        </Paged>
                      )}
                  </Card>

                  {/* Payments */}
                  <Card>
                      <h3 className="mb-2.5 flex items-center gap-2 text-xs font-medium tracking-wide text-muted uppercase">
                        <CreditCard className="size-3.5" aria-hidden="true" />
                        {t("payments.title")}
                      </h3>

                      {payments.length === 0 ? (
                        <p className="text-xs text-muted">{t("payments.none")}</p>
                      ) : (
                        <Paged items={payments} pageSize={PAYMENT_PAGE_SIZE}>
                          {(pagePayments) => (
                        <table className="w-full text-xs">
                          <thead>
                            <tr className="text-muted">
                              <th className="py-1 text-start font-medium">
                                {t("payments.date")}
                              </th>
                              <th className="py-1 text-start font-medium">
                                {t("payments.method")}
                              </th>
                              <th className="py-1 text-end font-medium">
                                {t("payments.amount")}
                              </th>
                              <th className="py-1 text-end font-medium" />
                            </tr>
                          </thead>
                          <tbody>
                            {pagePayments.map((payment) => {
                              const isReversal = payment.reversesPaymentId !== null;
                              // From every payment, not this page's: the
                              // reversal may sit on another page of the list.
                              const reversed = payments.some(
                                (p) => p.reversesPaymentId === payment.id,
                              );
                              return (
                                <tr key={payment.id} className="group border-t border-line">
                                  <td className="py-2 text-ink-soft">
                                    <DateText value={payment.paidOn} />
                                  </td>
                                  <td className="py-2 text-ink-soft">
                                    {t(`methods.${payment.methodCode}`, {
                                      defaultValue: payment.methodCode,
                                    })}
                                    {payment.examFeeChargeId !== null && (
                                      <span className="ms-1 text-brand">
                                        · {t("payments.forExam")}
                                      </span>
                                    )}
                                    {isReversal && (
                                      <span className="ms-1 text-warn">
                                        ({t("payments.reversal")})
                                      </span>
                                    )}
                                  </td>
                                  <td className="py-2 text-end">
                                    <Money
                                      centimes={payment.amountCentimes}
                                      className={
                                        isReversal ? "text-warn" : "font-medium text-ink"
                                      }
                                    />
                                  </td>
                                  <td className="py-2">
                                    <div className="flex items-center justify-end gap-1">
                                      {!isReversal && !reversed && (
                                        <IconButton
                                          aria-label={t("payments.reverse")}
                                          className="size-8 text-owed hover:bg-owed-soft"
                                          onClick={() => setReversing(payment)}
                                        >
                                          <Undo2 className="size-3.5" aria-hidden="true" />
                                        </IconButton>
                                      )}
                                    </div>
                                  </td>
                                </tr>
                              );
                            })}
                          </tbody>
                        </table>
                          )}
                        </Paged>
                      )}
                      <p className="mt-2.5 text-xs text-faint">{t("payments.appendOnlyNote")}</p>
                  </Card>
                  </div>
                );
              })}
            </div>
          </>
        )}
      </div>

      {newEnrollment && (
        <NewEnrollmentDialog
          candidateId={candidateId}
          onClose={() => setNewEnrollment(false)}
          onDone={() => {
            setNewEnrollment(false);
            void refresh();
          }}
        />
      )}

      {bookingFor !== null && (
        <ScheduleSessionDialog
          enrollmentId={bookingFor}
          {...(makeUpKind ? { initialKind: makeUpKind } : {})}
          onClose={() => {
            setBookingFor(null);
            setMakeUpKind(null);
          }}
          onDone={() => {
            setBookingFor(null);
            setMakeUpKind(null);
            void refresh();
          }}
        />
      )}

      {addingHoursTo !== null && (
        <AddLessonDialog
          enrollmentId={addingHoursTo}
          onClose={() => setAddingHoursTo(null)}
          onDone={() => {
            setAddingHoursTo(null);
            void refresh();
          }}
        />
      )}

      {reversing && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-ink/40 p-6 backdrop-blur-[2px]">
          <div
            role="dialog"
            aria-modal="true"
            className="w-full max-w-md rounded-lg border border-line bg-surface p-5 shadow-overlay"
          >
            <h2 className="text-base font-semibold tracking-tight text-ink">
              {t("payments.reverseTitle")}
            </h2>
            <p className="mt-2 text-sm text-ink-soft">
              <DateText value={reversing.paidOn} /> · <Money centimes={reversing.amountCentimes} />
            </p>
            <p className="mt-2 text-xs text-muted">{t("payments.reverseBody")}</p>
            <div className="mt-5 flex gap-2">
              <Button
                variant="primary"
                className="bg-owed hover:bg-owed"
                icon={<Undo2 className="size-4" aria-hidden="true" />}
                onClick={() => void doReverse(reversing)}
              >
                {t("payments.reverseConfirm")}
              </Button>
              <Button onClick={() => setReversing(null)}>{t("payments.reverseKeep")}</Button>
            </div>
          </div>
        </div>
      )}

      {editing && (
        <EditEnrollmentDialog
          enrollment={editing}
          onClose={() => setEditing(null)}
          onDone={() => {
            setEditing(null);
            void refresh();
          }}
        />
      )}

      {payingFor && (
        <TakePaymentDialog
          enrollmentId={payingFor.enrollment.id}
          balanceCentimes={payingFor.owed.training}
          examFeesOwedCentimes={payingFor.owed.exams}
          onClose={() => setPayingFor(null)}
          onDone={onPaymentTaken}
        />
      )}
    </>
  );
}
