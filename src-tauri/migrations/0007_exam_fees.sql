-- Exam fees: charged once per stage, never again for a retake.
--
-- FORWARD-ONLY. See CLAUDE.md.
--
-- How the money actually works (docs/05-OPEN-QUESTIONS.md, answered by the
-- owner): a candidate pays ONCE at registration for the training, and then
-- again before each exam — a separate fee the school collects and hands to
-- the examiner. If he fails, he sits it again and pays NOTHING the second
-- time. He has already paid for that exam.
--
-- So the fee attaches to the STAGE, not to the sitting. That is the whole
-- design: `idx_exam_fee_charges_once` makes a second charge for the same
-- stage impossible at the database level, so the free repassage is a
-- constraint the schema enforces rather than a rule someone has to remember
-- in every code path that enters a candidate for an exam.
--
-- Note this replaces nothing. The hour-based `extra_hour_price_centimes` on
-- enrolments stays for the schools that do charge by the hour; it is simply
-- not how the exam money moves.

-- ---------------------------------------------------------------------------
-- The school's own fee list. Editable in Réglages, like every other price.
--
-- Seeded at ZERO deliberately. The owner quoted 3 000 DZD for the code, but
-- that is one school in one wilaya in one year, and the amount is set by the
-- administration, not by us. Shipping a number we invented would be read as
-- authoritative. Zero means "not configured", shows nothing on a candidate's
-- file, and costs nobody money until the owner types what he actually charges.
-- ---------------------------------------------------------------------------
CREATE TABLE exam_fees (
    kind            TEXT    PRIMARY KEY
                    CHECK (kind IN ('theory', 'maneuvers', 'road')),
    amount_centimes INTEGER NOT NULL DEFAULT 0,
    updated_at      TEXT    NOT NULL DEFAULT ''
);

INSERT INTO exam_fees (kind, amount_centimes) VALUES
    ('theory',    0),
    ('maneuvers', 0),
    ('road',      0);

-- ---------------------------------------------------------------------------
-- What a given candidate was actually charged.
--
-- `amount_centimes` is a SNAPSHOT, for the same reason the agreed price on an
-- enrolment is: the administration raising the code fee next year must not
-- silently change what a candidate who sat it last year owes.
-- ---------------------------------------------------------------------------
CREATE TABLE exam_fee_charges (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    enrollment_id   INTEGER NOT NULL REFERENCES enrollments(id),

    kind            TEXT    NOT NULL
                    CHECK (kind IN ('theory', 'maneuvers', 'road')),

    amount_centimes INTEGER NOT NULL,

    -- The day the school entered him for that exam — which is when the money
    -- is actually asked for, not the day of the sitting.
    charged_on      TEXT    NOT NULL,

    created_at      TEXT    NOT NULL,
    deleted_at      TEXT
);

CREATE INDEX idx_exam_fee_charges_enrollment
    ON exam_fee_charges(enrollment_id)
    WHERE deleted_at IS NULL;

-- The free repassage, as a constraint. One charge per stage per enrolment,
-- however many times he sits it.
CREATE UNIQUE INDEX idx_exam_fee_charges_once
    ON exam_fee_charges(enrollment_id, kind)
    WHERE deleted_at IS NULL;
