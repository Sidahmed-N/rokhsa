import { useTranslation } from "react-i18next";
import { Check, Clock, GraduationCap, Minus, X } from "lucide-react";

import FreeText from "@/components/FreeText";
import { Num } from "@/components/Num";
import { Badge, type Tone } from "@/components/ui/Surface";
import { EXAM_KINDS, type ExamProgress, type ExamResult, type Standing } from "@/lib/exams";
import { kindStyle } from "@/lib/kinds";

/**
 * One candidate's exam record.
 *
 * Two halves, because they answer different questions. The strip across the
 * top is "where are they now" — the thing someone standing at the desk is
 * asking. The rows underneath are "how did they get here", which matters
 * when a candidate argues about a repassage or the owner wants to know how
 * many goes it took.
 *
 * Nothing here is editable. A result is recorded against the sitting it
 * belongs to, on the Examens screen, because that is where the batch is and
 * where the date comes from. Letting it be typed here as well would be a
 * second way to create the same row.
 */

const STANDING_TONE: Record<Standing, Tone> = {
  notSat: "neutral",
  awaiting: "brand",
  failed: "owed",
  passed: "ok",
};

const OUTCOME_TONE: Record<string, Tone> = {
  presented: "brand",
  passed: "ok",
  failed: "owed",
  absent: "warn",
};

function StandingIcon({ standing }: { standing: Standing }) {
  const className = "size-3.5";
  if (standing === "passed") return <Check className={className} aria-hidden="true" />;
  if (standing === "failed") return <X className={className} aria-hidden="true" />;
  if (standing === "awaiting") return <Clock className={className} aria-hidden="true" />;
  return <Minus className={className} aria-hidden="true" />;
}

export default function ExamHistory({
  results,
  progress,
}: {
  results: ExamResult[];
  progress: ExamProgress | undefined;
}) {
  const { t } = useTranslation();

  const standingOf = (kind: string): Standing => {
    if (!progress) return "notSat";
    if (kind === "theory") return progress.theory;
    if (kind === "maneuvers") return progress.maneuvers;
    return progress.road;
  };

  const attemptsOf = (kind: string): number => {
    if (!progress) return 0;
    if (kind === "theory") return progress.theoryAttempts;
    if (kind === "maneuvers") return progress.maneuversAttempts;
    return progress.roadAttempts;
  };

  return (
    <div>
      <h3 className="mb-2.5 flex flex-wrap items-center gap-2 text-xs font-medium tracking-wide text-muted uppercase">
        <GraduationCap className="size-3.5" aria-hidden="true" />
        {t("exams.title")}
        {progress?.finished && <Badge tone="ok">{t("board.col_finished")}</Badge>}
        {progress && !progress.finished && progress.nextDue && (
          <span className="font-normal normal-case">
            {t("exams.nextDue")}{" "}
            <span className="font-medium text-ink">
              {t(`lessons.kind_${progress.nextDue}`)}
            </span>
          </span>
        )}
      </h3>

      {/* Where they are now: the three stages at a glance. */}
      <div className="grid gap-2 sm:grid-cols-3">
        {EXAM_KINDS.map((kind) => {
          const standing = standingOf(kind);
          const attempts = attemptsOf(kind);
          return (
            <div
              key={kind}
              className={[
                "flex items-center gap-2 rounded-lg px-2.5 py-2",
                standing === "notSat" ? "bg-sunken" : kindStyle(kind).chip,
              ].join(" ")}
            >
              <span className="text-xs font-semibold">{t(`lessons.kind_${kind}`)}</span>
              <Badge tone={STANDING_TONE[standing]} className="ms-auto">
                <span className="flex items-center gap-1">
                  <StandingIcon standing={standing} />
                  {t(`exams.standing_${standing}`)}
                </span>
              </Badge>
              {/* Only worth saying once it took more than one go. */}
              {attempts > 1 && (
                <span className="text-xs opacity-70">
                  <Num>{attempts}</Num>×
                </span>
              )}
            </div>
          );
        })}
      </div>

      {/* How they got here. */}
      {results.length === 0 ? (
        <p className="mt-3 text-xs text-muted">{t("exams.noneForCandidate")}</p>
      ) : (
        <table className="mt-3 w-full text-xs">
          <tbody>
            {results.map((row) => (
              <tr key={row.id} className="border-t border-line">
                <td className="py-2">
                  <Num className="font-medium text-ink">{row.satOn}</Num>
                </td>
                <td className="py-2 text-ink-soft">{t(`lessons.kind_${row.kind}`)}</td>
                <td className="py-2">
                  {row.attempt > 1 && (
                    <span className="text-muted">
                      {t("exams.attempt", { n: row.attempt })}
                    </span>
                  )}
                </td>
                <td className="py-2">
                  <Badge tone={OUTCOME_TONE[row.outcome] ?? "neutral"}>
                    {t(`exams.outcome_${row.outcome}`)}
                  </Badge>
                </td>
                <td className="py-2">
                  {row.place && (
                    <FreeText as="span" className="text-muted">
                      {row.place}
                    </FreeText>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
