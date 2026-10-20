-- Sessions can now be PLANNED, not only recorded after the fact.
--
-- FORWARD-ONLY. See CLAUDE.md.
--
-- Until now `lessons` was a log of the past: the secretary opened a candidate
-- after the lesson and typed what had happened. That left the school running
-- its day off the wall planner, and an app that does not help run the day gets
-- opened once a week — at which point the hours go stale and the receivables
-- total, which depends on them, is wrong too.
--
-- CLAUDE.md already called for this: "A lesson has both a scheduled window and
-- an actual window. They are different columns and both matter."
--
-- Design note: `occurred_on` and `minutes` stay NOT NULL. A planned session
-- seeds them from the plan and they are corrected on completion, which avoids
-- rebuilding the table and keeps "what we intended" and "what happened" as
-- separate facts. Everything that counts hours or money MUST filter on
-- status = 'done', or a session merely planned would count as hours taught.

ALTER TABLE lessons ADD COLUMN scheduled_on TEXT;

-- "HH:MM", or NULL. Optional on purpose: some schools book a time, others just
-- say "Thursday" and the candidate turns up when they can.
ALTER TABLE lessons ADD COLUMN scheduled_time TEXT;

-- Existing rows were all logged after the event, so 'done' is the correct
-- backfill and the right default for a straight log entry.
ALTER TABLE lessons ADD COLUMN status TEXT NOT NULL DEFAULT 'done'
    CHECK (status IN ('planned', 'done', 'missed', 'cancelled'));

-- What the candidate should work on next: carried from the previous session's
-- note when planning, so the instructor sees it without digging.
ALTER TABLE lessons ADD COLUMN plan_note TEXT NOT NULL DEFAULT '';

CREATE INDEX idx_lessons_scheduled ON lessons(scheduled_on)
    WHERE deleted_at IS NULL AND status = 'planned';

CREATE INDEX idx_lessons_status ON lessons(status) WHERE deleted_at IS NULL;
