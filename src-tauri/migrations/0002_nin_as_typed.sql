-- Keep the NIN exactly as the school typed it, and match on digits separately.
--
-- 0001 stored only the digits, so "109 2004 1600 5482 130" came back as
-- "109200416005482130". For a product whose whole point is producing correct
-- paperwork, silently reformatting an official identity number is wrong: the
-- secretary compares what is on screen against the document in their hand.
--
-- So `nin` now holds the text as entered and `nin_digits` holds the normalised
-- form used for uniqueness and search.

ALTER TABLE candidates ADD COLUMN nin_digits TEXT;

-- Backfill: at this point `nin` already holds digits only, so they are equal.
UPDATE candidates SET nin_digits = nin WHERE nin IS NOT NULL AND nin <> '';

-- Uniqueness moves to the normalised column: the same person must not be
-- registrable twice just by spacing their number differently.
DROP INDEX IF EXISTS idx_candidates_nin_unique;

CREATE UNIQUE INDEX idx_candidates_nin_digits_unique
    ON candidates(nin_digits)
    WHERE nin_digits IS NOT NULL AND nin_digits <> '' AND deleted_at IS NULL;

CREATE INDEX idx_candidates_nin_digits ON candidates(nin_digits);
