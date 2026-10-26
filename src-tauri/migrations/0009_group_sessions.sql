-- Several candidates in one session.
--
-- FORWARD-ONLY. See CLAUDE.md.
--
-- How the school actually teaches (docs/05-OPEN-QUESTIONS.md Q17): the
-- instructor takes three or four candidates for the créneau, gives them a day
-- and a time, and they come and take their turn one by one, week after week
-- until the exam. A candidate who fails is either folded in with another
-- group or taught alone — "it depends on what the instructor wants".
--
-- That last sentence decided the shape. A GROUP is not a thing the instructor
-- manages: there is nothing to create, name, or add members to, because he
-- moves people between groups and out of them as he feels like. There is only
-- "who did I tell to come on Tuesday". So this adds no group table.
--
-- It also does NOT move to one session row with an attendee list, which was
-- the obvious modelling answer. Every hour counted, every balance and the
-- whole candidate file hang off one lesson row per candidate, and rewriting
-- that to read through an attendee table would put the money logic — the
-- highest-risk code in the product — in the path of a UI change. Instead each
-- candidate keeps their own row, exactly as before, and rows booked together
-- share a session id.
--
-- What that buys:
--   * hours, balances, and the candidate's lesson log are untouched
--   * attendance is already per candidate — `lessons.status` is 'done' or
--     'missed' for THAT person, so one absentee in a group of three is
--     recorded without any new concept
--   * teaching one candidate alone is not a special case: it is a row with no
--     session id
--
-- What it costs, and how it is paid: the slot's facts (day, time, instructor,
-- duration) are repeated on each row, so they can drift apart. Every
-- operation on a booked session must therefore be keyed by `session_id` and
-- write all its rows at once — never one row of a group on its own.

-- Nothing but an identity to group by. The shared facts stay on the lesson
-- rows; giving this table columns would start the drift this design avoids.
CREATE TABLE lesson_sessions (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT    NOT NULL
);

-- NULL means the lesson is its own session, which is both the backfill for
-- every existing row and the normal state of a one-candidate booking. No data
-- migration is needed and no row has to be invented.
ALTER TABLE lessons ADD COLUMN session_id INTEGER REFERENCES lesson_sessions(id);

CREATE INDEX idx_lessons_session ON lessons(session_id)
    WHERE deleted_at IS NULL AND session_id IS NOT NULL;
