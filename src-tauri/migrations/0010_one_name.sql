-- One name per candidate, in whatever script it was typed.
--
-- FORWARD-ONLY. See CLAUDE.md.
--
-- 0001 stored every name twice, Latin and Arabic, on the assumption that the
-- school's paperwork would need both. The owner decided against it: filling
-- ten boxes to register one person is the kind of friction that sends a
-- secretary back to the notebook, and the school writes a name once, the way
-- the candidate gives it. So the form now has one box per name, and a box
-- takes French or Arabic alike.
--
-- The `_fr` columns become the name. They were already the required ones
-- (last_name_fr is NOT NULL), so every existing candidate has a value there
-- unless the school typed only the Arabic — and that case is copied across
-- below, field by field, so nobody ends up nameless.
--
-- The `_ar` columns are KEPT and no longer read or written. Dropping them
-- would destroy Arabic names schools already typed; keeping them costs
-- nothing, and if the Direction des Transports turns out to need the Arabic
-- form on paper, the data typed before this change is still there.

ALTER TABLE candidates RENAME COLUMN last_name_fr   TO last_name;
ALTER TABLE candidates RENAME COLUMN first_name_fr  TO first_name;
ALTER TABLE candidates RENAME COLUMN father_name_fr TO father_name;
ALTER TABLE candidates RENAME COLUMN mother_name_fr TO mother_name;
ALTER TABLE candidates RENAME COLUMN birth_place_fr TO birth_place;

UPDATE candidates SET last_name   = last_name_ar   WHERE last_name   = '' AND last_name_ar   <> '';
UPDATE candidates SET first_name  = first_name_ar  WHERE first_name  = '' AND first_name_ar  <> '';
UPDATE candidates SET father_name = father_name_ar WHERE father_name = '' AND father_name_ar <> '';
UPDATE candidates SET mother_name = mother_name_ar WHERE mother_name = '' AND mother_name_ar <> '';
UPDATE candidates SET birth_place = birth_place_ar WHERE birth_place = '' AND birth_place_ar <> '';
