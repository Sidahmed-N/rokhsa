import { useCallback, useEffect, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
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
  Printer,
  Trash2,
  Undo2,
} from "lucide-react";

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

function ProgressBar({ percent }: { percent: number | null }) {
  // percent can exceed 100 when a candidate has done more than the target;
  // the bar caps but the number next to it does not.
  const width = percent === null ? 0 : Math.min(100, Math.max(0, percent));
  const complete = percent !== null && percent >= 100;
  return (
    <div className="h-1.5 w-full overflow-hidden rounded-full bg-line">
      <div
        className={["h-full rounded-full", complete ? "bg-ok" : "bg-brand"].join(" ")}
        style={{ width: `${width}%` }}
      />
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
    <div>
      <div className="mb-1.5 flex items-baseline justify-between text-xs">
        <span className="font-medium text-ink-soft">{label}</span>
        <span className="text-ink">
          <Duration minutes={progress.doneMinutes} />
          {progress.targetMinutes > 0 && (
            <>
              <span className="text-faint"> / </span>
              <Duration minutes={progress.targetMinutes} />
            </>
          )}
          {progress.percent !== null && (
            // The "%" goes INSIDE the isolate. Left outside it, the bidi
            // algorithm puts it on the far side of the digits in Arabic and
            // "23%" renders as "%23".
            <span className="ms-2 text-muted">
              <Num>{progress.percent}%</Num>
            </span>
          )}
        </span>
      </div>
      <ProgressBar percent={progress.percent} />
    </div>
  );
}

/** One figure in the money strip. */
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
                        {summary.enrollment.categoryCode}
                        <span className="text-faint"> — </span>
                        {locale === "ar" ? summary.categoryNameAr : summary.categoryNameFr}
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

                    {/* Hours */}
                    <div className="border-b border-line px-4 py-4">
                      <div className="grid gap-3 sm:grid-cols-2">
                        <HoursRow label={t("lessons.kind_theory")} progress={summary.theory} />
                        <HoursRow label={t("lessons.practical")} progress={summary.practical} />
                      </div>

                      <div className="mt-4 flex flex-wrap gap-2">
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

                      {lessons.length > 0 && (
                        <table className="mt-4 w-full text-xs">
                          <tbody>
                            {lessons.map((lesson) => {
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
                                      <span className="ms-2 text-muted">
                                        {lesson.notes || lesson.planNote}
                                      </span>
                                    )}
                                  </td>
                                  <td className="py-2">
                                    <div className="flex items-center justify-end gap-1 opacity-0 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100">
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
                        <table className="w-full text-xs">
                          <thead>
                            <tr className="text-muted">
                              <th className="py-1 text-start font-medium">
                                {t("payments.receiptNo")}
                              </th>
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
                            {payments.map((payment) => {
                              const isReversal = payment.reversesPaymentId !== null;
                              return (
                                <tr key={payment.id} className="group border-t border-line">
                                  <td className="py-2">
                                    <Num className="font-medium text-ink">
                                      {payment.receiptNumber}
                                    </Num>
                                  </td>
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
                                      <Link
                                        to={`/receipt/${payment.id}`}
                                        aria-label={t("common.print")}
                                        className="inline-flex size-8 items-center justify-center rounded-md text-ink-soft transition-colors hover:bg-sunken hover:text-ink"
                                      >
                                        <Printer className="size-3.5" aria-hidden="true" />
                                      </Link>
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
