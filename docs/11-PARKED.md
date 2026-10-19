# Parked — agreed to come back to

**Parked:** 2026-10-19, by PM decision. Not forgotten, not fixed.

Three gaps found when auditing the backend against `CLAUDE.md`. None of them
blocks a one-person school testing the app today, because the owner is the only
user and "who did it" is always him. All three block selling it.

---

## 1. Price changes leave no trace

**What happens today:** the secretary agrees 35 000 DA with a candidate. Someone
later changes it to 25 000 DA. The app accepts it silently — no record of what
it was, who changed it, or when.

Payments already write an audit row. Nothing else does: enrolment price and
discount, expenses, hours, school settings, and the price list all change
without a trace.

**Why it matters:** this is how money goes missing without anyone being able to
prove it. The audit trail is the whole reason an owner trusts the numbers.

**What it needs:** extend the existing `audit_log` writes to every mutation that
touches money. The table and the pattern already exist in `db/payments.rs` and
`db/candidates.rs` — it is mostly repetition, not design.

**Breaks the rule in CLAUDE.md:** "Every mutation that touches money writes an
audit log row (who, when, before, after)."

---

## 2. There are no logins

**What happens today:** anyone who opens the app can do everything — see total
revenue, change any price, delete any record. There is no way to say "this
person is the secretary; she takes payments but does not see what the school
earns."

`staff` and `staff_roles` tables exist and are empty. `payments.received_by_staff_id`
is always null, so cash cannot be attributed to a person.

**Why it matters:** oversight features (cash reconciliation, who took which
payment) are meaningless without it. It is the difference between a tool for a
solo owner and a tool for a school with staff.

**What it needs:** Argon2id password hashing, a login screen, "stay signed in on
this device" so a solo owner is not nagged daily, and permission checks in the
Rust command layer — never only in the UI. The role model is already designed in
`docs/02-DOMAIN-MODEL.md §2`: one person can hold several roles at once, and
permissions are the union of them.

**Breaks the rule in CLAUDE.md:** "Enforce authorisation in the Rust command
layer, never only in the UI."

---

## 3. Nothing stops the app being copied

**What happens today:** the `.exe` runs anywhere. Another school can take it on
a USB stick and use it. There is no key and no check.

**Why it matters:** the business model is a one-time sale per school
(`docs/08-PACKAGING.md`). Without a licence there is no sale to protect.

**What it needs:** the design is already written in `docs/03-STACK.md`,
Decision 6 — an Ed25519-signed offline licence file, machine-bound, perpetual,
carrying a version ceiling and a nullable expiry. `src-tauri/src/license/` does
not exist yet.

Worth remembering from that document: price is the real anti-piracy strategy,
not the key. The key stops casual copying between schools, which is the actual
volume threat.

---

## Also not built, but deliberately out of the first release

Listed so nobody mistakes them for oversights: daily cash session (open, count,
close — the "Caisse" screen is a period summary, not a reconciled drawer),
candidate document checklist, exam sessions, fleet and vehicle documents,
instructor payroll, and editing a logged lesson (you can add and delete, not
edit).
