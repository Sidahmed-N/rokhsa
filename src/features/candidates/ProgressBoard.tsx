import { useCallback, useEffect, useState, type DragEvent } from "react";
import { Link } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { Phone } from "lucide-react";

import FreeText from "@/components/FreeText";
import Modal from "@/components/Modal";
import { Num } from "@/components/Num";
import DateField from "@/components/ui/DateField";
import { Badge, Card, ErrorNote } from "@/components/ui/Surface";
import { isAppError } from "@/lib/candidates";
import {
  getExamBoard,
  moveOnBoard,
  planBoardMove,
  type BoardCard,
  type BoardColumn,
  type BoardMove,
} from "@/lib/exams";
import { kindStyle } from "@/lib/kinds";
import { personName } from "@/lib/names";
import { isDesktop } from "@/lib/tauri";
import { todayIso } from "@/lib/training";

/**
 * Where everyone is, in one screen — and the quickest way to move them on.
 *
 * The list answers "find me Benaïssa"; this answers "who do I put in the
 * next code batch, who has been stuck on the créneau for two months, and
 * who has finished and is no longer my problem".
 *
 * A card is an ENROLMENT, not a candidate. Someone training for B and C1 is
 * at a different stage in each and appears twice.
 *
 * DRAGGING RECORDS AN EXAM EVENT. The column is derived from exam results,
 * so a card cannot simply be "put" somewhere — it would jump back on the next
 * load. Dropping it on a column records the result that puts it there
 * (passed, presented, failed, or a pass undone), after a confirmation. What
 * each drop means is decided in Rust (`domain::exams::plan_move`), not here,
 * and the result is the same row the Examens screen and the candidate's file
 * read.
 */

/** The columns, in the order of the journey. */
const COLUMNS = ["theory", "maneuvers", "road", "awaiting", "finished"] as const;
type Column = (typeof COLUMNS)[number];

/** Quiet after a month, loud after two — a candidate who has stalled. */
function stalled(days: number | null): boolean {
  return days !== null && days >= 60;
}

/** A drop waiting for the owner to confirm it. */
type Pending = { card: BoardCard; to: BoardColumn; move: BoardMove };

export default function ProgressBoard() {
  const { t } = useTranslation();

  const [cards, setCards] = useState<BoardCard[]>([]);
  const [error, setError] = useState<string | null>(null);

  const [dragging, setDragging] = useState<BoardCard | null>(null);
  const [over, setOver] = useState<Column | null>(null);
  // A drop that means nothing ("one step at a time"). Shown until the next
  // drag, so it is read rather than flashed. Kept as the raw error and
  // translated when drawn: a translated string went on showing French
  // after the language was switched to Arabic.
  const [notice, setNotice] = useState<unknown>(null);
  const [pending, setPending] = useState<Pending | null>(null);

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

  async function drop(column: Column) {
    const card = dragging;
    setDragging(null);
    setOver(null);
    if (!card || card.column === column) return;
    try {
      const move = await planBoardMove(card.enrollmentId, column);
      setPending({ card, to: column, move });
    } catch (raw: unknown) {
      setNotice(raw);
    }
  }

  const withdrawn = cards.filter((c) => c.column === "withdrawn");

  return (
    <div>
      {error && <ErrorNote>{error}</ErrorNote>}

      {notice !== null ? (
        <p className="mb-3 rounded-md border border-warn/30 bg-warn-soft px-3 py-2 text-sm text-warn">
          {errorText(notice)}
        </p>
      ) : (
        <p className="mb-3 text-xs text-muted">{t("board.dragHint")}</p>
      )}

      <div className="grid gap-3 lg:grid-cols-5">
        {COLUMNS.map((column) => {
          const mine = cards.filter((c) => c.column === column);
          const isStage = column !== "awaiting" && column !== "finished";
          const isTarget = over === column && dragging !== null && dragging.column !== column;

          return (
            <section
              key={column}
              // The whole column is the drop zone, not just its cards: an
              // empty column is exactly where a card most often goes.
              onDragOver={(event: DragEvent) => {
                if (!dragging) return;
                event.preventDefault();
                event.dataTransfer.dropEffect = "move";
                if (over !== column) setOver(column);
              }}
              onDragLeave={(event: DragEvent) => {
                // Leaving for one of its own children is not leaving.
                if (!event.currentTarget.contains(event.relatedTarget as Node | null)) {
                  setOver((current) => (current === column ? null : current));
                }
              }}
              onDrop={(event: DragEvent) => {
                event.preventDefault();
                void drop(column);
              }}
              className={[
                // Outline, not border or padding: highlighting a column must
                // not shift the layout under the pointer.
                "min-w-0 rounded-lg transition-colors",
                isTarget ? "bg-brand-soft/40 outline-2 outline-offset-4 outline-dashed outline-brand" : "",
              ].join(" ")}
            >
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
                  <div
                    key={card.enrollmentId}
                    draggable
                    onDragStart={(event: DragEvent) => {
                      event.dataTransfer.effectAllowed = "move";
                      // Some engines refuse to start a drag that carries no data.
                      event.dataTransfer.setData("text/plain", String(card.enrollmentId));
                      setNotice(null);
                      setDragging(card);
                    }}
                    onDragEnd={() => {
                      setDragging(null);
                      setOver(null);
                    }}
                    className={[
                      "cursor-grab active:cursor-grabbing",
                      dragging?.enrollmentId === card.enrollmentId ? "opacity-40" : "",
                    ].join(" ")}
                  >
                    <Card className="p-2.5">
                      {/* The name is in whatever script the school typed. The
                          link keeps the page's direction so the name starts on
                          the page's side; the inner span takes the name's own
                          direction so a long one is clipped at its END.
                          draggable={false}: a link drags its URL by default,
                          and the card is what should move. */}
                      <Link
                        to={`/candidates/${card.candidateId}`}
                        draggable={false}
                        className="block text-sm font-medium text-ink hover:text-brand hover:underline"
                      >
                        <span dir="auto" className="inline-block max-w-full truncate align-top">
                          {personName(card)}
                        </span>
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
                  </div>
                ))}
              </div>
            </section>
          );
        })}
      </div>

      {/* Abandons are kept visible but out of the way: the owner still wants
          to see them, and they are not part of the pipeline. Not draggable —
          withdrawing is a decision made on the candidate's file. */}
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

      {pending && (
        <ConfirmMove
          pending={pending}
          onClose={() => setPending(null)}
          onDone={() => {
            setPending(null);
            void load();
          }}
        />
      )}
    </div>
  );
}

/**
 * "Benali a réussi le code — le 12 octobre?"
 *
 * Every drop is confirmed, because every drop writes an exam result and a
 * card let go one column too early should not quietly record a pass. A date
 * is asked only where the drop creates a new result; a result coming back
 * for an exam he was presented at keeps that exam's date.
 */
function ConfirmMove({
  pending,
  onClose,
  onDone,
}: {
  pending: Pending;
  onClose: () => void;
  onDone: () => void;
}) {
  const { t } = useTranslation();
  const { card, to, move } = pending;

  const asksDate = move.action === "present" || move.action === "pass";
  const [date, setDate] = useState(todayIso());
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function save() {
    setSaving(true);
    setError(null);
    try {
      await moveOnBoard(card.enrollmentId, to, asksDate ? date : "");
      onDone();
    } catch (raw: unknown) {
      setError(isAppError(raw) ? t(raw.code, { defaultValue: raw.message }) : String(raw));
      setSaving(false);
    }
  }

  const exam = t(`lessons.kind_${move.kind}`);

  return (
    <Modal
      title={t(`board.confirm.${move.action}`, { exam })}
      onClose={onClose}
      onSubmit={() => void save()}
      submitLabel={t("board.confirm.submit")}
      submitting={saving}
      error={error}
    >
      {/* The name on its own line, not interpolated into the sentence: it
          may be in the other script, and inside a sentence the bidi
          algorithm would reorder it against the words around it. The line
          keeps the page's direction (so it starts on the page's side) and
          the name its own. */}
      <p className="text-base font-semibold text-ink">
        <FreeText as="span">{personName(card)}</FreeText>
      </p>

      {move.action === "resultFailed" && (
        <p className="mt-2 text-sm text-muted">{t("board.confirm.failedHint")}</p>
      )}
      {move.action === "undoPass" && (
        <p className="mt-2 text-sm text-muted">{t("board.confirm.undoHint")}</p>
      )}

      {asksDate && (
        <div className="mt-4">
          <span className="mb-1 block text-xs text-muted">{t("board.confirm.date")}</span>
          <DateField value={date} onChange={setDate} aria-label={t("board.confirm.date")} />
        </div>
      )}
    </Modal>
  );
}
