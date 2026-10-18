-- Tariqi — initial schema.
--
-- FORWARD-ONLY. Once this has shipped it is never edited; corrections go in a
-- new migration. See CLAUDE.md.
--
-- Covers the first release (docs/09-FIRST-RELEASE.md): the school, its
-- categories, its staff, and its candidates. Enrolment, hours, payments and
-- expenses arrive in 0002 when those screens are built — there is no value in
-- shipping tables nothing exercises.
--
-- Conventions:
--   * timestamps are UTC ISO-8601 TEXT
--   * money is INTEGER centimes of DZD, never REAL
--   * anything with legal or financial weight is soft-deleted via deleted_at

-- ---------------------------------------------------------------------------
-- The school. Exactly one row, enforced by the CHECK on the primary key.
-- ---------------------------------------------------------------------------
CREATE TABLE school (
    id                  INTEGER PRIMARY KEY CHECK (id = 1),
    name_fr             TEXT    NOT NULL DEFAULT '',
    name_ar             TEXT    NOT NULL DEFAULT '',
    address_fr          TEXT    NOT NULL DEFAULT '',
    address_ar          TEXT    NOT NULL DEFAULT '',
    commune             TEXT    NOT NULL DEFAULT '',
    wilaya_code         TEXT    NOT NULL DEFAULT '',
    phone               TEXT    NOT NULL DEFAULT '',
    email               TEXT    NOT NULL DEFAULT '',
    agrement_number     TEXT    NOT NULL DEFAULT '',
    default_language    TEXT    NOT NULL DEFAULT 'fr'
                                CHECK (default_language IN ('fr', 'ar')),
    receipt_prefix      TEXT    NOT NULL DEFAULT '',
    -- Receipt numbers must be gapless and never reused: a gap looks like theft
    -- to an owner. Allocation happens inside a transaction.
    next_receipt_number INTEGER NOT NULL DEFAULT 1,
    created_at          TEXT    NOT NULL,
    updated_at          TEXT    NOT NULL
);

-- ---------------------------------------------------------------------------
-- Licence categories.
--
-- This is the school's PRICE LIST, not a regulatory table. The owner types in
-- what he teaches, what he charges, and how many hours he gives. Nothing here
-- is checked against any law and no action is ever refused because of it.
-- ---------------------------------------------------------------------------
CREATE TABLE licence_categories (
    code                   TEXT    PRIMARY KEY,
    code_ar                TEXT    NOT NULL DEFAULT '',
    name_fr                TEXT    NOT NULL,
    name_ar                TEXT    NOT NULL,
    theory_hours_target    INTEGER NOT NULL DEFAULT 0,
    practical_hours_target INTEGER NOT NULL DEFAULT 0,
    default_price_centimes INTEGER NOT NULL DEFAULT 0,
    is_offered             INTEGER NOT NULL DEFAULT 1,
    sort_order             INTEGER NOT NULL DEFAULT 0
);

-- ---------------------------------------------------------------------------
-- Staff.
--
-- One row per human, ever. In most Algerian auto-écoles the owner is also an
-- instructor and often the secretary too, so roles are a SET in a separate
-- table, never a column here. See docs/02-DOMAIN-MODEL.md §2.
-- ---------------------------------------------------------------------------
CREATE TABLE staff (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    first_name_fr TEXT NOT NULL DEFAULT '',
    last_name_fr  TEXT NOT NULL DEFAULT '',
    first_name_ar TEXT NOT NULL DEFAULT '',
    last_name_ar  TEXT NOT NULL DEFAULT '',
    phone         TEXT NOT NULL DEFAULT '',
    is_active     INTEGER NOT NULL DEFAULT 1,
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL,
    deleted_at    TEXT
);

CREATE TABLE staff_roles (
    staff_id INTEGER NOT NULL REFERENCES staff(id) ON DELETE CASCADE,
    role     TEXT    NOT NULL
             CHECK (role IN ('owner', 'manager', 'secretary', 'instructor')),
    PRIMARY KEY (staff_id, role)
);

-- ---------------------------------------------------------------------------
-- Candidates.
--
-- Names are stored in BOTH scripts and neither is derived from the other:
-- Algerian paperwork needs the Arabic form, while the Latin form is used for
-- internal search and Latin-script printouts.
-- ---------------------------------------------------------------------------
CREATE TABLE candidates (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    nin            TEXT,
    first_name_fr  TEXT NOT NULL,
    last_name_fr   TEXT NOT NULL,
    first_name_ar  TEXT NOT NULL DEFAULT '',
    last_name_ar   TEXT NOT NULL DEFAULT '',
    father_name_fr TEXT NOT NULL DEFAULT '',
    father_name_ar TEXT NOT NULL DEFAULT '',
    mother_name_fr TEXT NOT NULL DEFAULT '',
    mother_name_ar TEXT NOT NULL DEFAULT '',
    birth_date     TEXT,
    birth_place_fr TEXT NOT NULL DEFAULT '',
    birth_place_ar TEXT NOT NULL DEFAULT '',
    gender         TEXT CHECK (gender IS NULL OR gender IN ('m', 'f')),
    blood_group    TEXT NOT NULL DEFAULT '',
    phone          TEXT NOT NULL DEFAULT '',
    phone_alt      TEXT NOT NULL DEFAULT '',
    address_fr     TEXT NOT NULL DEFAULT '',
    address_ar     TEXT NOT NULL DEFAULT '',
    commune        TEXT NOT NULL DEFAULT '',
    wilaya_code    TEXT NOT NULL DEFAULT '',
    photo_path     TEXT,
    status         TEXT NOT NULL DEFAULT 'active'
                   CHECK (status IN ('prospect', 'active', 'inactive', 'completed', 'withdrawn')),
    notes          TEXT NOT NULL DEFAULT '',

    -- Normalised haystack for search: Latin lowercased, Arabic with alef forms
    -- and tashkeel folded, plus phone and NIN digits. Built in Rust on write so
    -- that searching "بن علي" finds "بْن عَلِي". Without folding, Arabic search
    -- silently misses rows that look identical to the user.
    search_text    TEXT NOT NULL DEFAULT '',

    registered_at  TEXT NOT NULL,
    created_at     TEXT NOT NULL,
    updated_at     TEXT NOT NULL,
    deleted_at     TEXT
);

CREATE INDEX idx_candidates_search  ON candidates(search_text);
CREATE INDEX idx_candidates_status  ON candidates(status) WHERE deleted_at IS NULL;
CREATE INDEX idx_candidates_created ON candidates(created_at);

-- A NIN identifies one person, so it must be unique — but only among live rows,
-- and only when present. A partial unique index does exactly that: soft-deleted
-- candidates do not block re-registering the same person, and a candidate who
-- has not brought their papers yet can still be registered with no NIN at all.
CREATE UNIQUE INDEX idx_candidates_nin_unique
    ON candidates(nin)
    WHERE nin IS NOT NULL AND nin <> '' AND deleted_at IS NULL;

-- ---------------------------------------------------------------------------
-- Audit log. Append-only; never edited or deleted from the UI.
-- ---------------------------------------------------------------------------
CREATE TABLE audit_log (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    at          TEXT    NOT NULL,
    staff_id    INTEGER REFERENCES staff(id),
    entity_type TEXT    NOT NULL,
    entity_id   TEXT    NOT NULL,
    action      TEXT    NOT NULL,
    before_json TEXT,
    after_json  TEXT
);

CREATE INDEX idx_audit_entity ON audit_log(entity_type, entity_id);
CREATE INDEX idx_audit_at     ON audit_log(at);

-- ---------------------------------------------------------------------------
-- Seed
-- ---------------------------------------------------------------------------
INSERT INTO school (id, created_at, updated_at)
VALUES (1, datetime('now'), datetime('now'));

-- Default categories, all editable in settings. Hour targets start at 0 so the
-- owner sets what he actually gives rather than inheriting a number we invented.
INSERT INTO licence_categories
    (code, code_ar, name_fr, name_ar, sort_order) VALUES
    ('A1', 'أ1', 'Cyclomoteur',        'دراجة نارية صغيرة', 1),
    ('A',  'أ',  'Motocyclette',       'دراجة نارية',       2),
    ('B',  'ب',  'Véhicule léger',     'سيارة خفيفة',       3),
    ('C1', 'ج1', 'Camion léger',       'شاحنة خفيفة',       4),
    ('C',  'ج',  'Poids lourd',        'وزن ثقيل',          5),
    ('D',  'د',  'Transport en commun','النقل الجماعي',     6),
    ('F',  'و',  'Véhicule aménagé',   'مركبة مهيأة',       7);
