# Product Requirements — Tariqi

**Status:** Draft for PM review
**Date:** 2026-10-17

---

## 1. The problem

An Algerian auto-école today runs on paper and memory:

- Candidate files live in cardboard folders. Finding out whether a candidate's *certificat
  médical* has expired means opening the folder.
- Lesson scheduling is a wall planner or a notebook. Double-booking an instructor or a
  vehicle is routine, and a candidate who shows up to a double-booked slot is lost goodwill.
- **Nobody knows the real financial position.** Candidates pay in instalments. The owner
  cannot answer "how much money is owed to me right now?" without adding up notebooks.
- Cash goes through several hands with no reconciliation. Shrinkage is invisible.
- Hours trained per candidate are tracked by the instructor's recollection, so schools
  either under-deliver (regulatory risk, bad pass rates) or over-deliver (lost money).
- Exam sessions are assembled by hand each round: who is eligible, who has paid, who is
  on their second attempt.
- Vehicle documents — *assurance*, *vignette*, *contrôle technique* — expire unnoticed until
  a traffic stop.

### Market check

A search for existing software found **no product built for Algerian auto-écoles**. The
available tools are French (Anolla, Drive App, and similar) and assume French categories,
French paperwork, online card payment, and constant connectivity. None of that transfers.

**The gap is not "scheduling software." It is Algerian paperwork plus Algerian cash.**

---

## 2. Who uses it

| Role | Who they are | What they need |
|---|---|---|
| **Owner** (*gérant*) | Owns the school. **Very often teaches as well**, and in a small school does the desk work too | Money: revenue, receivables, cash reconciliation, pass rates, what each instructor and vehicle earns — plus everything the instructor and secretary need, because he is them |
| **Secretary** | Front desk, handles registration and cash | Fast candidate registration, document checklist, take a payment and print a receipt in seconds |
| **Instructor** (*moniteur*) | Teaches théorie / créneau / circulation | Today's schedule, mark attendance, log hours and km |
| **Candidate** | The student | (v1: no software access) Clear statement of hours used, balance owed, exam date |

### These are roles, not people

**In most Algerian auto-écoles the owner is also an instructor** — and in a one- or two-person
school, the secretary as well. He registers the candidate, takes the cash, teaches the lesson,
and closes the books. **Treat that as the default configuration, not the small-business
exception.**

What follows from it:

- One person holds several roles at once, so roles are a set, not a single assignment
  (see `02-DOMAIN-MODEL.md §2`).
- **Nothing may require a hand-off between roles.** If taking a payment assumes a secretary
  and teaching assumes somebody else, the app is unusable for its most common customer.
- Screens must not imply staff who don't exist. With one instructor and one vehicle, the
  pickers for them should not appear at all.
- Logging in cannot be a daily annoyance for a solo owner on his own PC.
- The features that exist for oversight — cash variance, per-instructor pass rates — stay,
  because larger schools need them, but they must read as useful rather than accusatory when
  there is only one person to oversee.

**The primary user is whoever is at the desk, and that is often the owner himself.** The
product succeeds or fails on whether registration and taking a payment are faster than the
notebook. If they are slower, the notebook wins.

---

## 3. Scope of v1

### In scope

**A. School setup**
Agrément details, wilaya, letterhead and logo for printed documents, licence categories
offered with their hour requirements and prices, user accounts and roles, language default.

**B. Candidates and dossiers**
Registration with the identity fields Algerian paperwork requires (bilingual names, NIN,
parents' names, place of birth, blood group, photo). A **document checklist per candidate**
with received/missing state and expiry dates, plus scanned attachments. Search by name in
either script, by NIN, or by phone.

**C. Enrolment and training plan**
A candidate enrols in one or more categories. Each enrolment carries the agreed price, any
discount, and a training plan derived from that category's configured hour requirements.
Progress shows hours completed vs required, per lesson type.

**D. Scheduling**
A day/week calendar across instructors and vehicles. Hard conflict prevention: an instructor
cannot be in two places, a vehicle cannot be in two places, a candidate cannot be in two
lessons. Recurring slots. Cancellation and no-show handling with a policy on whether a
no-show consumes a paid hour.

**E. Lessons and attendance**
Mark a lesson effectuée / absente / annulée. Record actual start and end, odometer start and
end, and instructor notes. Printable **feuille de présence**.

**F. Money**
Packages (*forfaits*) per category, extra-hour pricing, instalment payments with method
(espèces, chèque, virement, BaridiMob, Edahabia, CIB), **numbered printed receipts**, and a
live balance per candidate. A **receivables view**: who owes what, how overdue.
A **daily cash session**: open the drawer, record movements, count at close, record the
variance. Expenses with categories, so the owner sees actual profit.

**G. Exam sessions**
Create a session (date, centre, type). Add eligible candidates — eligibility checks hours
completed and balance owed. Record results per candidate (admis / ajourné / absent), track
attempt number, and generate the retake. Printable candidate list for the session.

**H. Fleet**
Vehicles with plate, category served, transmission, odometer. Document expiry tracking with
alerts. Maintenance and fuel logs attributed to a vehicle.

**I. Alerts dashboard**
One screen answering: documents expiring, instructor certification expiring, vehicles due
for contrôle technique, candidates with overdue balances, candidates ready for exam,
candidates who have gone inactive.

**J. Printing**
The subsystem that makes the product defensible. Every document the school hands over or
files, rendered correctly in French or Arabic, on the school's letterhead:
registration form, training contract, receipt, attendance sheet, candidate progress
statement, exam session list, dossier cover sheet.

**K. Reports**
Revenue by period and by category, receivables ageing, instructor hours and productivity,
vehicle utilisation, **pass rate by instructor and by category** (the number that tells an
owner who to keep).

**L. Licensing, backup, installer, and self-sufficiency**
Perpetual offline licence key, machine-bound, with a version ceiling and **no expiry**.
Scheduled local backup, one-click USB backup, and backup to the school's own cloud folder.
Signed NSIS installer with offline WebView2 fallback.

Because the sale is one-time, **the product must run without you.** These are scope, not polish:
- A **first-run wizard** that configures a school with nobody on the phone
- A **printable user manual in French and Arabic**, shipped with the installer
- **Self-service restore** — the most common support request in this category, and it must
  never require the vendor
- **Error messages that say what to do next**, not what went wrong internally
- A **one-click diagnostics report** the owner can send when he does call

Every support call eats the margin on that sale permanently. This section pays for itself.

### Explicitly out of scope for v1

- Student-facing mobile app — planned, and the Rust command layer is shaped to allow it
- Code-de-la-route quiz / mock exam bank — strong v2 differentiator, large content effort
- Cloud sync, multi-branch consolidation, remote support — the paid add-on that restores
  recurring revenue
- SMS or WhatsApp reminders — needs connectivity and a gateway contract
- Online card payment — not realistic for this market
- Any direct electronic submission to the ministry — nobody has confirmed an interface exists
- macOS or Linux builds
- Payroll beyond "hours worked per instructor"
- Accounting-standard bookkeeping or tax filing output

---

## 4. What makes it win

Ranked by how hard a competitor would find it to copy:

1. **The printed Algerian paperwork is correct.** A French tool can never produce this.
   It requires knowing the actual forms, which the PM can obtain.
2. **Correct Arabic.** Real RTL, real Arabic shaping in print. Most small vendors botch this.
3. **Cash discipline.** The daily cash session and receivables view address the owner's
   single biggest unspoken problem — money leaking — which no scheduling tool talks about.
4. **It works during an outage.** Local-first is a feature here, not a compromise.
5. **Honest hour tracking.** Protects the school on inspection and stops hours being given away.

---

## 5. Success criteria for v1

| Measure | Target |
|---|---|
| Register a new candidate, start to printed form | under 3 minutes |
| Take a payment and print a receipt | under 30 seconds |
| Owner answers "what am I owed?" | one screen, no arithmetic |
| Pilot school stops using its paper register for scheduling and money | the real test |
| Installer size | under 40 MB |
| Cold start on a 4 GB HDD machine | under 5 seconds |
| Data loss after a forced power cut mid-operation | zero committed records lost |

The pilot criterion is the one that counts. One real school running on it for a full exam
cycle — registration through results — validates the product. Everything before that is a guess.

---

## 6. Commercial model

**One-time perpetual sale. No subscription, no annual renewal.** Set by the PM, who knows the
market. Full working-out in [`08-PACKAGING.md`](08-PACKAGING.md); the two consequences that
reach into this document are:

### Support cost is a permanent liability against a single payment
There is no next year's fee to fund it. A school that calls four times has cost more than it
paid. **So self-sufficiency is a product requirement**, not polish — see scope item L.

### Piracy has no backstop, so price is the defence
With a perpetual licence there is nothing to withhold from a cracked copy later. A crack is a
permanently lost sale and a free competitor. The defence is **not stronger DRM**: it is pricing
low enough that paying is obviously easier than hunting for a crack, and reaching volume while
no competitor exists. Keeping Basic genuinely cheap matters more than the margin on each sale.

### Shape
- **Free** edition capped at ~10–15 active candidates — pure distribution, costs nothing
- **Basic** for the one-person school, deliberately cheap, the volume product
- **Pro** at roughly 2–2.5× for schools with employed instructors; upgrading is a top-up
- **Add-on modules** sold once each (code de la route, student app)
- **Paid v2** in two or three years — a new sale to the same schools, which is why the licence
  carries a version ceiling from v1.0
- **Setup, migration and training services** — real work, genuinely valued, impossible to pirate

**Cloud sync is dropped from the plan.** A one-time payment against a perpetual hosting bill
loses money on every customer and gets worse with success. The same benefit comes from backup
to the school's own Google Drive or OneDrive folder, to USB, or to another PC on their network —
their storage, no server cost.

Collection is manual (BaridiMob, CCP, Edahabia, bank transfer, or cash to a representative),
so licence keys are issued by hand. Fine for the first dozen schools; **write the key-generator
script before launch.**

Open: the DZD figures, how many auto-écoles exist in Algeria (in this model that number *is*
lifetime revenue), and whether a per-wilaya reseller network is realistic.

---

## 7. Legal and compliance

- The app stores substantial **personal data** (identity numbers, medical certificate status,
  photos, addresses) and so falls under Algeria's personal data protection law **18-07**.
  Required before pilot: a registration/consent notice the school can present, a documented
  retention policy, and local encryption of backups. **Needs legal confirmation.**
- The school's own obligations to the Direction des Transports define the printed outputs.
  **The exact forms and registers must be obtained before building the print subsystem**
  — this is the single largest unknown in the project.
- Instructor continuing-education obligation (reportedly a five-yearly requirement under a
  ministerial *arrêté* of 14/02/2019) is worth tracking as an expiry alert.

---

## Sources consulted

- [Permis de conduire : types, procédure, examens et coûts — DemarchesDZ](https://www.demarchesdz.com/permis-conduire/)
- [Permis de conduire Algérie 2026 : prix, délais — Sakinadz](https://www.sakinadz.com/blog/guides/permis-conduire-algerie-2026-guide)
- [Les heures d'apprentissage passent de 30 à 55 heures — Le Courrier d'Algérie](https://lecourrier-dalgerie.com/permis-de-conduire-les-heures-dapprentissage-passent-de-30-a-55-heures/)
- [Auto-écoles dans le viseur : nouvelles règles pour les moniteurs — Algérie360](https://www.algerie360.com/auto-ecoles-dans-le-viseur-nouvelles-regles-et-obligations-pour-les-moniteurs/)
- [Algérie : durcissement des règles du permis de conduire — APA News](https://fr.apanews.net/news/algerie-durcissement-des-regles-du-permis-de-conduire/)
- [Permis de conduire catégories A, B, C, F : procédure et dossier](https://formulairesdumonde.com/index.php?id=39)
- [Auto-école (activité réglementée) — Upgrowth DZ](https://www.upgrowth.dz/code-activite/auto-ecole-604612)

**None of the regulatory figures above is confirmed against a published official text.**
Treat all of them as provisional and verify locally.
