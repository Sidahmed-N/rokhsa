import { useCallback, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { Archive, Phone } from "lucide-react";

import FreeText from "@/components/FreeText";
import Modal from "@/components/Modal";
import { Num } from "@/components/Num";
import { Badge, Card, ErrorNote } from "@/components/ui/Surface";
import { isAppError, setCandidatesArchived } from "@/lib/candidates";
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
 * at a different stage in each and appears twice.
 *
 * READ ONLY. The column is derived from exam results, which are recorded on
 * the Examens screen, in a sitting. Cards used to be draggable, and a drop
 * recorded the result itself (a pass, a presentation) with no sitting, no
 * place and no exam fee: the candidate's file said "Créneau" while Examens
 * showed nothing, and the fee was never asked for. Two ways to record one
 * exam disagreed, so the owner removed the second one (2026-11-05).
 */

/** The columns, in the order of the journey. */
const COLUMNS = ["theory", "maneuvers", "road", "awaiting", "finished"] as const;

/** Quiet after a month, loud after two — a candidate who has stalled. */
function stalled(days: number | null): boolean {
  return days !== null && days >= 60;
}

export default function ProgressBoard() {
  const { t } = useTranslation();

  const [cards, setCards] = useState<BoardCard[]>([]);
  const [error, setError] = useState<string | null>(null);

  const errorText = useCallback(
    (raw: unknown) => (isAppError(raw) ? t(raw.code, { defaultValue: raw.message }) : String(raw)),
    [t],
  );

  const load = useCallback(async () => {
    if (!isDesktop()) {
      setError(t("candidates.needsDesktop"));
      return;
    }
    try {
      setCards(await getExamBoard());
      setError(null);
    } catch (raw: unknown) {
      setError(errorText(raw));
    }
  }, [t, errorText]);

  useEffect(() => {
    void load();
  }, [load]);

  const withdrawn = cards.filter((c) => c.column === "withdrawn");

  // Everyone who has finished, at once: that column only ever grows.
  const [archiving, setArchiving] = useState<BoardCard[] | null>(null);
  const [archiveBusy, setArchiveBusy] = useState(false);
  const [archiveError, setArchiveError] = useState<string | null>(null);

  async function archiveAll(finished: BoardCard[]) {
    setArchiveBusy(true);
    try {
      // A candidate with two enrolments would otherwise be sent twice.
      const ids = [...new Set(finished.map((c) => c.candidateId))];
      await setCandidatesArchived(ids, true);
      setArchiving(null);
      setArchiveError(null);
      void load();
    } catch (raw: unknown) {
      setArchiveError(errorText(raw));
    } finally {
      setArchiveBusy(false);
    }
  }

  return (
    <div>
      {error && <ErrorNote>{error}</ErrorNote>}

      <p className="mb-3 text-xs text-muted">{t("board.viewHint")}</p>

      <div className="grid gap-3 lg:grid-cols-5">
        {COLUMNS.map((column) => {
          const mine = cards.filter((c) => c.column === column);
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
                {column === "finished" && mine.length > 0 && (
                  <button
                    type="button"
                    onClick={() => setArchiving(mine)}
                    title={t("board.archiveTitle")}
                    className="inline-flex items-center gap-1 rounded-md px-1.5 py-0.5 text-xs font-medium text-muted transition-colors hover:bg-sunken hover:text-ink"
                  >
                    <Archive className="size-3.5" aria-hidden="true" />
                    {t("board.archiveFinished")}
                  </button>
                )}
              </header>

              <div className="grid gap-2">
                {mine.length === 0 && (
                  <p className="rounded-lg border border-dashed border-line px-2 py-5 text-center text-xs text-faint">
                    {t("board.columnEmpty")}
                  </p>
                )}

                {mine.map((card) => (
                  // The whole card opens the file. draggable={false}: a link
                  // drags its URL by default, and a card that seems to move
                  // would suggest the board records something.
                  <Link
                    key={card.enrollmentId}
                    to={`/candidates/${card.candidateId}`}
                    draggable={false}
                    className="group block rounded-lg"
                  >
                    <Card className="p-2.5 transition-colors group-hover:border-brand-line">
                      {/* The name is in whatever script the school typed. The
                          line keeps the page's direction so the name starts on
                          the page's side; the inner span takes the name's own
                          direction so a long one is clipped at its END. */}
                      <span className="block text-sm font-medium text-ink group-hover:text-brand">
                        <span dir="auto" className="inline-block max-w-full truncate align-top">
                          {personName(card)}
                        </span>
                      </span>

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
                  </Link>
                ))}
              </div>
            </section>
          );
        })}
      </div>

      {/* Abandons are kept visible but out of the way: the owner still wants
          to see them, and they are not part of the pipeline. Withdrawing is
          a decision made on the candidate's file. */}
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
                <FreeText as="span">{personName(card)}</FreeText>
              </Link>
            ))}
          </div>
        </section>
      )}

      {archiving && (
        <Modal
          title={t("board.archiveTitle")}
          onClose={() => {
            setArchiving(null);
            setArchiveError(null);
          }}
          onSubmit={() => void archiveAll(archiving)}
          submitLabel={t("board.archiveFinished")}
          submitting={archiveBusy}
          error={archiveError}
        >
          <p className="text-sm text-ink-soft">{t("board.archiveBody")}</p>
          {/* Each name on its own line, in its own script: inside a sentence
              the bidi algorithm would reorder it against the words around it. */}
          <ul className="mt-3 max-h-56 overflow-y-auto rounded-md border border-line bg-sunken px-3 py-2 text-sm text-ink">
            {archiving.map((card) => (
              <li key={card.enrollmentId} className="truncate py-0.5">
                <span dir="auto">{personName(card)}</span>
              </li>
            ))}
          </ul>
        </Modal>
      )}

    </div>
  );
}
