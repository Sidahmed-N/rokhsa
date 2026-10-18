-- Enrolment, hours and payments — the money loop.
--
-- FORWARD-ONLY. See CLAUDE.md.
--
-- Two rules shape every table here:
--   * money is INTEGER centimes of DZD, never REAL
--   * payments are APPEND-ONLY: a mistake is reversed with a linked negative
--     row, never edited or deleted, and receipt numbers are never reused

-- ---------------------------------------------------------------------------
-- Payment methods. Seeded, editable, and matching how Algerian schools are
-- actually paid — cash dominates.
-- ---------------------------------------------------------------------------
CREATE TABLE payment_methods (
    code               TEXT PRIMARY KEY,
    name_fr            TEXT    NOT NULL,
    name_ar            TEXT    NOT NULL,
    is_cash            INTEGER NOT NULL DEFAULT 0,
    requires_reference INTEGER NOT NULL DEFAULT 0,
    is_active          INTEGER NOT NULL DEFAULT 1,
    sort_order         INTEGER NOT NULL DEFAULT 0
);

INSERT INTO payment_methods (code, name_fr, name_ar, is_cash, requires_reference, sort_order) VALUES
    ('cash',      'Espèces',          'نقدا',        1, 0, 1),
    ('cheque',    'Chèque',           'صك',          0, 1, 2),
    ('transfer',  'Virement bancaire','تحويل بنكي',  0, 1, 3),
    ('baridimob', 'BaridiMob',        'بريدي موب',   0, 1, 4),
    ('edahabia',  'Edahabia',         'الذهبية',     0, 1, 5),
    ('cib',       'CIB',              'CIB',         0, 1, 6);

-- ---------------------------------------------------------------------------
-- Enrolment: one candidate, one category, one agreed price.
-- ---------------------------------------------------------------------------
CREATE TABLE enrollments (
    id                     INTEGER PRIMARY KEY AUTOINCREMENT,
    candidate_id           INTEGER NOT NULL REFERENCES candidates(id),
    category_code          TEXT    NOT NULL REFERENCES licence_categories(code),
    enrolled_on            TEXT    NOT NULL,

    -- SNAPSHOTS. Copied at enrolment and never read live from the price list.
    -- Changing what the school charges tomorrow must not retroactively change
    -- what an existing candidate owes, or what their progress bar says. This
    -- is the most common and most damaging mistake in this kind of software.
    agreed_price_centimes  INTEGER NOT NULL DEFAULT 0,
    extra_hour_price_centimes INTEGER NOT NULL DEFAULT 0,
    theory_hours_target    INTEGER NOT NULL DEFAULT 0,
    practical_hours_target INTEGER NOT NULL DEFAULT 0,

    discount_centimes      INTEGER NOT NULL DEFAULT 0,
    discount_reason        TEXT    NOT NULL DEFAULT '',

    status                 TEXT    NOT NULL DEFAULT 'forming'
                           CHECK (status IN ('forming', 'ready', 'presented', 'passed', 'failed', 'withdrawn')),
    notes                  TEXT    NOT NULL DEFAULT '',
    created_at             TEXT    NOT NULL,
    updated_at             TEXT    NOT NULL,
    deleted_at             TEXT
);

CREATE INDEX idx_enrollments_candidate ON enrollments(candidate_id) WHERE deleted_at IS NULL;

-- A candidate should not be enrolled twice in the same category at once, but
-- may re-enrol after withdrawing, so this is scoped to live rows.
CREATE UNIQUE INDEX idx_enrollments_unique_live
    ON enrollments(candidate_id, category_code)
    WHERE deleted_at IS NULL;

-- ---------------------------------------------------------------------------
-- Hours log.
--
-- Deliberately NOT a calendar: no booking, no conflict checking, no scheduled
-- window. It records that a lesson happened. Scheduling is a later release.
--
-- Duration is stored in whole MINUTES for the same reason money is stored in
-- centimes: a float duration drifts, and hours-remaining is a number the
-- candidate argues about.
-- ---------------------------------------------------------------------------
CREATE TABLE lessons (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    enrollment_id       INTEGER NOT NULL REFERENCES enrollments(id),
    instructor_staff_id INTEGER REFERENCES staff(id),
    kind                TEXT    NOT NULL
                        CHECK (kind IN ('theory', 'maneuvers', 'road')),
    occurred_on         TEXT    NOT NULL,
    minutes             INTEGER NOT NULL CHECK (minutes > 0),
    notes               TEXT    NOT NULL DEFAULT '',
    created_at          TEXT    NOT NULL,
    updated_at          TEXT    NOT NULL,
    deleted_at          TEXT
);

CREATE INDEX idx_lessons_enrollment ON lessons(enrollment_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_lessons_date       ON lessons(occurred_on);

-- ---------------------------------------------------------------------------
-- Payments. APPEND-ONLY.
--
-- No updated_at and no deleted_at, on purpose: there is no code path that
-- edits or removes a payment. A wrong entry is corrected by inserting a
-- negative row pointing at it via reverses_payment_id, which leaves the
-- original receipt and its number intact.
-- ---------------------------------------------------------------------------
CREATE TABLE payments (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    enrollment_id       INTEGER NOT NULL REFERENCES enrollments(id),
    amount_centimes     INTEGER NOT NULL,
    paid_on             TEXT    NOT NULL,
    method_code         TEXT    NOT NULL REFERENCES payment_methods(code),
    reference           TEXT    NOT NULL DEFAULT '',

    -- Gapless and never reused: a gap looks like theft to an owner and like
    -- fraud to an inspector. Allocated inside a transaction.
    receipt_number      TEXT    NOT NULL UNIQUE,

    reverses_payment_id INTEGER REFERENCES payments(id),
    received_by_staff_id INTEGER REFERENCES staff(id),
    notes               TEXT    NOT NULL DEFAULT '',
    created_at          TEXT    NOT NULL
);

CREATE INDEX idx_payments_enrollment ON payments(enrollment_id);
CREATE INDEX idx_payments_date       ON payments(paid_on);

-- A payment may only be reversed once.
CREATE UNIQUE INDEX idx_payments_reverses_unique
    ON payments(reverses_payment_id)
    WHERE reverses_payment_id IS NOT NULL;
