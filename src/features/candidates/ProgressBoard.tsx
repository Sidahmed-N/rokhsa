import { useCallback, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { Phone } from "lucide-react";

import { Num } from "@/components/Num";
import { Badge, Card, ErrorNote } from "@/components/ui/Surface";
import { currentLanguage } from "@/i18n";
import { isAppError } from "@/lib/candidates";
import { getExamBoard, type BoardCard } from "@/lib/exams";
import { kindStyle } from "@/lib/kinds";
import { personName } from "@/lib/names";
import { isDesktop } from "@/lib/tauri";

/**
 * Where everyone is, in one screen.
 *
 * The list answers "find me Benaïssa"; this answers "who do I put in the
 * next code batch, who has been stuck on the créneau for two months, and
 * who has finished and is no longer my problem".
 *
 * A card is an ENROLMENT, not a candidate. Someone training for B and C1 is
 * at a different stage in each and appears twice — surprising the first
 * time, but the alternative is showing one of their two stages and hiding
 * the other.
 *
 * Nothing here is draggable. The stage is derived from exam results, so the
 * only honest way to move a card is to record a result; a drag would have to
 * invent one. Moving a candidate on happens on the Examens screen, where the
 * thing being recorded is what actually happened.
 */

/** The columns, in the order of the journey. */
const COLUMNS = ["theory", "maneuvers", "road", "awaiting", "finished"] as const;
type Column = (typeof COLUMNS)[number];

/**
 * Which column a card belongs in.
 *
 * Withdrawn is decided by the enrolment's own status, not by the results: a
 * candidate who stopped coming has not failed anything, the school simply
 * gave up on them, and nothing in their exam history says so.
 */
function columnFor(card: BoardCard): Column | "withdrawn" {
  if (card.enrollmentStatus === "withdrawn") return "withdrawn";
  if (card.progress.finished) return "finished";

  const due = card.progress.nextDue;
  if (due === null) return "finished";

  // Presented and waiting for the result is its own state: there is nothing
  // for the school to do, which is exactly why it should not sit in the
  // column of people who need teaching.
  const standing =
    due === "theory"
      ? card.progress.theory
      : due === "maneuvers"
        ? card.progress.maneuvers
        : card.progress.road;

  if (standing === "awaiting") return "awaiting";
  return due as Column;
}

/** Quiet after a month, loud after two — a candidate who has stalled. */
function stalled(days: number | null): boolean {
  return days !== null && days >= 60;
}

export default function ProgressBoard() {
  const { t } = useTranslation();
  const locale = currentLanguage();

  const [cards, setCards] = useState<BoardCard[]>([]);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    if (!isDesktop()) {
      setError(t("candidates.needsDesktop"));
      return;
    }
    try {
      setCards(await getExamBoard());
      setError(null);
    } catch (raw: unknown) {
      setError(isAppError(raw) ? t(raw.code, { defaultValue: raw.message }) : String(raw));
    }
  }, [t]);

  useEffect(() => {
    void load();
  }, [load]);

  const withdrawn = cards.filter((c) => columnFor(c) === "withdrawn");

  return (
    <div>
      {error && <ErrorNote>{error}</ErrorNote>}

      <div className="grid gap-3 lg:grid-cols-5">
        {COLUMNS.map((column) => {
          const mine = cards.filter((c) => columnFor(c) === column);
          const isStage = column !== "awaiting" && column !== "finished";

          return (
            <section key={column} className="min-w-0">
              <header className="mb-2 flex items-center gap-2 px-1">
                {isStage && (
                  <span
                    className={["size-2 rounded-full", kindStyle(column).dot].join(" ")}
                    aria-hidden="true"
                  />
                )}
                <h3 className="truncate text-xs font-semibold tracking-wide text-ink uppercase">
                  {isStage ? t(`lessons.kind_${column}`) : t(`board.col_${column}`)}
                </h3>
                <span className="ms-auto rounded-full bg-sunken px-1.5 text-xs font-medium text-muted">
                  <Num>{mine.length}</Num>
                </span>
              </header>

              <div className="grid gap-2">
                {mine.length === 0 && (
                  <p className="rounded-lg border border-dashed border-line px-2 py-5 text-center text-xs text-faint">
                    {t("board.columnEmpty")}
                  </p>
                )}

                {mine.map((card) => (
                  <Card key={card.enrollmentId} className="p-2.5">
                    <Link
                      to={`/candidates/${card.candidateId}`}
                      className="block truncate text-sm font-medium text-ink hover:text-brand hover:underline"
                    >
                      {personName(card, locale)}
                    </Link>

                    <div className="mt-1 flex flex-wrap items-center gap-x-2 gap-y-1">
                      {/* A second go at the exam they are due. */}
                      {column === "maneuvers" && card.progress.maneuversAttempts > 0 && (
                        <Badge tone="warn">
                          {t("board.retake", { n: card.progress.maneuversAttempts + 1 })}
                        </Badge>
                      )}
                      {column === "road" && card.progress.roadAttempts > 0 && (
                        <Badge tone="warn">
                          {t("board.retake", { n: card.progress.roadAttempts + 1 })}
                        </Badge>
                      )}
                      {column === "theory" && card.progress.theoryAttempts > 0 && (
                        <Badge tone="warn">
                          {t("board.retake", { n: card.progress.theoryAttempts + 1 })}
                        </Badge>
                      )}
                      {stalled(card.daysSinceExam) && (
                        <Badge tone="owed">
                          <Num>{card.daysSinceExam}</Num> {t("receivables.daysAgo")}
                        </Badge>
                      )}
                    </div>

                    {card.phone && (
                      <span className="mt-1 flex items-center gap-1 text-xs text-muted">
                        <Phone className="size-3 text-faint" aria-hidden="true" />
                        <Num>{card.phone}</Num>
                      </span>
                    )}
                  </Card>
                ))}
              </div>
            </section>
          );
        })}
      </div>

      {/* Abandons are kept visible but out of the way: the owner still wants
          to see them, and they are not part of the pipeline. */}
      {withdrawn.length > 0 && (
        <section className="mt-5">
          <h3 className="mb-2 px-1 text-xs font-semibold tracking-wide text-muted uppercase">
            {t("board.col_withdrawn")} <Num>{withdrawn.length}</Num>
          </h3>
          <div className="flex flex-wrap gap-2">
            {withdrawn.map((card) => (
              <Link
                key={card.enrollmentId}
                to={`/candidates/${card.candidateId}`}
                className="rounded-lg border border-line bg-surface px-2.5 py-1.5 text-xs text-muted shadow-card transition-colors hover:text-ink"
              >
                {personName(card, locale)}
              </Link>
            ))}
          </div>
        </section>
      )}
    </div>
  );
}
