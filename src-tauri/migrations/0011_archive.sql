-- Archived candidates.
--
-- FORWARD-ONLY. See CLAUDE.md.
--
-- A candidate who has passed everything stays in the school's books forever:
-- their payments are part of past months' takings, and deleting them would
-- make those months show less money than was actually collected. But the
-- owner does not want to scroll past them every morning either.
--
-- Archiving is that: the candidate leaves the list, the board and the session
-- picker, and nothing else changes. Their payments still count, money they
-- still owe still shows in Créances, and one click brings them back.
-- NULL means not archived; otherwise when it was done.
--
-- Not `deleted_at`, and not a new value of `status`: deleted means gone from
-- the books, and `status` is the school's own label, which archiving must not
-- overwrite (an archived candidate is still "Terminé" or "Abandon").

ALTER TABLE candidates ADD COLUMN archived_at TEXT;
