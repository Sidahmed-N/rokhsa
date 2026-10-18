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
