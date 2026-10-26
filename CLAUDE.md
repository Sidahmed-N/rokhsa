# Tariqi — Auto-École Management System (Algeria)

> Working codename: **Tariqi** (طريقي / "my road"). Not final — see `docs/05-OPEN-QUESTIONS.md`.

This file is the engineering contract for this repository. Read it before writing code.

---

## 1. What this product is

An **offline-first Windows desktop application** that runs the day-to-day operations of an
Algerian driving school (*auto-école*): candidate files, lesson scheduling, instructor and
vehicle management, cash collection, exam sessions, and the printed paperwork the
*Direction des Transports de la Wilaya* expects.

It is sold as a **licensed product to many independent schools**, not hosted as SaaS.
Each school runs its own copy against its own local SQLite database. There is no central
server in v1.

### Non-negotiable product constraints

| Constraint | Why it exists |
|---|---|
| **Works with zero internet, forever** | Connectivity and grid power are unreliable; a school must be able to take a payment and print a receipt during an outage. |
| **Bilingual French + Arabic at launch, both complete** | French is the administrative language of auto-écoles; Arabic is the official language and many staff prefer it. |
| **Full RTL layout for Arabic** | Not a mirrored hack — correct logical layout, correct Arabic text shaping in print output. |
| **Runs on a weak office PC** | Assume 4 GB RAM, HDD, Windows 10. Installer must be small enough to pass over a slow link or a USB stick. |
| **Cash is the primary payment method** | Partial payments (*versements*) and a daily cash-register reconciliation are core features, not afterthoughts. The app does **not** print a receipt — schools write the candidate's own card by hand and did not want it duplicated. |
| **Paper is the output** | The school's legal obligations are discharged on paper. Printing is a first-class subsystem, not a report module. Nothing is printed *yet*: the receipt was removed, and the Arabic print proof goes with it until the first real printout (the exam batch list) brings it back. |

---

## 2. Hard engineering rules

These are not style preferences. Breaking them causes data loss, legal problems, or a rewrite.

### Money
- **Store money as integer centimes of DZD.** Type alias `Centimes = i64` in Rust,
  `type Centimes = number` in TS. **Never** use `f32`/`f64`/JS floats for money.
- Every mutation that touches money writes an **audit log row** (who, when, before, after).
- Money rows are **append-only**. A wrong payment is corrected with a reversing entry,
  never by editing or deleting the original.
- Every payment carries a **gapless sequential number that no screen shows**. The printed
  receipt was removed at the owner's request, but the number stays: it is the only thing
  that makes a deleted payment visible, because a missing number is a hole an owner can
  see even when the row is gone. Never reuse one, and do not put it back on screen.

### Regulatory values
- **The app records what the school does; it does not enforce the law.** It is a bookkeeping
  and organisation tool. It never refuses an action because a regulation says so — at most it
  *warns*, and the owner decides. A school that wants to record 12 hours when the rule says 30
  must be able to.
- **Nothing regulatory is hardcoded.** Required theory hours, required practical hours,
  minimum age, licence categories, exam types, fee amounts, document checklists — all of it
  lives in seeded, editable database tables.
- **One exception, and it is a product decision, not a regulatory one:** the school teaches
  category **B only**, so `CATEGORY` is a constant in `TrainingDialogs.tsx` and no category
  appears anywhere in the interface. The `licence_categories` table and
  `enrollments.category_code` stay exactly as they were, still seeded with every category.
  A school that also teaches moto is therefore a screen to put back, not a migration to
  unpick — which is why the column was kept rather than dropped.
- Rationale: the official hour counts could not be verified against a published *arrêté*,
  and a new traffic law is in discussion. Assume every number changes within two years.
- **A common practice is not a rule, and must never become a constraint.** Migration 0007
  made "the exam fee is paid once, a repassage is free" a unique index, so a school that
  charges the retake again could not say so. That is one school's habit, not a law, and
  the owner caught it. Before enforcing anything in the schema, ask whether a school could
  reasonably do the opposite — if it could, the app records both. 0008 undid it.
- Seed data ships as a **migration + seed file** per wilaya/configuration, so a school can be
  corrected in the field without shipping a new binary.

### Bilingual / i18n
- **No user-visible string literals in components.** Every string goes through `t()`.
- Translation keys are namespaced by feature (`candidates.form.nin_label`).
- A key missing from `ar` fails CI. We do not ship a half-translated screen.
- **Use CSS logical properties only**: `ms-*`/`me-*`/`ps-*`/`pe-*`/`start-*`/`end-*`.
  `ml-*`, `pr-*`, `left-*`, `right-*` are banned in app code and enforced by lint.
- Person and place names are stored in **both scripts**: `last_name_fr` + `last_name_ar`, etc.
  Algerian administrative documents require the Arabic form; the Latin form is used for
  internal search and Latin-script printouts. Neither is derived from the other.
- **Every number, identifier and Latin string rendered in the UI goes through `<Num>` or
  `<Money>`** (`src/components/Num.tsx`). Never interpolate a formatted number straight into
  JSX, and never call `formatDZD()` inside a JSX expression — `npm run check:rtl` fails on it.
  Reason: in RTL text the bidi algorithm reorders space-separated runs, so a national ID
  stored as `109 2004 16 0054821` prints as `0054821 16 2004 109`. It looks perfect in French
  and silently wrong in Arabic. This was a real bug in the first receipt, caught only by
  measuring character positions in the browser — not by reading the code.
- **Text the school typed goes through `<FreeText>`** (`src/components/FreeText.tsx`), which
  sets `dir="auto"`. A plan note, a lesson note or an expense description may be French or
  Arabic and the app cannot know which. Beyond alignment, this decides where `truncate` puts
  the ellipsis: a French note in an Arabic page without it is laid out RTL and clipped at its
  *beginning*, so the week grid showed `...ation en ville, giratoires`. Correct in French,
  wrong only in Arabic — the signature of every bidi bug here.
- **A unit attached to a number goes INSIDE the isolate.** `<Num>{percent}</Num>%` renders as
  `%23` in Arabic, because the `%` left outside is a neutral character that resolves against
  the surrounding RTL text and lands on the far side of the digits. Write
  `<Num>{percent}%</Num>`. The same applies to any suffix a reader treats as part of the
  value.
- **Never put a number on both sides of an Arabic word.** `<Num>` isolates a value from
  its *surroundings*; it does not stop the bidi algorithm reordering runs *inside* it. A
  duration built as `1 سا 30` renders as `1 30 سا` — the unit is stranded at the end and
  "1 h 30" reads as "1 30 h". Leave a single numeric run instead (`1:30 سا`). This is why
  the Arabic duration format is deliberately *not* a translation of the French one, and why
  `<Duration>` exists. Note the same shape is fine in ordinary RTL prose
  (`{{planned}} … {{total}}` reads correctly), so the rule is about forced-LTR spans, not
  about Arabic sentences — measured in the WebView, both ways.
- **Never put a logical margin/padding on the same element that carries `dir`.**
  `ms-*` is `margin-inline-start`, which resolves against *that element's own* direction, so
  `<span dir="rtl" className="ms-2">` puts the gap on the far side and the text ends up
  touching its neighbour. Put the spacing on an outer element that inherits the surrounding
  direction, and the `dir` on an inner one. `check:rtl` cannot catch this — only looking at
  the screen does.
- **Identity numbers are stored as typed AND as digits.** `nin` holds what the school wrote
  (so it prints the way their paperwork reads); `nin_digits` is the normalised form used for
  uniqueness and search. Never display the normalised form.
- **A utility class must not set `display` unless that is its whole job.** `.screen-only`
  exists to hide something when printing; an early version also set `display: block`, which
  silently overrode `flex` on every element carrying both and collapsed those layouts.

### Dates and time
- Store all timestamps as **UTC ISO-8601 strings**. Algeria is UTC+01:00 with **no DST**,
  but do not rely on that in storage — only in display.
- A *lesson* has both a **scheduled** window and an **actual** window. They are different
  columns and both matter for billing and for instructor payroll.

### Group sessions
- **A session can hold several candidates**, because the *créneau* is taught in groups:
  three or four told to come Tuesday at eight, taking their turn one by one.
- **There is no group object, deliberately.** The instructor moves a failed candidate into
  another group or teaches him alone "depending on what the instructor wants", so a group
  is not a thing he manages — it is only "who did I tell to come on Tuesday".
- **One lesson row per candidate, linked by `session_id`** (migration 0009). Rows booked
  together share an id; `NULL` means the candidate is on their own. This was chosen over
  one session row with an attendee list so that hours, balances and the candidate's file —
  the money path — keep reading exactly what they always read. A UI change must not put
  the highest-risk code in its path.
- **The cost, and the rule that pays it:** the slot's facts (day, time, duration) are
  repeated on every row of a group and can drift. So every operation on a booked session
  is keyed by `session_id` and writes all its rows at once — `reschedule` already does.
  Never move, cancel or re-time one row of a group on its own.
- **Attendance is per candidate and already exists**: `lessons.status` is `done` or
  `missed` for *that person*, so one absentee in a group of three needs no new concept.
- `schedule_many` is the ONLY insert path for a booking; `schedule` is a wrapper over it.
  A second insert path is a second chance for a group's rows to disagree.

### Data integrity
- Migrations are **forward-only**. Never edit a migration that has shipped; add a new one.
- **Soft-delete** anything with legal or financial significance (candidates, enrolments,
  payments, lessons, exam results). Hard delete is reserved for genuinely transient rows.
- Every write path must be safe to interrupt: wrap multi-row operations in a transaction.
  The user *will* lose power mid-save.
- **Never open the database with ad-hoc connection options — always `db::connect`.**
  A second connection that does not set `journal_mode = WAL` will try to change the
  journal mode of a database another connection already holds in WAL, and that corrupts
  it: `database disk image is malformed`, with the main tables still readable so the
  damage is easy to miss. This happened to a diagnostic binary that opened the live
  database "just to count rows" while the app was running. `src-tauri/src/bin/inspect.rs`
  now goes through `db::connect` for exactly this reason, which also means it applies
  migrations — so copy a file before pointing it at one you want left untouched.
- **A read-only SQLite connection cannot read the `-wal` sidecar**, because it cannot
  create the `-shm` it needs. It silently reports the stale contents of the main file
  instead of failing. During the same incident this reported 30 candidates and migrations
  1–6 for a database that actually held neither, and sent the investigation the wrong way
  twice. Never diagnose a WAL database read-only.

### Identity and access
- **One person can hold several roles at once, and usually does.** In most Algerian
  auto-écoles the owner is also an instructor, and often the secretary too. Roles are a
  **set per person**, never a single column. `owner` + `instructor` is the common case,
  not an edge case.
- A person's permissions are the **union** of their roles' permissions. An owner who also
  teaches sees the whole business *and* appears on the schedule as a teaching resource.
- **Separate the person from their capabilities.** `staff` holds the human; `staff_roles`
  holds what they are permitted to do; `instructor_profile` holds what is needed to teach
  (authorisation number, categories, certification expiry, pay model). One person, one
  identity row — never duplicated across a "users" and an "instructors" table.
- Passwords hashed with **Argon2id**. No plaintext, no reversible storage, ever.
- Restrictions are stated as **negative capabilities of a role**, and only bite when the person
  holds no other role granting it: a `secretary` alone cannot see aggregate revenue; an
  `instructor` alone sees only their own schedule.
  Enforce authorisation in the **Rust command layer**, never only in the UI.

### The one-person school is the default case, not the small case
Design every screen so it works for a school with **one person, one vehicle**:
- When only one instructor exists, **do not show an instructor picker** — default it silently.
  Same for vehicles. A dropdown with one option is a bug.
- The scheduling calendar must collapse its instructor axis to a single column gracefully.
- Never require a hand-off between roles to complete a task. The same person registers the
  candidate, takes the cash, teaches the lesson, and closes the day.
- Login must not be daily friction for a solo owner on his own PC: offer "stay signed in on
  this device", and re-authenticate only for sensitive actions (price changes, data export,
  deletion, restore).
- Features justified by staff oversight (cash variance, per-instructor pass rates) stay —
  they matter to larger schools — but must never *imply* staff that do not exist.

### Backups
- Automatic local SQLite backup on a schedule and on app close, retained with rotation.
- One-click **"Backup to USB"** that writes a timestamped, verified copy.
- Restore is a guarded, explicit flow that never silently overwrites a live database.
- **Never make a backup by copying the `.db` file.** The database runs in WAL mode, so recent
  writes live in the `-wal` sidecar and a file copy silently loses the last day's payments.
  Use `VACUUM INTO`, which asks SQLite for a consistent snapshot while it is in use.
- **A copy is not a backup until it has been verified** — opened, integrity-checked, and its
  row counts compared with the source. An unverified copy is discovered to be worthless on
  the one day it is needed. `write_backup` deletes a copy that fails, so a file present in
  the backups folder always means a good backup.
- **A restore is staged, not applied immediately.** The chosen file is validated and parked,
  and swapped in at the next startup before anything opens the database; the replaced
  database is kept, not deleted. Swapping a file under live connections corrupts it, and the
  stale `-wal`/`-shm` sidecars must be deleted or SQLite replays them onto the restored file.

---

## 3. Stack

Decided in `docs/03-STACK.md` (read it for the reasoning and the rejected alternatives).

- **Shell**: Tauri 2 (Rust) — small installer, low RAM, WebView2 on Windows
- **UI**: React + TypeScript (strict), Vite
- **Styling**: Tailwind CSS (logical properties) + a small in-house primitive set in
  `src/components/ui` (Button, Card, Badge, Segmented, DateField, EmptyState, …).
  **shadcn/ui was planned and never adopted** — there is no `components.json` and no Radix
  dependency. Reconsider it when a control needs real focus management (a combobox, a menu);
  do not half-adopt it, and do not assume it is present.
- **Calendar**: FullCalendar (`@fullcalendar/core`, `react`, `timegrid`, `daygrid`,
  `interaction`), all pinned to the same 6.x minor — the packages must match majors, and
  `interaction` has no 7.x. These are **MIT and free to ship commercially**, provided the
  copyright headers stay intact.
  **FullCalendar Premium is NOT licensed here.** Premium is what provides *resource* views —
  one column per instructor or per vehicle — which is exactly the obvious Pro-tier feature.
  It starts around $480. Do not promise per-instructor columns without budgeting for it.
  The week and month views are FullCalendar; the **day view deliberately is not**. The day is
  a to-do list with Fait/Absent on each row, which is the morning routine that keeps the
  hours current. A grid would make that worse.
  FullCalendar renders its own day headings and "+N more" badge as plain text, which the
  bidi algorithm reorders in Arabic ("17/10" became "10/17", "+1" became "1+"). Both are
  overridden in `PlannerView.tsx` and rendered inside an isolate.
- **Motion**: `motion` (MIT). Transforms only — a card lifting under the pointer. Never
  animate layout, filters or shadows in a list: this runs on 4 GB and an HDD.
- **Icons**: `lucide-react`. **Fonts**: Inter (Latin) + Cairo (Arabic), both OFL and variable, declared in
  `src/styles/fonts.css` against bundled woff2 files. Never add a font or icon CDN — the app
  must render identically with no internet, on screen and on paper.
- **i18n**: i18next + react-i18next, with document `dir` switching
- **Database**: SQLite, accessed from Rust via `sqlx` with compile-time checked queries
- **Rust↔TS boundary**: Tauri commands with `tauri-specta` generating TypeScript types
- **Printing**: HTML print layouts rendered by WebView2 — **not** a Rust PDF library
  (Rust PDF crates generally do not shape Arabic text correctly; the WebView does)
- **Packaging**: NSIS installer, plus an offline WebView2 bootstrapper for older machines

### Architectural rule: the Rust layer owns the business

The frontend is a **view**. It does not contain business rules, money arithmetic,
authorisation checks, or licence validation. All of that lives in Rust commands, because:

1. It is the only layer we can trust — the frontend bundle is trivially modifiable.
2. It gives one testable place for the rules that matter.
3. It keeps licence enforcement out of inspectable JavaScript.

The frontend may read freely for display. It may **never** compute a balance, a price,
or an eligibility decision.

---

## 4. Domain glossary

Use these exact terms in code, in both schemas and identifiers. Do not invent English
substitutes for concepts that have an established Algerian name.

| Code term | French | Arabic | Meaning |
|---|---|---|---|
| `candidate` | candidat | مترشح | A person training for a licence |
| `dossier` | dossier | ملف | A candidate's file of required documents |
| `instructor` | moniteur | مدرب | Driving instructor |
| `enrollment` | inscription | تسجيل | A candidate registered for one licence category |
| `package` | forfait | جزافي | Priced bundle of hours for a category |
| `installment` | versement | دفعة | A partial payment against a package |
| `lesson` | séance | حصة | One scheduled training session |
| `theory` | code / théorie | قانون المرور | Classroom highway-code training |
| `maneuvers` | créneau | المناورة | Parking / manoeuvring exam and training |
| `road` | circulation / conduite | السياقة | On-road driving |
| `exam_session` | session d'examen | دورة الامتحان | A scheduled official exam sitting |
| `passed` / `failed` | admis / ajourné | ناجح / راسب | Exam outcome |
| `retake` | repassage | إعادة | A further attempt after failing |
| `cash_session` | caisse | الصندوق | A day's cash drawer, opened and reconciled |
| `wilaya` | wilaya | ولاية | Province (48/58 of them — seeded, configurable) |
| `agrement` | agrément | رخصة الاستغلال | The school's own operating licence |
| `nin` | NIN | رقم التعريف الوطني | 18-digit national identification number |

---

## 5. Conventions

### Repository layout
```
/src                 React frontend
  /features          One folder per domain area (candidates, lessons, cash, ...)
  /components/ui     shadcn primitives
  /lib               Shared helpers, formatters, generated bindings
  /locales           fr.json, ar.json
  /print             Print-only layouts (none yet — see "Paper is the output")
/src-tauri
  /src
    /commands        Tauri command handlers — one module per feature
    /domain          Business rules, pure and unit-tested
    /db              Queries, connection management
    /license         Licence verification
  /migrations        Forward-only SQL migrations
/docs                Planning documents (numbered)
```

### Naming
- Rust: `snake_case`, TS: `camelCase`, SQL tables: `snake_case` plural (`candidates`).
- Tauri commands: `verb_noun` — `create_candidate`, `list_lessons_for_day`.
- Timestamps: `created_at`, `updated_at`, `deleted_at`.

### Testing
- **Business rules in `src-tauri/src/domain` require unit tests.** Pricing, balance,
  hour-completion, eligibility, and cash reconciliation are the highest-risk code in the
  product; they are pure functions so they can be tested exhaustively.
- Vitest for frontend logic, not for rendering trivia.
- Before calling a feature done, verify it in the running app — not only in tests.

### Code style
- No `any` in TypeScript. No `unwrap()` on a code path a user can reach.
- Errors surface to the user as a translated, actionable message, never a raw error string.

---

## 6. Working agreement with the user

The user is the **product manager**, not a developer on this project. Therefore:

- **Plan and discuss before implementing.** Propose, explain the trade-off, get a decision.
- Explain decisions in product terms (what the school's secretary experiences) and only
  then in technical terms.
- The user is Algerian and is the authority on domain reality: pricing, paperwork, how
  schools actually operate. **Ask them rather than guessing**, and record answers in
  `docs/05-OPEN-QUESTIONS.md`.
- Never assert an unverified regulatory fact as settled. Flag it and ask.

---

## 7. Commands

```bash
npm run dev          # Vite dev server on http://localhost:5173
npm run build        # typecheck + production build
npm run typecheck    # tsc --noEmit
npm run check:i18n   # fails if any key is missing or empty in fr or ar
npm run check:rtl    # fails on physical CSS props and un-isolated numbers
npm run check        # all three — run before every commit
```

```bash
cd src-tauri && cargo test --lib   # domain + db tests
cd src-tauri && cargo test         # the above plus the crash-consistency test
cd src-tauri && cargo run --bin seed  # load demo candidates for a demo or a visual check
```

**The power-cut test is real and must stay in the release checklist.**
`tests/crash_consistency.rs` starts `crash_writer`, lets it write payments, then
kills the process with no chance to flush, and checks the database still opens, has no
duplicate or missing receipt numbers, and that the school's receipt counter agrees with
the payments on disk — which is the proof that allocating a number and inserting a payment
cannot be torn apart. It also asserts the durability pragmas are actually in force, because
losing `synchronous = FULL` in a refactor would silently make the first test meaningless.

**Verify Arabic by looking at it, not by reading the code.** Append `?lang=ar` to any URL to
force Arabic for one load. Bidi and RTL bugs are invisible in source and invisible in French —
every one found so far was found on screen, not in review.

```bash
npm run desktop        # tauri dev - builds Rust and launches the app window
npm run desktop:build  # production build + NSIS installer
```

### Toolchain
Rust 1.99.1 and Visual Studio Build Tools 2022 (MSVC 14.44) are installed.
Cargo lives at `%USERPROFILE%\.cargo\bin`.

### Windows MAX_PATH
`src-tauri/.cargo/config.toml` redirects cargo's `target-dir` to a short path. It is
**machine-local and gitignored**.

Without it the build fails with:

```
LINK : fatal error LNK1104: cannot open file ...build_script_build-*.exe
```

That reads like a missing dependency but is the linker unable to **create** its own output:
the project currently sits under a deep path and cargo's build tree exceeds the 260-character
Windows limit. Moving the project somewhere short (e.g. `C:\dev\tariqi`) fixes it properly and
makes that config file deletable.

### "Invalid hook call" / blank window after adding imports
Vite's dependency cache can end up serving two different optimize hashes (e.g.
`react-i18next.js?v=bd3a91d7` alongside `react-dom_client.js?v=1efda266`), which loads two
copies of React and blanks the whole app. It is NOT a code bug — check `npm ls react` first,
and if it is deduped, stop the dev server, delete `node_modules/.vite`, and restart.
Note the browser console keeps the old messages after the fix, so verify with the rendered
page (`get_page_text`), not the console.

### A debug build loads from the dev server
`devUrl` points at `http://localhost:5173`, so a debug binary launched **without** Vite running
shows a blank "can't reach this page" window. That is expected, not a bug. Release builds embed
the frontend and need no server.

<!-- claude-reflect: learned rules (saved from the band; edit freely) -->
## Learned rules
- Do not add traffic-law or regulatory tracking to this project; it is a business tool for recording hours, candidates, and money in and out.
- Assume a one-time sale for this product; do not propose monthly or annual subscription tiers or recurring pricing.
<!-- end claude-reflect -->
