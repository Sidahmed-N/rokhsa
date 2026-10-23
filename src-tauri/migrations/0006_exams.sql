-- Exams: the batch sitting, and one result per candidate in it.
--
-- FORWARD-ONLY. See CLAUDE.md.
--
-- Until now the only record of an outcome was a single `enrollments.status`
-- column. One value for a whole enrolment cannot say "passed the code, failed
-- the créneau, has not sat the circulation yet", which is the normal state of
-- most candidates in a school.
--
-- How it actually works (docs/05-OPEN-QUESTIONS.md, answered by the owner):
-- the school is told "these three, tomorrow 08:00, this centre, for the code".
-- One date, one place, one exam type, several candidates. So the SITTING is a
-- real object, not a date field hanging off a candidate — the school has to
-- assemble the batch, print who is going, and afterwards record who passed.
--
-- The three exams are sequential: code, then weeks of créneau training, then
-- the créneau exam, then weeks of circulation training, then that exam. Each
-- is passed or failed on its own, and failing one means retaking THAT one
-- while the earlier passes stand.

CREATE TABLE exam_sessions (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,

    -- Same vocabulary as lessons.kind on purpose: these are the three stages
    -- of the journey, not a separate taxonomy.
    kind           TEXT    NOT NULL
                   CHECK (kind IN ('theory', 'maneuvers', 'road')),

    held_on        TEXT    NOT NULL,
    -- "HH:MM", or empty. The school is usually given a time, but not always,
    -- and an empty string is honest where a made-up 00:00 is not.
    held_at        TEXT    NOT NULL DEFAULT '',
    -- Where the wilaya told them to go. Free text: centre names are not
    -- something we can enumerate from here.
    place          TEXT    NOT NULL DEFAULT '',
    notes          TEXT    NOT NULL DEFAULT '',

    created_at     TEXT    NOT NULL,
    updated_at     TEXT    NOT NULL,
    deleted_at     TEXT
);

CREATE INDEX idx_exam_sessions_date ON exam_sessions(held_on)
    WHERE deleted_at IS NULL;

-- One row per candidate per attempt.
--
-- APPEND-IN-SPIRIT: a retake is a NEW row, never an edit of the failure. The
-- earlier attempt is part of the candidate's history and the owner cares how
-- many goes it took. Correcting a result that was typed wrong is an edit; a
-- repassage is not a correction.
CREATE TABLE exam_results (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    enrollment_id  INTEGER NOT NULL REFERENCES enrollments(id),

    -- Null when the school recorded an outcome without having created a
    -- sitting for it — a result that came in for someone presented long ago,
    -- or a school entering history when it first installs the app.
    session_id     INTEGER REFERENCES exam_sessions(id),

    -- Denormalised from the sitting so a result without one still knows which
    -- exam it was, and so the candidate's history reads without a join.
    kind           TEXT    NOT NULL
                   CHECK (kind IN ('theory', 'maneuvers', 'road')),

    sat_on         TEXT    NOT NULL,

    -- 'presented' means the batch exists and the outcome is not back yet.
    -- Without it the school could not print tomorrow's list, which is half
    -- the reason the sitting is a real object.
    outcome        TEXT    NOT NULL DEFAULT 'presented'
                   CHECK (outcome IN ('presented', 'passed', 'failed', 'absent')),

    notes          TEXT    NOT NULL DEFAULT '',

    created_at     TEXT    NOT NULL,
    updated_at     TEXT    NOT NULL,
    deleted_at     TEXT
);

CREATE INDEX idx_exam_results_enrollment ON exam_results(enrollment_id)
    WHERE deleted_at IS NULL;

CREATE INDEX idx_exam_results_session ON exam_results(session_id)
    WHERE deleted_at IS NULL;

-- A candidate appears once per sitting. Presenting the same person twice in
-- one batch is a data-entry slip, not a thing that happens.
CREATE UNIQUE INDEX idx_exam_results_unique_in_session
    ON exam_results(session_id, enrollment_id)
    WHERE deleted_at IS NULL AND session_id IS NOT NULL;
