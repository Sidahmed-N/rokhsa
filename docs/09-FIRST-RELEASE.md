# First Release — the smallest thing a school will pay for

**Status:** Recommendation for PM decision
**Date:** 2026-10-18
**Goal set by PM: revenue soon. Not a complete product — a sellable one.**

---

## 1. Why the previous plan was wrong for this goal

`04-ROADMAP.md` describes a proper product: eight phases, scheduling, exams, fleet,
reports, the lot. It is the right plan for building the best auto-école software in Algeria.

**It is the wrong plan for getting paid soon.** No money arrives until Phase 7, and every
phase before it is unpaid work. If revenue is the goal, the question changes from
*"what should this product do?"* to **"what is the least I can build that an owner will hand
over cash for?"**

Those have very different answers.

---

## 2. What an owner will actually pay for

From the problem analysis in `01-PRD.md`, the pains ranked by how much they hurt:

| Pain | Hurts | In first release? |
|---|---|---|
| **"I don't know how much money people owe me"** | 🔴 Severe | ✅ Yes |
| **Cash goes through hands with no record** | 🔴 Severe | ✅ Yes |
| **No idea of real profit — money in vs out** | 🔴 Severe | ✅ Yes |
| Candidate files are paper, slow to search | 🟠 Daily friction | ✅ Yes |
| Hours per candidate tracked by memory | 🟠 Costs money | ✅ Yes |
| Double-booked instructors and vehicles | 🟠 Annoying | ❌ Later |
| Assembling exam sessions by hand | 🟡 Periodic | ❌ Later |
| Vehicle documents expiring unnoticed | 🟡 Occasional | ❌ Later |

**The top three are all money.** That is the product. A scheduling calendar is the
impressive-looking feature; the receivables list is the one that opens a wallet.

---

## 3. First release scope

Everything here, nothing else.

### School setup
Name, address, phone, logo, and receipt numbering. One screen. Enough to print paperwork
with the school's name on it.

### Candidates
Add, edit, and search. Bilingual names (FR + AR), phone, NIN, date of birth, address.
Search by either script, by phone, or by NIN. Duplicate warning on save.

### Enrolment
A candidate is enrolled in a category at an **agreed price**, with an optional discount.
Categories are a simple editable list with a default price and an hour target — plain
configuration, nothing more.

### Hours log
Record an hour: date, type (théorie / créneau / conduite), duration, instructor, note.
**No calendar, no conflict checking, no booking.** Just "this happened." Progress shows
hours done against the target on the enrolment.

### Payments — the core of the release
Amount, date, method (espèces, chèque, virement, BaridiMob, Edahabia, CIB), note.
**A numbered, printed receipt every time.** Append-only: a mistake is reversed with a
linked negative entry, never edited away.

### Expenses
A simple log: amount, date, category, note, optional vehicle name. Completes "money in
and out" without a fleet module.

### The three screens that sell it
1. **Candidate balance** — agreed price, paid, owed, hours done.
2. **Receivables** — every candidate who owes money, sorted by how much, with a total.
   This is the screen that closes the sale. Show it first in any demo.
3. **Money in / out for a period** — total collected, total spent, difference.

### Printing
Payment receipt, candidate statement. Both in French and Arabic, on the school's letterhead.

### Non-negotiables carried over
- Full French + Arabic with RTL
- Money as integer centimes, append-only, gapless receipt numbers
- **Automatic backup + one-click USB backup** — never cut this, at any deadline
- Safe to interrupt mid-write; the power will go out
- Works with zero internet
- Licence check

---

## 4. Explicitly not in the first release

Cut for speed. Each is a reason to buy the *next* version.

- **Scheduling calendar**, conflict prevention, recurring slots
- **Exam sessions**, eligibility, results, retakes
- **Document checklist** for dossiers
- **Cash session** open/count/close — payments are still recorded, just not reconciled daily
- **Fleet**: vehicle records, document expiry, maintenance, fuel
- **Multi-user, roles, permissions** — single user in v1
- **Report suite**, Excel export
- **Alerts dashboard**
- Everything in `04-ROADMAP.md` post-v1

**Single user is the big simplification.** Most schools are one person anyway
(`05-OPEN-QUESTIONS.md` Q15), and dropping accounts, roles, and permission checks removes a
large slice of work. The `staff` / `staff_roles` tables still go in the schema so multi-user
is an addition later, not a migration headache — but no UI, no login complexity.

---

## 5. Build order within the release

Each step ends with something demonstrable, so the release can be shown to a school early and
sold before it is finished.

1. **Foundation** — Tauri + React + SQLite, migrations, FR/AR with RTL, backup, and one
   letterhead document printing correctly in Arabic.
   *Arabic print is the only real technical risk in the project; retire it first.*
2. **Candidates** — CRUD plus bilingual search. The app becomes usable as a address book.
3. **Enrolment + hours** — prices and hour targets. Now it tracks training.
4. **Payments + receipt printing** — **the moment it becomes worth money.**
5. **The three money screens** — balance, receivables, money in/out.
6. **Expenses.**
7. **Polish, installer, licence, user manual.**

**Show it to a school after step 5.** Not to collect feedback — to find out whether they will
pay, and how much. That answer changes everything downstream and it is cheap to get.

---

## 6. Two honest notes

### On "quick"
This is genuinely much smaller than the eight-phase plan, but it is not a weekend. The
non-negotiables — bilingual Arabic with correct print output, money correctness, backup,
an installer that works on a weak PC with no internet — are most of the work in any version.
Cutting scheduling and exams removes a lot; it does not make it trivial. I would rather say
that now than have you promise a school a date I cannot hit.

### On switching to yearly later
You said if it does well you would move to a yearly payment. That works, with one constraint
worth knowing now: **schools that already bought a perpetual licence cannot be moved onto a
subscription.** They bought it. Yearly would apply to *new* customers only, leaving two classes
of customer — which is normal and fine, early buyers simply got a better deal.

The thing that makes the switch cost nothing later is small and belongs in this release:
**put a nullable `expires_at` field in the licence file format now, and leave it empty.**
Perpetual licences carry no expiry; if you ever issue yearly ones, you fill the field in. The
verification code reads it from day one. Same reasoning as the version ceiling in
`03-STACK.md` — a field costs nothing today and avoids re-licensing every customer later.
