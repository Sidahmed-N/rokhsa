# Roadmap — Rokhsa

> ⚠️ **Superseded as the active plan.** The PM's goal is revenue soon, and this eight-phase
> plan delivers none until Phase 7. **[`09-FIRST-RELEASE.md`](09-FIRST-RELEASE.md) is what we
> are building** — a much smaller, sellable release covering candidates, hours, and money.
>
> This document stays as the longer-term picture: the phases below describe where the product
> goes after the first release pays for itself, and the ordering logic still holds.

**Status:** Long-term reference
**Date:** 2026-10-17

Phases are ordered by dependency and by risk, not by how interesting they are. Sizes are
relative (S / M / L / XL), not calendar estimates — I will not invent dates.

**Guiding principle:** the app's job is to record what the school does — candidates, hours,
money in and out — not to enforce regulation. Almost nothing here depends on an outside
answer. The one real technical risk is Arabic printing, so Phase 0 retires it immediately,
and Phase 2 ends with something a school can look at and react to.

---

## Phase 0 — Foundation `M`

Nothing user-visible. Everything later depends on it being right.

- Scaffold Tauri 2 + React + TS + Vite + Tailwind + shadcn
- SQLite setup: WAL, foreign keys on, durability pragmas, health check
- `sqlx` migration pipeline, forward-only, embedded
- Seed data: wilayas, licence categories, exam types, payment methods,
  document requirements, expense categories
- i18n scaffolding: fr/ar, `dir` switching, Arabic font bundled, CI key-parity check
- RTL-safe component conventions and the lint rule banning physical CSS properties
- Auth: users, roles, Argon2id, login screen, role-gated command layer
- Audit log infrastructure
- `tauri-specta` type generation wired into the build
- Backup service: scheduled local backup, USB backup, guarded restore
- Print infrastructure: hidden render window, letterhead injection, one proof-of-concept
  document in **both** French and Arabic

**Done when:** the app launches, a user logs in, the language switches cleanly including
layout direction, and a letterhead document prints correctly in Arabic. That last item is
the technical risk in this whole project and it gets retired in Phase 0.

---

## Phase 1 — Candidates, dossiers, and the paperwork `L`

- School settings: agrément, letterhead, logo, categories offered, prices
- Candidate CRUD with bilingual identity fields and photo
- Bilingual search (Arabic normalisation, NIN, phone) with duplicate detection
- Document checklist per candidate, expiry tracking, scanned attachments
- Enrolment in a category with package and price snapshotting
- **Printed documents:** registration form, training contract, dossier cover

**Done when:** a secretary can register a real candidate end-to-end in under three minutes
and hand them correct paper.

> **Note:** the data side of this phase needs nothing from outside. Only the three printed
> documents need photos of the school's current paperwork (`05-OPEN-QUESTIONS.md` Q1), so
> build the forms last within the phase and the dependency never bites.

---

## Phase 2 — Money `L`

Brought forward ahead of scheduling, because it is the owner's actual pain and the fastest
route to something they will pay for.

- Packages and extra-hour pricing
- Payments with method, reference, gapless numbered receipts, append-only correction flow
- Derived balance per candidate and per enrolment
- Receivables view with ageing
- Cash session: open, movements, count, variance
- Expenses with categories and vehicle attribution
- State fees tracked separately from school revenue
- **Printed documents:** receipt, candidate account statement

**Done when:** the owner can answer "what am I owed, and did the drawer balance today?" on
one screen, and the numbers survive an adversarial review against the paper notebooks.

**This is the first point at which the product is worth showing to a school.** Get it in
front of the pilot here, before building scheduling, and let their reaction reorder what follows.

---

## Phase 3 — Scheduling and lessons `L`

- Instructors CRUD, categories taught, certification expiry
- Vehicles CRUD (enough for scheduling; fleet depth comes in Phase 5)
- Day/week calendar across instructors and vehicles
- Conflict prevention at selection time (instructor, vehicle, candidate)
- Recurring slots, cancellation, no-show policy
- Lesson completion: actual times, odometer, notes, attendance signature
- Derived progress: hours by type vs required, hours remaining
- **Printed documents:** attendance sheet, progress statement

**Done when:** the wall planner comes down.

---

## Phase 4 — Exams `M`

- Exam sessions (date, centre, type)
- Eligibility engine with stated reasons for ineligibility
- Add candidates, collect state fees
- Bulk result entry, attempt tracking, retake generation
- Pass-rate reporting by instructor and by category
- **Printed documents:** exam session candidate list

**Done when:** the pilot school assembles a real exam session from the app instead of by hand.

---

## Phase 5 — Fleet and operating costs `M`

- Vehicle documents with expiry alerts (assurance, vignette, contrôle technique, carte grise)
- Maintenance records with next-due tracking
- Fuel logs, cost per km
- Vehicle utilisation and profitability

---

## Phase 6 — Dashboard, alerts, reports `M`

- Alerts: expiring documents, expiring instructor certification, contrôle technique due,
  overdue balances, exam-ready candidates, candidates gone inactive
- Reports: revenue by period and category, receivables ageing, instructor hours and
  productivity, vehicle utilisation, pass rates
- Export to Excel/CSV, because an owner's accountant will ask

---

## Phase 7 — Product hardening and release `L`

- Licence system: Ed25519 verification, grace period, machine binding
- NSIS installer with offline WebView2 bootstrapper
- Updater, plus the USB full-installer path
- Code signing (pending PM decision on cost)
- End-to-end test suite over the critical paths
- **Power-cut test** in the release checklist
- Performance pass against a 4 GB HDD machine with a few thousand candidates
- **Self-sufficiency work — the sale is one-time, so the product must run without you:**
  first-run wizard, demo data, printable user guide in French and Arabic, self-service
  restore, one-click diagnostics report, and error messages that say what to do next.
  Every support call this prevents is margin kept permanently.

**Done when:** a school with no IT support can install it from a USB stick and start working.

---

## Phase 8 — Pilot `M`

One real school, one full exam cycle, from registration through results.
Data migration from their notebooks. On-site observation of the secretary using it —
not a feedback form; watching where they hesitate.

**This phase decides whether the product is right.** Expect it to generate more change than
any build phase. Budget for that rather than treating pilot feedback as scope creep.

---

## Post-v1 — ranked by commercial value

Re-ranked for the **one-time sale** model: every item here must be sellable as its own
purchase, because there is no renewal to carry it.

1. **Code-de-la-route module** `XL` — bilingual question bank, mock exams, per-candidate
   scores. **Now the top post-v1 item**: it is a clean one-time add-on sale to every existing
   customer, it is what candidates actually want, and no competitor has it. Largest content
   effort in the project — the software is the small part.
2. **Paid v2** `L` — the main way a perpetual-licence product earns again from the same
   schools. This is why the licence carries a version ceiling from v1.0.
3. **Thermal receipt printing (ESC/POS)** `S` — small effort, matches counter reality, and an
   easy reason to buy the upgrade. Confirm printer reality first (Q8).
4. **Student Android app** `L` — schedule, hours used, balance, exam date, practice quizzes.
   **Needs rethinking without a sync server:** a local-network link, an exported file, or a
   standalone quiz app paired by code. Worth designing cheaply rather than dropping — it is a
   strong word-of-mouth driver.
5. **Multi-branch consolidation** `M` — for larger schools, once they exist as customers.
6. **SMS / WhatsApp reminders** `M` — needs connectivity and a gateway contract, and a gateway
   is a recurring cost against a one-time sale, so it likely has to be the school's own account.

**Dropped: cloud backup and sync.** A one-time payment against a perpetual hosting bill loses
money on every customer and gets worse with success. The benefit is delivered instead by
backup to the school's own Google Drive / OneDrive folder, to USB, or to another PC on their
network — their storage, no server cost to you. See `08-PACKAGING.md §2`.

---

## Sequencing rationale

Three deliberate choices worth challenging if you disagree:

1. **Printing is proven in Phase 0, not Phase 7.** Correct Arabic print output is the
   highest technical risk; discovering a problem late would be very expensive.
2. **Money comes before scheduling.** Scheduling is more visible, but money is what the
   owner buys. Reaching a showable product one phase sooner is worth more than feature order
   neatness.
3. **The pilot is a phase, not a milestone.** It will change the product. Treating it as
   real work rather than a sign-off step is the difference between a product schools use and
   one they politely abandon.
