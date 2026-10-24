-- The exam fee is typed when the candidate is entered, not fixed in settings.
--
-- FORWARD-ONLY. See CLAUDE.md.
--
-- 0007 got this wrong, and the owner caught it. It read "the fee is paid once
-- per stage, a repassage is free" as a law and enforced it with a unique index
-- on (enrollment_id, kind), so a second charge for the same exam was
-- impossible. But that is one school's practice, not a rule:
--
--   "Maybe one-time thing for some people, and for other schools, it's not a
--    one-time thing. So we need to make it flexible."
--
-- That is CLAUDE.md's first rule about regulatory values — the app records
-- what the school does, it never refuses an action because a rule says so —
-- and 0007 broke it. A school that charges again for the repassage must be
-- able to say so.
--
-- So: the index goes, the settings table goes, and the amount moves to where
-- the owner is actually standing — the moment he enters a candidate for a
-- sitting. A charge now belongs to THAT ENTRY, which also means removing
-- someone from a sitting removes their fee with it, instead of the
-- "is he still entered for this stage anywhere" bookkeeping 0007 needed.

-- The entry the fee was charged for. Nullable only so the backfill below can
-- run; every new charge sets it.
ALTER TABLE exam_fee_charges ADD COLUMN result_id INTEGER REFERENCES exam_results(id);

-- Existing charges predate the link. Attach each to the candidate's earliest
-- live sitting of that exam, which is the one it was raised for.
UPDATE exam_fee_charges
SET result_id = (
    SELECT r.id FROM exam_results r
    WHERE r.enrollment_id = exam_fee_charges.enrollment_id
      AND r.kind         = exam_fee_charges.kind
      AND r.deleted_at IS NULL
    ORDER BY r.sat_on, r.id
    LIMIT 1
)
WHERE result_id IS NULL;

-- A charge whose sitting is gone has nothing left to explain it.
UPDATE exam_fee_charges
SET deleted_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
WHERE result_id IS NULL AND deleted_at IS NULL;

-- The enforcement that should never have been here.
DROP INDEX IF EXISTS idx_exam_fee_charges_once;

CREATE INDEX idx_exam_fee_charges_result
    ON exam_fee_charges(result_id)
    WHERE deleted_at IS NULL;

-- No school-wide fixed amount any more.
DROP TABLE IF EXISTS exam_fees;
