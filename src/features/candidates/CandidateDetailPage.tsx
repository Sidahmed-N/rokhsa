import { useCallback, useEffect, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { useTranslation } from "react-i18next";

import { Duration, Money, Num } from "@/components/Num";
import { currentLanguage } from "@/i18n";
import {
  displayName,
  getCandidate,
  isAppError,
  type Candidate,
} from "@/lib/candidates";
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
  AddLessonDialog,
  NewEnrollmentDialog,
  ScheduleSessionDialog,
  TakePaymentDialog,
} from "@/features/training/TrainingDialogs";

function ProgressBar({ percent }: { percent: number | null }) {
  // percent can exceed 100 when a candidate has done more than the target;
  // the bar caps but the number next to it does not.
  const width = percent === null ? 0 : Math.min(100, Math.max(0, percent));
  return (
    <div className="h-1.5 w-full overflow-hidden rounded-full bg-line">
      <div className="h-full rounded-full bg-brand" style={{ width: `${width}%` }} />
    </div>
  );
}

function HoursRow({
  label,
  progress,
}: {
  label: string;
  progress: { doneMinutes: number; targetMinutes: number; percent: number | null };
}) {
  return (
    <div className="mb-2">
      <div className="mb-1 flex items-baseline justify-between text-xs">
        <span className="text-muted">{label}</span>
        <span>
          <Duration minutes={progress.doneMinutes} />
          {progress.targetMinutes > 0 && (
            <>
              {" / "}
              <Duration minutes={progress.targetMinutes} />
            </>
          )}
          {progress.percent !== null && (
            <span className="ms-2 text-muted">
              <Num>{progress.percent}</Num>%
            </span>
          )}
        </span>
      </div>
      <ProgressBar percent={progress.percent} />
    </div>
  );
}

export default function CandidateDetailPage() {
  const { t } = useTranslation();
  const locale = currentLanguage();
  const params = useParams();
  const navigate = useNavigate();
  const candidateId = Number(params.id);

  const [candidate, setCandidate] = useState<Candidate | null>(null);
  const [summaries, setSummaries] = useState<EnrollmentSummary[]>([]);
  const [lessonsBy, setLessonsBy] = useState<Record<number, Lesson[]>>({});
  const [paymentsBy, setPaymentsBy] = useState<Record<number, Payment[]>>({});
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const [newEnrollment, setNewEnrollment] = useState(false);
  const [addingHoursTo, setAddingHoursTo] = useState<number | null>(null);
  const [bookingFor, setBookingFor] = useState<number | null>(null);
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
      setLessonsBy(Object.fromEntries(lessonEntries));
      setPaymentsBy(Object.fromEntries(paymentEntries));
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
   * Take payment, then go straight to the receipt and print it.
   *
   * The receipt lives at its own route, so this is the same code path a
   * reprint uses — there is no second copy of the receipt logic to drift.
   */
  function onPaymentTaken(payment: Payment) {
    setPayingFor(null);
    navigate(`/receipt/${payment.id}?autoprint=1`);
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
        <Link to="/candidates" className="text-sm text-brand hover:underline">
          ← {t("candidates.title")}
        </Link>

        {error && (
          <p className="mt-3 rounded-md border border-owed bg-white px-3 py-2 text-sm text-owed">
            {error}
          </p>
        )}

        {loading && <p className="mt-4 text-sm text-muted">{t("common.loading")}</p>}

        {candidate && (
          <>
            <header className="mt-3 mb-5">
              <h1 className="text-xl font-semibold">{displayName(candidate, locale)}</h1>
              <p className="mt-1 text-sm text-muted">
                {candidate.phone && <Num>{candidate.phone}</Num>}
                {candidate.nin && (
                  <>
                    {candidate.phone && " · "}
                    {t("candidates.nin")}: <Num>{candidate.nin}</Num>
                  </>
                )}
              </p>
            </header>

            {summaries.length === 0 && (
              <p className="mb-4 rounded-md border border-line bg-white px-3 py-3 text-sm text-muted">
                {t("enrollments.none")}
              </p>
            )}

            <button
              type="button"
              onClick={() => setNewEnrollment(true)}
              className="mb-5 rounded-md bg-brand px-3 py-1.5 text-sm font-medium text-white"
            >
              {t("enrollments.new")}
            </button>

            {summaries.map((summary) => {
              const id = summary.enrollment.id;
              const lessons = lessonsBy[id] ?? [];
              const payments = paymentsBy[id] ?? [];
              const owes = summary.balanceCentimes > 0;

              return (
                <section key={id} className="mb-6 rounded-lg border border-line bg-white">
                  <header className="flex flex-wrap items-baseline gap-3 border-b border-line px-4 py-3">
                    <h2 className="text-base font-semibold">
                      {summary.enrollment.categoryCode} —{" "}
                      {locale === "ar" ? summary.categoryNameAr : summary.categoryNameFr}
                    </h2>
                    <span className="rounded-full bg-brand-soft px-2 py-0.5 text-xs text-brand">
                      {t(`enrollments.status_${summary.enrollment.status}`)}
                    </span>
                    <span className="ms-auto text-xs text-muted">
                      {t("enrollments.enrolledOn")}{" "}
                      <Num>{summary.enrollment.enrolledOn}</Num>
                    </span>
                  </header>

                  {/* Money — the reason an owner opens this screen */}
                  <div className="grid gap-4 border-b border-line px-4 py-4 sm:grid-cols-3">
                    <div>
                      <p className="text-xs text-muted">{t("enrollments.totalDue")}</p>
                      <Money centimes={summary.totalDueCentimes} className="text-sm font-medium" />
                      {summary.charges.extraHours > 0 && (
                        <p className="mt-0.5 text-xs text-warn">
                          {t("enrollments.includesExtraHours")}{" "}
                          <Money centimes={summary.charges.extraHours} />
                        </p>
                      )}
                      {summary.charges.discount > 0 && (
                        <p className="mt-0.5 text-xs text-muted">
                          {t("enrollments.discount")}{" "}
                          <Money centimes={summary.charges.discount} />
                        </p>
                      )}
                    </div>
                    <div>
                      <p className="text-xs text-muted">{t("enrollments.paid")}</p>
                      <Money centimes={summary.paidCentimes} className="text-sm font-medium" />
                    </div>
                    <div>
                      <p className="text-xs text-muted">{t("enrollments.balance")}</p>
                      <Money
                        centimes={summary.balanceCentimes}
                        className={[
                          "text-lg font-bold",
                          owes ? "text-owed" : "text-ok",
                        ].join(" ")}
                      />
                    </div>
                  </div>

                  {/* Hours */}
                  <div className="border-b border-line px-4 py-4">
                    <HoursRow label={t("lessons.kind_theory")} progress={summary.theory} />
                    <HoursRow label={t("lessons.practical")} progress={summary.practical} />

                    <div className="mt-3 flex gap-2">
                      <button
                        type="button"
                        onClick={() => setBookingFor(id)}
                        className="rounded-md border border-brand px-3 py-1.5 text-xs text-brand"
                      >
                        {t("calendar.book")}
                      </button>
                      <button
                        type="button"
                        onClick={() => setAddingHoursTo(id)}
                        className="rounded-md border border-line px-3 py-1.5 text-xs"
                      >
                        {t("lessons.add")}
                      </button>
                      <button
                        type="button"
                        onClick={() => setPayingFor(summary)}
                        className="rounded-md bg-brand px-3 py-1.5 text-xs font-medium text-white"
                      >
                        {t("payments.take")}
                      </button>
                    </div>

                    {lessons.length > 0 && (
                      <table className="mt-3 w-full text-xs">
                        <tbody>
                          {lessons.map((lesson) => {
                            const upcoming = lesson.status === "planned";
                            return (
                              <tr
                                key={lesson.id}
                                className={[
                                  "border-t border-line align-top",
                                  upcoming ? "bg-brand-soft/40" : "",
                                ].join(" ")}
                              >
                                <td className="py-1.5">
                                  <Num>
                                    {upcoming
                                      ? (lesson.scheduledOn ?? lesson.occurredOn)
                                      : lesson.occurredOn}
                                  </Num>
                                  {upcoming && lesson.scheduledTime && (
                                    <span className="ms-1 text-muted">
                                      <Num>{lesson.scheduledTime}</Num>
                                    </span>
                                  )}
                                </td>
                                <td className="py-1.5">{t(`lessons.kind_${lesson.kind}`)}</td>
                                <td className="py-1.5">
                                  <Duration minutes={lesson.minutes} />
                                </td>
                                <td className="py-1.5">
                                  <span
                                    className={
                                      lesson.status === "done"
                                        ? "text-ok"
                                        : upcoming
                                          ? "text-brand"
                                          : "text-warn"
                                    }
                                  >
                                    {t(`calendar.status_${lesson.status}`)}
                                  </span>
                                  {/* The note used to be written and never shown. */}
                                  {(lesson.notes || lesson.planNote) && (
                                    <span className="ms-2 text-muted">
                                      {lesson.notes || lesson.planNote}
                                    </span>
                                  )}
                                </td>
                                <td className="py-1.5 text-end">
                                  {upcoming && (
                                    <button
                                      type="button"
                                      onClick={() =>
                                        void completeLesson(lesson.id, {
                                          occurredOn: "",
                                          minutes: 0,
                                          notes: "",
                                        })
                                          .then(refresh)
                                          .catch((e: unknown) => setError(errorText(e)))
                                      }
                                      className="me-2 text-brand hover:underline"
                                    >
                                      {t("calendar.markDone")}
                                    </button>
                                  )}
                                  <button
                                    type="button"
                                    onClick={() =>
                                      void deleteLesson(lesson.id)
                                        .then(refresh)
                                        .catch((e: unknown) => setError(errorText(e)))
                                    }
                                    className="text-owed hover:underline"
                                  >
                                    {t("common.delete")}
                                  </button>
                                </td>
                              </tr>
                            );
                          })}
                        </tbody>
                      </table>
                    )}
                  </div>

                  {/* Payments */}
                  <div className="px-4 py-4">
                    <h3 className="mb-2 text-xs uppercase tracking-wide text-muted">
                      {t("payments.title")}
                    </h3>

                    {payments.length === 0 ? (
                      <p className="text-xs text-muted">{t("payments.none")}</p>
                    ) : (
                      <table className="w-full text-xs">
                        <thead>
                          <tr className="text-muted">
                            <th className="py-1 text-start font-medium">
                              {t("payments.receiptNo")}
                            </th>
                            <th className="py-1 text-start font-medium">{t("payments.date")}</th>
                            <th className="py-1 text-start font-medium">{t("payments.method")}</th>
                            <th className="py-1 text-end font-medium">{t("payments.amount")}</th>
                            <th className="py-1 text-end font-medium" />
                          </tr>
                        </thead>
                        <tbody>
                          {payments.map((payment) => {
                            const isReversal = payment.reversesPaymentId !== null;
                            return (
                              <tr key={payment.id} className="border-t border-line">
                                <td className="py-1.5">
                                  <Num>{payment.receiptNumber}</Num>
                                </td>
                                <td className="py-1.5">
                                  <Num>{payment.paidOn}</Num>
                                </td>
                                <td className="py-1.5">
                                  {t(`methods.${payment.methodCode}`, {
                                    defaultValue: payment.methodCode,
                                  })}
                                  {isReversal && (
                                    <span className="ms-1 text-warn">
                                      ({t("payments.reversal")})
                                    </span>
                                  )}
                                </td>
                                <td className="py-1.5 text-end">
                                  <Money
                                    centimes={payment.amountCentimes}
                                    className={isReversal ? "text-warn" : ""}
                                  />
                                </td>
                                <td className="py-1.5 text-end">
                                  <Link
                                    to={`/receipt/${payment.id}`}
                                    className="text-brand hover:underline"
                                  >
                                    {t("common.print")}
                                  </Link>
                                  {!isReversal && (
                                    <button
                                      type="button"
                                      onClick={() => void doReverse(payment)}
                                      className="ms-2 text-owed hover:underline"
                                    >
                                      {t("payments.reverse")}
                                    </button>
                                  )}
                                </td>
                              </tr>
                            );
                          })}
                        </tbody>
                      </table>
                    )}
                    <p className="mt-2 text-xs text-muted">{t("payments.appendOnlyNote")}</p>
                  </div>
                </section>
              );
            })}
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
          onClose={() => setBookingFor(null)}
          onDone={() => {
            setBookingFor(null);
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
