# Domain Model — Rokhsa

**Status:** Draft for PM review
**Date:** 2026-10-17

Entities, the rules that bind them, and the workflows they have to support.
Field lists are indicative, not final DDL.

---

## 1. Configuration (seeded, editable — never hardcoded)

### `wilayas`
`code`, `name_fr`, `name_ar`. Seeded with all wilayas. Used for addresses, exam centres,
and the school's own agrément.

### `licence_categories`
The school's own list of what it teaches and what it charges. **Plain configuration, like a
price list** — not regulatory tracking. The owner types in his categories, his prices, and
how many hours he gives.

| Field | Notes |
|---|---|
| `code` | Whatever the school calls them — `A1`, `A`, `B`, `C1`, `C`, `D`, `F` as defaults |
| `code_ar` | Arabic label |
| `name_fr`, `name_ar` | Display names |
| `theory_hours_target` | How many theory hours this category includes |
| `practical_hours_target` | How many driving hours it includes |
| `default_price_centimes` | Suggested price, copied onto a new enrolment |
| `is_offered` | A given school does not teach every category |
| `sort_order` | |

**These are targets, not rules.** The app shows "18 of 30 hours" and nothing more. It never
refuses to record an hour, never blocks an action, and never compares anything to a law.
Per `CLAUDE.md`: the app records what the school does.

Defaults are seeded so a new school is usable immediately, and **every field is editable in
settings** — if a default is wrong for a school, the owner fixes it in under a minute.

### `document_requirements`
The candidate dossier checklist, configurable per category.

`code`, `name_fr`, `name_ar`, `applies_to_categories`, `is_mandatory`,
`has_expiry` (e.g. *certificat médical*), `default_validity_days`, `sort_order`.

Provisional list to confirm with PM: acte de naissance, certificat de résidence,
certificat médical, certificat d'aptitude visuelle, copie carte d'identité nationale,
groupe sanguin, photos d'identité, timbre fiscal.

### `exam_types`
`code` (`code`, `creneau`, `circulation`), `name_fr`, `name_ar`, `state_fee_centimes`,
`order_in_sequence`, `must_pass_before` (dependency — typically code precedes the practical).

### `payment_methods`
`code`, `name_fr`, `name_ar`, `requires_reference`, `is_cash`, `is_active`.
Seeded: espèces, chèque, virement bancaire, BaridiMob, Edahabia, CIB.

### `expense_categories`
Seeded: carburant, entretien véhicule, assurance, vignette, contrôle technique, loyer,
salaires, taxes et impôts, fournitures, divers.

---

## 2. The school and its users

### `school`
Single row in v1 (multi-branch deferred).
`name_fr`, `name_ar`, `agrement_number`, `agrement_date`, `wilaya_code`, `commune`,
`address_fr`, `address_ar`, `phone`, `email`, `nif`, `logo`, `letterhead_settings`,
`default_language`, `receipt_number_prefix`, `next_receipt_number`.

### Staff — one person, several hats

**Corrected 2026-10-17.** The earlier model had a `users` table with a single `role` column
and a separate `instructors` table. That was wrong for this market: **in most Algerian
auto-écoles the owner is also an instructor**, and in a one-person school he is the secretary
too. A single-role column forces either a duplicated identity or a lie about who does what.

Three tables, cleanly separated into *who they are*, *what they may do*, and *what they need
in order to teach*:

#### `staff`
The person. One row per human, ever.
`first_name_fr/_ar`, `last_name_fr/_ar`, `phone`, `nin`, `address`, `photo`, `hired_at`,
`is_active`, `notes`.

#### `staff_roles`
`staff_id`, `role` (`owner` | `manager` | `secretary` | `instructor`).
Many rows per person. **`owner` + `instructor` is the expected default**, not an edge case.

#### `instructor_profile`
Only for staff who teach. `staff_id` (PK), `authorization_number`, `categories_taught`,
`certification_expires_at`, `pay_model` (`hourly` | `salary` | `commission` | `owner_draw`),
`hourly_rate_centimes`, `monthly_salary_centimes`.

`certification_expires_at` feeds the alerts dashboard — the reported five-yearly
continuing-education obligation makes this a genuine compliance risk, and it applies to an
owner who teaches exactly as it does to an employee.

#### `user_accounts`
Login credentials, **optional** — a staff member who never uses the computer needs no account.
`staff_id`, `username`, `password_hash` (Argon2id), `preferred_language`,
`stay_signed_in`, `last_login_at`, `is_active`.

### Permission rule
A person's permissions are the **union across all their roles**. Restrictions are negative
capabilities that only bite when no other held role grants the thing:

| Holds | Sees the books? | On the schedule? |
|---|---|---|
| `owner` | Yes | No |
| `instructor` | No — own schedule only | Yes |
| `owner` + `instructor` | **Yes** | **Yes** |
| `secretary` | No aggregate revenue | No |

**Invariants:**
- At least one active person with the `owner` role must always exist. Refuse to deactivate or
  strip the role from the last one.
- `lessons.instructor_id` references a `staff_id` that **must** have an `instructor_profile`.
- `payments.received_by` references a `staff_id`, so money is always attributable to a human
  even when that human is the owner.

### `audit_log`
`user_id`, `at`, `entity_type`, `entity_id`, `action`, `before_json`, `after_json`.
Written for every money mutation, every result entry, and every permission change.
Append-only; never editable from the UI.

---

## 3. People

### `candidates`
```
nin                  18-digit national ID — unique where present
first_name_fr/_ar    both stored, neither derived
last_name_fr/_ar
father_name_fr/_ar   required on Algerian forms
mother_name_fr/_ar   required on Algerian forms
birth_date
birth_place_fr/_ar
birth_wilaya_code
gender
blood_group
address_fr/_ar, commune, wilaya_code
phone, phone_alt
photo
registered_at
status               prospect | active | inactive | completed | withdrawn
notes
guardian_*           for minor candidates (relevant to A1)
```

**Search must work on either script, on NIN, and on phone.** This drives an index strategy
and a normalised search column — Arabic search needs diacritic and alef-form normalisation
to be usable.

### Instructors
Not a separate table — see **Staff** in §2. An instructor is a `staff` row holding the
`instructor` role plus an `instructor_profile`. Usually that same person also holds `owner`.

### `candidate_documents`
`candidate_id`, `requirement_code`, `status` (`missing` | `received` | `expired`),
`received_at`, `expires_at`, `attachment_path`, `notes`.

**Derived rule:** a dossier is *complete* when every mandatory requirement for the
candidate's enrolled categories is `received` and not expired. Computed in Rust, not stored.

---

## 4. Training

### `enrollments`
A candidate enrolled in **one category**. A candidate may hold several.

```
candidate_id, category_code
enrolled_at
package_id                  the forfait applied
agreed_price_centimes       snapshot — never read live from the price list
discount_centimes, discount_reason
status    forming | ready_for_exam | presented | passed | failed | withdrawn
```

**Invariant — price snapshotting:** `agreed_price_centimes` is copied at enrolment.
Changing the price list must never retroactively alter what an existing candidate owes.
This is the most common and most damaging mistake in this class of software.

### `packages` (forfaits)
`name_fr/_ar`, `category_code`, `included_theory_hours`, `included_practical_hours`,
`price_centimes`, `extra_hour_price_centimes`, `is_active`, `valid_from`, `valid_to`.

### `lessons` (séances)
```
enrollment_id, instructor_id, vehicle_id (nullable for theory)
type                  theory | maneuvers | road
scheduled_start, scheduled_end
actual_start, actual_end        separate columns — both matter
status                planned | completed | absent | cancelled
odometer_start, odometer_end
consumes_paid_hour    boolean — policy decision for no-shows
notes
candidate_signed      attendance capture
```

**Invariants enforced in Rust:**
1. No overlapping `planned`/`completed` lesson for the same instructor.
2. No overlapping lesson for the same vehicle.
3. No overlapping lesson for the same candidate.
4. A vehicle must serve the enrolment's category.
5. An instructor must be authorised for the enrolment's category.
6. Hours completed is the **sum of actual durations of `completed` lessons** — never a
   stored counter that can drift.

### Progress (derived, not stored)
Per enrolment: hours completed by type vs required by type, percentage, hours remaining,
and whether the regulatory minimum is met. Pure function, heavily unit-tested.

---

## 5. Money

This is the subsystem where correctness is non-negotiable. **All amounts integer centimes.**

### `payments` (versements)
```
enrollment_id (or candidate_id for non-enrolment fees)
amount_centimes          positive; a correction is a separate negative row
paid_at
method_code, reference
receipt_number           unique, gapless, never reused
received_by_user_id
cash_session_id          when method is cash
reverses_payment_id      set on a correcting entry
notes
```

**Invariants:**
- Append-only. No edits, no deletes. Mistakes are reversed with a linked negative entry.
- Receipt numbers are allocated in a transaction and are **gapless** — a gap looks like theft
  to an owner and like fraud to an inspector.
- Balance is always **derived**: `agreed_price − discount + extra_hours_charged − sum(payments)`.
  Never a stored field.

### `cash_sessions` (caisse)
`opened_at`, `opened_by`, `opening_float_centimes`, `closed_at`, `closed_by`,
`counted_centimes`, `expected_centimes`, `variance_centimes`, `notes`.

**This is the feature an owner will pay for.** It makes cash shrinkage visible: expected is
derived from the session's cash payments and cash expenses; variance is recorded, not hidden.

**Invariant:** only one open cash session at a time. A cash payment outside an open session
must be refused.

### `expenses`
`category_code`, `amount_centimes`, `spent_at`, `vehicle_id` (nullable), `supplier`,
`description`, `paid_by_user_id`, `cash_session_id`, `attachment_path`.

### `state_fees`
Money collected from the candidate on the state's behalf (exam fees, *timbre fiscal*).
Tracked separately so it never inflates the school's revenue figures.

---

## 6. Exams

### `exam_sessions`
`exam_type_code`, `session_date`, `centre_name`, `wilaya_code`, `notes`, `status`
(`planned` | `held` | `cancelled`).

### `exam_registrations`
```
exam_session_id, enrollment_id
attempt_number
state_fee_paid
result            pending | passed | failed | absent
result_recorded_at, recorded_by_user_id
```

**Eligibility check (Rust, surfaced in the UI before adding a candidate):**
- Dossier complete and no expired mandatory document
- Required hours for the relevant exam type met
- Prerequisite exams passed (code generally precedes the practical)
- Balance within an allowed threshold — **policy, configurable; PM to decide whether a
  candidate with an outstanding balance may be presented**
- Minimum age met at the session date

**Retakes:** a `failed` result makes the enrolment eligible for a new registration with
`attempt_number + 1`. Whether a retake costs extra is a configurable policy.

---

## 7. Fleet

### `vehicles`
`plate` (Algerian format), `make`, `model`, `year`, `categories_served`,
`transmission` (manual | automatic), `fuel_type`, `has_dual_controls`,
`current_odometer`, `acquired_at`, `is_active`.

### `vehicle_documents`
`vehicle_id`, `type` (`assurance` | `vignette` | `controle_technique` | `carte_grise`),
`number`, `issued_at`, `expires_at`, `attachment_path`.
Drives expiry alerts.

### `maintenance_records`
`vehicle_id`, `performed_at`, `odometer`, `type`, `description`, `cost_centimes`,
`supplier`, `next_due_at`, `next_due_odometer`.

### `fuel_logs`
`vehicle_id`, `filled_at`, `odometer`, `litres`, `cost_centimes`, `station`.
Enables cost-per-km, which tells the owner which vehicle is losing money.

---

## 8. Printing

Each is an HTML print layout with French and Arabic variants, rendered on school letterhead.

| Document | French name | Trigger |
|---|---|---|
| Registration form | fiche d'inscription | On enrolment |
| Training contract | contrat de formation | On enrolment |
| Payment receipt | reçu de versement | On every payment |
| Attendance sheet | feuille de présence | Per period / per instructor |
| Progress statement | relevé de formation | On request, and before exam |
| Exam session list | liste des candidats | Per exam session |
| Dossier cover | chemise de dossier | On enrolment |
| Candidate account statement | relevé de compte | On request |

> **Blocking dependency:** the official forms and registers the Direction des Transports
> requires must be obtained before this subsystem is built. Everything above is inferred.

---

## 9. Core workflows

### Register a candidate — the speed-critical path
1. Search first, to avoid duplicates (either script, NIN, phone).
2. Identity entry. Minimum viable set, the rest deferrable — **a secretary must be able to
   register someone who has not brought every paper yet.**
3. Enrol in a category, pick a package, apply any discount.
4. Dossier checklist created from the category's requirements; tick what was brought.
5. Take a first instalment, print the receipt.
6. Print registration form + contract + dossier cover.

Target: under 3 minutes, mostly keyboard, no mouse hunting.

### Take a payment — the frequency-critical path
Find candidate → see balance → enter amount and method → print receipt.
Target: under 30 seconds. Must work with an open cash session.

### Schedule and run a lesson
Pick slot on the calendar → conflicts blocked at selection, not on save → candidate attends
→ instructor marks completed with actual times and odometer → progress updates derive
automatically.

### Present candidates for an exam
Create session → app lists eligible candidates with reasons for any ineligibility →
add them → collect state fees → print the list → after the sitting, record results in bulk →
failures become retake-eligible.

### Close the day
Close cash session → count → record variance → review tomorrow's schedule → alerts screen.

---

## 10. Open modelling questions for PM

1. Is *créneau* a separate training type with its own hour requirement, or part of practical?
2. May a candidate with an outstanding balance be presented for an exam?
3. Does a no-show consume a paid hour? Is that per-school policy?
4. Do retakes cost the candidate extra, and is there a standard amount?
5. Are instructors paid per hour taught, fixed salary, or commission on passes?
6. Is there an official register (*registre*) with a prescribed format that must be printable?
7. Do schools collect the state exam fee themselves, or does the candidate pay it directly?
8. Can one candidate train for two categories simultaneously in practice?
