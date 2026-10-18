-- Expenses — the "out" half of money in and out.
--
-- FORWARD-ONLY. See CLAUDE.md.
--
-- Unlike payments, an expense CAN be edited and soft-deleted: it is the
-- school's own record of its own spending, with no receipt handed to a third
-- party and no number sequence to keep gapless. The asymmetry is deliberate.

CREATE TABLE expense_categories (
    code       TEXT PRIMARY KEY,
    name_fr    TEXT    NOT NULL,
    name_ar    TEXT    NOT NULL,
    is_active  INTEGER NOT NULL DEFAULT 1,
    sort_order INTEGER NOT NULL DEFAULT 0
);

INSERT INTO expense_categories (code, name_fr, name_ar, sort_order) VALUES
    ('fuel',        'Carburant',            'الوقود',            1),
    ('maintenance', 'Entretien véhicule',   'صيانة المركبة',     2),
    ('insurance',   'Assurance',            'التأمين',           3),
    ('vignette',    'Vignette',             'الرسم على المركبة', 4),
    ('inspection',  'Contrôle technique',   'المراقبة التقنية',  5),
    ('rent',        'Loyer',                'الإيجار',           6),
    ('salaries',    'Salaires',             'الأجور',            7),
    ('taxes',       'Taxes et impôts',      'الضرائب والرسوم',   8),
    ('supplies',    'Fournitures',          'اللوازم',           9),
    ('other',       'Divers',               'متفرقات',          10);

CREATE TABLE expenses (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    category_code   TEXT    NOT NULL REFERENCES expense_categories(code),
    amount_centimes INTEGER NOT NULL CHECK (amount_centimes > 0),
    spent_on        TEXT    NOT NULL,
    description     TEXT    NOT NULL DEFAULT '',
    supplier        TEXT    NOT NULL DEFAULT '',
    -- Free text until the fleet module exists; it becomes a foreign key then.
    vehicle_label   TEXT    NOT NULL DEFAULT '',
    created_at      TEXT    NOT NULL,
    updated_at      TEXT    NOT NULL,
    deleted_at      TEXT
);

CREATE INDEX idx_expenses_date     ON expenses(spent_on)      WHERE deleted_at IS NULL;
CREATE INDEX idx_expenses_category ON expenses(category_code) WHERE deleted_at IS NULL;
