# Open Questions — Tariqi

**Owner:** PM (you). **Date:** 2026-10-17

You are the authority on Algerian domain reality; I am not, and the public sources are thin
and partly contradictory. Each question is marked with what it blocks.

Record answers inline under each question as they arrive.

---

## Blocking — needed before the relevant phase starts

> **Recalibrated 2026-10-17.** The regulatory questions below were over-weighted. The software
> records what the school does; it does not enforce the law. Hours, categories, and document
> checklists are **editable settings**, so a wrong default costs a minute to fix and blocks
> nothing. See [`07-REGULATORY.md`](07-REGULATORY.md) for why.
>
> **Only Q3 genuinely blocks the build**, because it can change the entire stack.

---

### Q1. Photos of the paperwork a school uses today — **needed for Phase 1 printing** 🟠

Not a legal research task. **Just photograph whatever paper the school currently hands out
and fills in.** That is what we reproduce, because it is what their candidates and their
wilaya office already accept.

- Their **registre** / ledger, if they keep one
- Every sheet inside one candidate dossier
- Registration form, training contract, receipt book, attendance sheet
- Whatever they submit when presenting candidates for an exam

**One useful question per document:** *"Est-ce un modèle officiel ou le vôtre ?"*
Official templates must be reproduced exactly; their own forms can be improved.

**Not a build blocker** — Phase 0 and the whole data side of Phase 1 proceed without it. It is
only needed when I start laying out print templates, and even then a template is an HTML file
that takes an hour to change.

**Answer:**

---

### Q2. Required training hours per category — **seed data only, blocks nothing** 🟡

> These are default values in a settings table. If they ship wrong, the owner edits them in
> under a minute. Answer this when convenient, not before building.


**Found the official text.** A ministerial decree in **_Journal Officiel_ no. 40, 9 June 2024**
defines the current national training programme, applied from the start of 2025. Confirmed
by the Interior Ministry and several Algerian outlets:

| Categories | Theory hours |
|---|---|
| A1, A, B, F (أ1، أ، ب، و) | **30** (raised from 25) |
| C1, C, D (ج1، ج، د) | **20** (raised from 15) |

Also confirmed: the real category codes are **A1, A, B, C1, C, D, F** — not the A1/A2/B/C/D/E/F
set I had guessed. The domain model is corrected.

New theory modules added by the decree: driver ethics (أخلاقيات السائق), first aid and conduct
at an accident scene (الإسعافات الأولية), modern vehicle technologies (التكنولوجيات الحديثة).

**What is still missing — this is the ask:**
1. **Practical hours per category.** The decree raised them but no press report gives the
   numbers, and they are needed for seed data.
2. **The full theory module list with hours per module**, since theory is now modular.
3. Whether **créneau** carries its own separate hour requirement.

**How to get it:** the decree text in JO no. 40 of 2024. `joradp.dz` refused connections from
outside Algeria, but should work for you:
- `https://www.joradp.dz/FTP/JO-ARABE/2024/A2024040.pdf`
- `https://www.joradp.dz/FTP/JO-FRANCAIS/2024/F2024040.pdf`

If those 404, browse the year index at `joradp.dz` for 2024, issue 40. The Centre National
des Permis de Conduire (CNPC) is the other route.

**Answer:**

---

### Q3. Windows version on real school PCs — **blocks Decision 1 in `03-STACK.md`** 🔴
What do the schools you have seen actually run? Specifically, **is any of them on Windows 7
or 8?**

**Why it blocks:** Tauri requires WebView2 and does not support Windows 7/8. If a meaningful
share of target schools run Windows 7, the whole shell decision changes to .NET + WPF. This
is the cheapest question to answer and the most expensive to get wrong.

**Tool provided — don't answer this by eye.** [`tools/survey-pc.ps1`](../tools/survey-pc.ps1)
is a read-only script that reports the Windows version and build, a plain YES/NO verdict on
WebView2 viability, RAM, disk, screen size, installed printers (flagging likely thermal
receipt printers), and whether the machine is online. It writes a `.txt` report beside itself.

Put it on a USB stick, run it on every school PC you can reach, and collect the reports.
It is written to run on PowerShell 2.0, so it works on Windows 7 too — which is the point.
Tested and working.

```
powershell -ExecutionPolicy Bypass -File .\survey-pc.ps1
```

One report from a real school PC answers Q3 and Q8 together.

**Answer:**

---

### Q4. Candidate dossier document list — **blocks Phase 1** 🟠
I could not find an authoritative Algerian list. My provisional set:
acte de naissance, certificat de résidence, certificat médical, certificat d'aptitude
visuelle, copie de la carte d'identité nationale, groupe sanguin, photos d'identité,
timbre fiscal.

- Is this right, and what is missing?
- Which have an expiry (medical certificate validity period)?
- Does the list differ by category?

**Answer:**

---

## Important — needed soon, shapes design

### Q15. How big is a typical school, really? 🟠

**Raised 2026-10-17.** You pointed out the owner is usually the instructor too. That changed
the staff model (`02-DOMAIN-MODEL.md §2`) and it changes UI defaults, so I need the actual
distribution rather than an assumption.

- What share of schools are **one person, one vehicle**?
- What share are **owner + one or two employed instructors**?
- How many are big enough to employ a dedicated secretary who is not the owner?
- Does the owner-instructor usually do the paperwork himself, or does a family member help?
- Is there a legal cap on candidates per instructor, or per school? (Reportedly introduced by
  law 26-09 — if real, the app should warn when approached.)

**Why it matters:**
- If most schools are one person, the UI should **hide** instructor and vehicle pickers by
  default, and the multi-instructor calendar is a secondary view rather than the main screen.
- It changes what "roles" means in practice — possibly a first-run question of
  *"Do you work alone?"* that configures everything.
- It changes **pricing**: a solo school's willingness to pay is very different from a
  six-vehicle school's, which may argue for tiering by instructor or vehicle count.
- It changes which features sell. Cash variance tracking is compelling to an owner watching
  employees and close to pointless to a man watching himself.

**Easy to answer from your own knowledge plus the school visit.**

**Answer:**

---

### Q5. Exam structure and current terminology 🟠
- Are the exams still **code**, **créneau**, and **circulation** as three separate sittings?
- Must code be passed before the practical ones?
- What happens on failure — is there a waiting period before a retake?
- Does a passed component expire if the others are not passed in time?

**Answer:**

---

### Q6. Money practices 🟠
- Do schools price as a **forfait** per category, per hour, or both?
- Typical instalment pattern (how many payments, at what points)?
- Does the school collect the **state exam fee / timbre fiscal** from the candidate, or does
  the candidate pay it directly?
- May a candidate with an outstanding balance be presented for an exam?
- Do retakes cost extra, and roughly how much?
- Does a no-show consume a paid hour?

**Answer:**

---

### Q7. Instructor pay model 🟡
Hourly for hours taught, fixed monthly salary, commission on passes, or a mix?
Determines whether Phase 6 reporting needs payroll-shaped output.

**Answer:**

---

### Q8. Printers at the counter 🟡
A4 laser/inkjet only, or do schools also use **thermal receipt printers**? If thermal is
common, ESC/POS support moves up the roadmap.

**Answer:**

---

## Commercial — needed before Phase 7

### Q9. Pricing and collection 🟠
- Annual licence price per school in DZD?
- How will schools pay you — BaridiMob, CCP, bank transfer, Edahabia, cash via a rep?
- A free tier capped by candidate count, as an acquisition tool — yes or no?
- Trial length?
- Sell direct, or recruit a reseller per wilaya?

**Answer:**

---

### Q10. Code signing certificate 🟡
Without one, Windows SmartScreen warns on every install, which hurts conversion badly for a
paid product sold to non-technical buyers. It is a real annual cost. Worth it?

**Answer:**

---

### Q11. Pilot school 🔴
Do you have a specific school that will pilot this? Which wilaya? How many vehicles,
instructors, and candidates per month?

**Why it matters:** the pilot is how the product gets right, and its size sets the
performance targets. Without a committed pilot, Phase 8 has nothing to run on.

**Answer:**

---

### Q12. Product name 🟡
Working name is **Tariqi** (طريقي, "my road"). Other candidates: **Rokhsa** (رخصة, "licence"),
**Siyaqa** (سياقة, "driving"), **Sawaq** (سواق, "driver"), or something French-leaning like
**AutoÉcole Pro**.

Needs to work in both languages, be easy to say on the phone, and ideally have a free
`.dz` or `.com` domain. Your call.

**Answer:**

---

## Legal

### Q13. Law 18-07 personal data compliance 🟠

**Research done — and it is more serious than I assumed.** What the sources establish:

- Law **18-07** is enforced by the **ANPDP** (Autorité nationale de protection des données à
  caractère personnel), an independent authority. It applies to private entities, not only
  public ones.
- **A prior declaration to the ANPDP is required before processing begins**, covering the
  controller's identity, purposes, data categories, recipients, retention period and security
  measures. Processing that presents a manifest risk to privacy needs **prior authorisation**,
  not just a declaration.
- **Transfers outside Algeria require ANPDP authorisation**, with limited exceptions.
  → **This directly constrains the cloud-sync add-on.** Hosting school data on OVH France
  would be a cross-border transfer. Worth knowing now, while the add-on is still only a plan.
- Penalties are reported up to **5 years' imprisonment and 500,000 DZD**, though sources
  disagree on the fine ranges.
- A **2025 amendment (law 25-11)** reportedly adds mandatory processing registers, impact
  assessments for high-risk processing, breach notification within 5 days, and possibly a
  mandatory DPO. **Unverified against the official text.**

**Questions that remain:**
1. Does an auto-école itself have to file the ANPDP declaration, or does our product carry
   any obligation as a processor? (Likely the school's, but confirm.)
2. Is a candidate consent notice required, and does a standard form exist?
3. Is there a mandated retention or deletion period for candidate files?
4. What exactly does law 25-11 change?

**Design consequences I am already taking:**
- Backups encrypted (already in `03-STACK.md`)
- A printable candidate privacy/consent notice belongs in the print subsystem
- A data-retention and deletion feature is v1 scope, not post-v1
- Cloud sync must be designed assuming a cross-border transfer authorisation is needed

**Still needs a local legal opinion, not my reading of compliance blogs.**

**Answer:**

---

### Q14. Is there a state digital platform? — **upgraded to important** 🟠

Two separate hints, neither confirmed:

1. One source said candidate data has been entered at the Ministry of Transport since 2006,
   suggesting an internal system — possibly one only officials touch.
2. Reports on law 26-09 mention linking school programmes to **digital evaluation platforms**.
3. A report described a **state distance-learning platform rejected by ~1,200 professionals**
   as hard to access and badly suited to candidates starting on different dates.
   **But that source was `snrtnews.com` — the Moroccan national broadcaster — and nearby
   results concerned Morocco's NARSA "Perminou" platform. It may not be Algerian at all.**
   I am not willing to assert it is.

**Why this is now important rather than a curiosity:**
- If Algeria is building a state platform, it is either a **competitor** or an
  **integration target**. Integrating would be the strongest differentiator available.
- If professionals genuinely rejected it as unusable, that is precisely our gap.
- If it mandates data submission in a particular format, that becomes a v1 requirement.

**Questions:**
1. Does an Algerian state platform for driving schools exist today? What is it called?
2. Do schools have to use it? What do they think of it?
3. Does law 26-09 or its implementing texts mandate electronic submission?

**Easy to answer during the school visit — just ask the owner.**

**Answer:**

---

## How exams actually work — ANSWERED 23 Oct 2026

Asked because the app had no exam record at all: a single `enrollments.status`
column, which cannot say "passed the code, failed the créneau, hasn't sat the
circulation yet".

**Answered by the PM, who is the authority on this:**

### 1. Candidates are presented in batches, not individually

The school does not get a time per candidate. It is told, in effect, *"these
three, tomorrow at 08:00, at this centre, for the code"*. One date, one place,
one exam type, several candidates.

→ So an **exam sitting is a real object**, not a date field on a candidate.
The school needs to assemble the batch, print who is going, and afterwards
record who passed. Modelling it as a per-candidate date would lose the batch,
which is the thing the school's own paperwork is organised around.

### 2. The three exams are sequential, and each gates the next

```
  code ──pass──▶ weeks of créneau training ──▶ créneau exam
                                                   │
                                                 pass
                                                   ▼
                            weeks of circulation training ──▶ circulation exam
```

A candidate sits the code first. Once passed, the instructor teaches créneau
for several weeks; when the candidate is ready the school presents them for the
créneau exam. Same again for circulation.

Each exam is passed or failed **on its own**. Failing one means retaking *that
one* — a *repassage* — while the earlier passes stand.

→ So the three session kinds (`theory` / `maneuvers` / `road`) are not merely
lesson types. They are the three **stages of the journey**, and which stage a
candidate is in is derivable from which exams they have passed.

### What this means for the build

- `enrollments.status` as a single stored value is the wrong shape. The stage
  is **derived** from the exam results, the same argument as the balance: a
  stored "passed" that disagrees with the result rows is the bug.
- **Record, do not enforce.** Per CLAUDE.md the app never refuses an action
  because a rule says so. If the school presents someone for the créneau who
  has not passed the code, the app warns and lets the owner decide. Schools
  have reasons we do not know about.
- A retake is not a correction — it is a new result for the same exam, with the
  earlier failure kept. Attempt count matters to the owner.

**Still unknown, and deliberately not assumed:**
- Whether there is a mandatory waiting period between a failure and a retake.
  Not needed: the app records dates, it does not police them.
- Whether a cap on attempts exists. Same answer.

---

## Q16. How the money actually works — **ANSWERED, THEN CORRECTED 2026-10-24** 🟢

Asked because the app had been built on an hour-based *forfait*: a price per
category, a target of N hours, and a progress bar counting them down. The owner
was asked plainly what a candidate pays and when.

### The owner's answer, in his words

> They pay the first time when they subscribe or whatever. They pay, so they
> learn and all that stuff. And then every time they pass an exam, for example,
> Code or Créneau, they pay for that too, separately. But they pay for it once,
> you know what I mean? So for example, if I have one of the candidates and he
> have Code tomorrow, he will pay me 3,000 that I will pay the examinator or
> whatever it's called. And if he succeeds, he moves to the next one. And if he
> doesn't, he doesn't pay for the next time he passed the exam.

Confirmed back to him as a story and agreed: *"Yes, exactly."* He added one
requirement — **"let's keep the each exam fee flexible, you know, just in case
we don't know"** — i.e. the amount must be editable, not shipped as a constant.

### The model

| What | When | How often |
|---|---|---|
| Training fee | At registration | Once per enrolment, payable in *versements* |
| Exam fee | Before each exam | Once per **stage**, never again for a *repassage* |

1. Registration fee — for the training itself, paid in instalments as before.
2. An exam fee per stage, collected by the school and handed to the examiner.
3. **A retake is free.** He has already paid for that exam.

### What this means for the build

- The fee attaches to the **stage**, not to the sitting. `0007_exam_fees.sql`
  makes this a unique index on `(enrollment_id, kind)`, so the free repassage
  is a database constraint rather than a rule every caller has to remember.
- The charged amount is **snapshotted** on the charge row, for the same reason
  the agreed price is snapshotted on the enrolment: the administration raising
  the code fee next year must not change what last year's candidate owes.
- Fees are seeded at **zero**. The 3 000 DZD quoted is one school, one wilaya,
  one year, and the amount is set by the administration, not by us — shipping
  an invented number would be read as authoritative.
- Charging happens as a side-effect of entering a candidate for a sitting, in
  the same transaction. There is deliberately no "charge this candidate"
  command: a fee with no exam behind it is a line nobody can explain.
- The exam fee is tracked **apart from the school's own revenue**
  (`Charges::school_due`), because it is collected on behalf of the examiner
  and was never the school's money. A discount reduces the school's price and
  must not touch the examiner's fee.

**Decided without asking, flagged to the owner, overrulable:**
- Registration remains payable in instalments — that was already the design and
  he did not contradict it.
- The examiner's money is recorded as money in, tagged as an exam fee, so the
  cash drawer is still right while revenue figures can exclude it.

**Still open:**
- Whether the school ever adds a margin on top of the examiner's fee.
- Whether the fee differs by licence category. Currently one amount per stage
  for the whole school.

---

## Q17. Group training — **PARTIALLY ANSWERED 2026-10-24** 🟠

> It doesn't work with hours in here. It works like, for example, I bring three
> people and I tell them, like if I'm the instructor, I bring three people to
> learn to do Créneau. I give them a day and time. They come and they do it one
> by one. And I like tell them next week and the other week until the exam
> comes. [...] Who succeeded goes to the next step, and who didn't, the
> instructor will reschedule him with another group or something like that. And
> if [...] one of them couldn't come, I will have to like make it up the other
> week or whatever day helps him or her.

### What this establishes

- The instructor does not sell hours. He runs a **group through a stage**: a
  recurring slot, same people, week after week, until he judges them ready.
- Candidates take their turn **one by one** inside the session.
- Failing means being folded into **another group**, not starting over.
- **Attendance has a consequence.** A candidate who misses is owed a make-up on
  another day — an absence is not merely a mark.

### What this means for the build — NOT YET DONE

- `lessons.enrollment_id` is `NOT NULL`: one lesson belongs to one candidate.
  A group session needs several candidates on one session, each present or
  absent. Every screen built so far assumes one-per-lesson.
- The hour counters and `6 h / 30 h` progress bars on the candidate file
  describe a product nobody sells. They are still in the app.

**Still unanswered, and blocking the group work:**
- Whether an hours figure is needed anywhere at all — for the dossier, the
  wilaya paperwork, or an inspection — or whether it can go entirely.
- Whether a group is a lasting named thing the owner manages, or simply
  whoever he called for Tuesday.

---

## Q16b. The correction — **2026-10-24** 🟢

Q16 was recorded as "the fee is paid once per stage, a repassage is free", and 0007
enforced it with a unique index. Shown the result, the owner corrected it:

> For the fees, for each time they go and do the exam, I said we let it be flexible.
> It's not a one-time thing. Maybe one-time thing for some people, and for other
> schools, it's not a one-time thing. [...] we can just have it somewhere else where
> the owner is entering a candidate for an exam instead of putting this in the settings
> as a fixed number thing.

**What was wrong:** the free repassage is one school's practice, and it was written into
the schema as a law. CLAUDE.md's first rule about regulatory values says the app records
what the school does and never refuses an action — 0007 refused one.

**The model now:** there is no school-wide amount. The owner types what the candidate is
paying as he enters him for the sitting; empty means nothing is owed, which is how a free
retake is said. A charge belongs to that entry, so removing the entry removes the fee.
0008 drops the index, drops the settings table, and links each charge to its result.

**Settled when asked:** whether to remember a default to save typing — no, typed every
time.

---

## Q18. Receipts — **ANSWERED 2026-10-24** 🟢

> I don't think we need the receipts, the receipt system. They have their own cards,
> they can write it down. This software is not for that.

The printed receipt, its route, the Arabic print proof that used it as a specimen, and the
receipt-numbering section in Réglages are all removed.

**Kept, deliberately, and invisible:** the gapless sequential number on every payment. It
is not for the candidate — it is the only thing that makes a *deleted* payment detectable,
since a missing number is a hole that survives the row being gone. It is also what the
power-cut test checks, and that test is the proof that allocating a number and inserting a
payment cannot be torn apart. The owner never sees it.

**Consequence to watch:** the app now prints nothing at all, so the "does Arabic print
correctly" risk is unproven again. It returns with the first real printout — most likely
the exam batch list, since the school has to hand the examiner a list of who is coming.

---

## Q19. Settings — **ANSWERED 2026-10-24** 🟢

> Do we really need the drive and school information in the settings too? The
> entire software is about just managing and stuff.

Checked before answering, and the owner was right on the evidence: **nothing read
the school information except the page that edited it.** No screen showed the
name, address, phone or agrément. The printed receipt was their only consumer and
it had been removed the same day.

Worse, `defaultLanguage` was a **dead control**. The app takes its language from
`localStorage`, written by the switcher in the sidebar (`src/i18n/index.ts`); the
settings value was saved to the database and never read. Changing it did nothing,
and it had looked functional the whole time.

So Réglages is gone. The nav item is now **Sauvegarde**, named for the one thing
left on it.

**Deliberately kept:** `db/school.rs` and the `school` row. `db::payments`
allocates every receipt number from `next_receipt_number`, and the tests there
encode why the counter freezes once money has moved.

**What brings the fields back:** the first real printout. An exam batch list is a
document *from* the school and wants its name and agrément at the top — two or
three fields, added when we know which, rather than seven kept on speculation.

**Blocking that, and still unanswered since the start:** Q8 — do schools have a
working printer at the counter, and is the examiner's list handed over on paper or
read out? Until that is known, building any printing is speculative.
