import { useCallback, useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { useTranslation } from "react-i18next";
import {
  BadgeCheck,
  Banknote,
  CalendarPlus,
  Check,
  ChevronLeft,
  CreditCard,
  IdCard,
  Phone,
  Plus,
  Trash2,
  Undo2,
} from "lucide-react";

import FreeText from "@/components/FreeText";
import { Paged } from "@/components/ui/Pager";
import ExamHistory from "@/features/candidates/ExamHistory";
import { Duration, Money, Num } from "@/components/Num";
import Button, { IconButton } from "@/components/ui/Button";
import { Badge, Card, EmptyState, ErrorNote, type Tone } from "@/components/ui/Surface";
import { currentLanguage } from "@/i18n";
import { displayName, getCandidate, isAppError, type Candidate } from "@/lib/candidates";
import { isDesktop } from "@/lib/tauri";
import {
  completeLesson,
  deleteLesson,
  listEnrollments,
  listLessons,
  listPayments,
  reversePayment,
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
  const locale = currentLanguage();
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

  async function doReverse(payment: Payment) {
    const reason = t("payments.reversedByUser");
    try {
      await reversePayment(payment.id, reason);
      await refresh();
    } catch (raw: unknown) {
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
            <header className="mt-3 mb-5 flex flex-wrap items-center gap-x-4 gap-y-2">
              <div>
                <h1 className="text-[1.375rem] font-semibold tracking-tight text-ink">
                  {displayName(candidate, locale)}
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

              <Button
                className="ms-auto"
                variant="primary"
                icon={<Plus className="size-4" aria-hidden="true" />}
                onClick={() => setNewEnrollment(true)}
              >
                {t("enrollments.new")}
              </Button>
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

                return (
                  <Card key={id} flush>
                    <header className="flex flex-wrap items-center gap-3 border-b border-line px-4 py-3">
                      <h2 className="text-base font-semibold tracking-tight text-ink">
                        {t("enrollments.heading")}
                      </h2>
                      <Badge tone="brand">
                        {t(`enrollments.status_${summary.enrollment.status}`)}
                      </Badge>
                      <span className="ms-auto text-xs text-muted">
                        {t("enrollments.enrolledOn")} <Num>{summary.enrollment.enrolledOn}</Num>
                      </span>
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

                      <Figure
                        label={t("enrollments.balance")}
                        strong
                        tone={owes ? "owed" : "ok"}
                      >
                        <Money centimes={summary.balanceCentimes} />
                      </Figure>
                    </div>

                    {/* The actions. The hour bars that used to head this
                        block are gone: they measured progress against a
                        30-hour allowance, and this school does not sell
                        hours. It trains a group week after week until the
                        instructor judges them ready for the exam, so a
                        percentage of a quota was a confident answer to a
                        question nobody asks. What the candidate has actually
                        done is the séance log below. */}
                    <div className="border-b border-line px-4 py-4">
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

                      {/* Exams, above the lesson log on purpose: "where is
                          this candidate up to" is the question, and twenty
                          rows of séances is the answer to a different one. */}
                      <div className="mt-4 border-t border-line pt-4">
                        <ExamHistory
                          results={examsBy[id] ?? []}
                          progress={examProgressBy[id]}
                        />
                      </div>

                      {/* A candidate trained over months has dozens of
                          séances; past a page they push the payments off
                          the screen. Upcoming ones sort first, so page one
                          is always what is next. */}
                      {lessons.length > 0 && (
                        <Paged items={lessons} pageSize={LESSON_PAGE_SIZE}>
                          {(pageLessons) => (
                        <table className="mt-4 w-full text-xs">
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
                                    <Num className="font-medium text-ink">
                                      {upcoming
                                        ? (lesson.scheduledOn ?? lesson.occurredOn)
                                        : lesson.occurredOn}
                                    </Num>
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

                                      {upcoming && (
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
                    </div>

                    {/* Payments */}
                    <div className="px-4 py-4">
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
                              return (
                                <tr key={payment.id} className="group border-t border-line">
                                  <td className="py-2 text-ink-soft">
                                    <Num>{payment.paidOn}</Num>
                                  </td>
                                  <td className="py-2 text-ink-soft">
                                    {t(`methods.${payment.methodCode}`, {
                                      defaultValue: payment.methodCode,
                                    })}
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
                                      {!isReversal && (
                                        <IconButton
                                          aria-label={t("payments.reverse")}
                                          className="size-8 text-owed hover:bg-owed-soft"
                                          onClick={() => void doReverse(payment)}
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
                    </div>
                  </Card>
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

      {payingFor && (
        <TakePaymentDialog
          enrollmentId={payingFor.enrollment.id}
          balanceCentimes={payingFor.balanceCentimes}
          onClose={() => setPayingFor(null)}
          onDone={onPaymentTaken}
        />
      )}
    </>
  );
}
