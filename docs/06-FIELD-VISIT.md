# Field Visit Checklist — one school, one visit

**Purpose:** come back with enough to unblock Phase 0 and Phase 1.
**Time needed:** about two hours. Morning is better — you see the day running.

Open this on your phone during the visit. Tick as you go.

---

## Before you go

- [ ] USB stick with [`tools/survey-pc.ps1`](../tools/survey-pc.ps1) on it
- [ ] Phone with plenty of storage — you will take 40+ photos
- [ ] Notebook (you will be in rooms where a laptop is awkward)
- [ ] Decide what you tell them. **Suggestion: "I'm building software for auto-écoles and I
      want to understand how yours really works."** Do not pitch. You are here to learn, and
      a pitch makes people describe the school they wish they ran.

---

## 1. Photograph the paperwork — **the single most valuable hour** 🔴

This unblocks Q1 and Phase 1. Everything else here is secondary. Get **every** form, blank
if possible and filled-in as an example (ask permission; cover the candidate's name and NIN).

- [ ] The **registre** / ledger — every column, every page type. Photograph a spread, not a corner.
- [ ] Candidate **dossier**: open one and photograph every single sheet inside it
- [ ] **Fiche d'inscription** (registration form) — blank and filled
- [ ] **Contrat de formation** (training contract) — is it notarised?
- [ ] **Reçu** / receipt book — is it pre-numbered? Carbon copy? Official stamp?
- [ ] **Feuille de présence** (attendance sheet)
- [ ] Whatever they submit to the **Direction des Transports** to present candidates
- [ ] Exam session candidate list, as submitted
- [ ] Any form bearing an official header, logo, or reference number — **especially these**
- [ ] The school's own **agrément** document
- [ ] Their price list, if displayed
- [ ] The wall planner / schedule board — photograph it as it is, mid-week

**Ask:** *"Est-ce qu'il y a un modèle officiel pour ce document, ou c'est le vôtre ?"*
*(هل هذه الوثيقة نموذج رسمي أم نموذج خاص بالمدرسة؟)*

This matters enormously: an official template must be reproduced exactly; a school's own
form can be improved.

---

## 2. Run the survey script — 5 minutes per PC 🔴

Unblocks Q3 and Q8.

- [ ] Run it on **every** PC in the office, not just the main one
- [ ] Fill in the handwritten section at the bottom of each report
- [ ] Photograph the printer(s) — model label included
- [ ] Note: is there a UPS? A generator?

```
powershell -ExecutionPolicy Bypass -File .\survey-pc.ps1
```

If a PC is too old to run it at all, **that is the answer to Q3** — photograph the screen.

---

## 3. Ask the owner (*gérant*)

Money and commercial reality. Q6, Q7, Q9, Q11.

- [ ] How many candidates per month? Per year?
- [ ] How many vehicles? Instructors?
- [ ] 🔴 **Do you teach yourself, or only manage?** (هل تدرّس بنفسك أم تسيّر فقط؟)
      And who does the paperwork — you, or someone else? This shapes the whole UI, so get it
      precisely. Ask the same about the other schools he knows: *"les autres, c'est pareil ?"*
- [ ] Is there now a **limit on candidates per instructor** or per school?
- [ ] Price per category — *forfait* or per hour, or both?
- [ ] How do candidates pay? How many instalments, at what points?
  *"Un candidat paie en combien de fois, normalement ?"*
- [ ] Who collects the **state exam fee** — the school or the candidate directly?
- [ ] **Do you let a candidate sit the exam if they still owe money?**
- [ ] Do retakes cost extra? How much?
- [ ] Does a no-show consume a paid hour?
- [ ] How are instructors paid — hourly, salary, commission on passes?
- [ ] **"How do you know, today, how much money people owe you?"**
      Listen carefully. The answer is usually some version of *I don't really*. That answer
      is the product.
- [ ] Has anyone tried to sell you software before? What happened?
- [ ] What would you pay per year for something that solved this?
      Ask it late, and ask it plainly.

### The new law — ask these too 🔴

Law **26-09** came into force on 20 May 2026 and its implementing texts are due around
November. This is live for them right now, so they will have opinions.

- [ ] *"Comment le nouveau code de la route de mai vous affecte ?"*
      (كيف أثر عليكم قانون المرور الجديد؟)
- [ ] Have you received a new **cahier des charges**? **Ask for a copy.**
- [ ] Is there now a **limit on how many candidates** you can have registered?
- [ ] Do candidates now have to bring a **drug-test certificate**?
- [ ] Are you being told to **replace vehicles** by a deadline?
- [ ] **Is there any government website or platform you have to use?**
      (هل هناك منصة رقمية حكومية ملزمون باستعمالها؟) — answers Q14
- [ ] What are you worried about with the **candidat libre** rule?
- [ ] What has the Direction des Transports asked you for that you cannot easily produce?
      **That is the feature list.**

---

## 4. Shadow the secretary — **watch, don't interview** 🔴

The secretary is the primary user. If registration is slower in the app than in the notebook,
the notebook wins and the product dies. You cannot design this from a description.

- [ ] Watch a **real registration**, start to finish. Time it with your phone.
- [ ] Watch a **real payment** being taken. Time that too.
- [ ] Count how many separate books, files, or papers they touch for one candidate
- [ ] Note every place they write the **same information twice**
- [ ] Ask: *"What takes the longest? What do you do most often?"*
- [ ] Ask: *"What's the worst part of your day here?"*
- [ ] **Do they work in French or Arabic?** Which do they type in? Watch the keyboard.
- [ ] How do they find a candidate's file — by name, by date, by memory?
- [ ] What happens when a candidate arrives having forgotten a document?
- [ ] How do they handle the end of the day with the cash?

---

## 5. Ask an instructor (*moniteur*)

- [ ] How do you know your schedule for tomorrow?
- [ ] How do you record that a lesson happened?
- [ ] How do you know how many hours a candidate has done?
- [ ] Do you carry anything with you in the car — a sheet, a book?
- [ ] Do you have a smartphone? Would you use an app in the car?
- [ ] About your five-yearly training obligation — how do you track when it's due?

---

## 6. Observe without asking

The things nobody tells you because they are too normal to mention.

- [ ] Did the power go out while you were there? How did everyone react?
- [ ] Is the internet working? Ask them if it always is.
- [ ] Where is the cash physically kept?
- [ ] How many people touch the cash in a day?
- [ ] Is the office crowded? Is there a queue at the desk?
- [ ] What is the screen resolution — is it a tiny old monitor?
- [ ] Is anyone using Excel for anything? **Ask for a copy of that file.** A school that
      already built a spreadsheet has told you exactly what it needs.
- [ ] What language are the papers on the walls in?

---

## 7. Before you leave

- [ ] **Ask directly: "Would you be willing to test this with me when it's ready?"**
      This answers Q11, the thing that turns the plan into a product.
- [ ] Get a phone number and a WhatsApp
- [ ] Ask whether they know other school owners — distribution, later
- [ ] Ask what the Direction des Transports has been pushing them on lately.
      Regulatory pressure is a sales argument.

---

## After the visit — same day, while it's fresh

- [ ] Dump every photo into one dated folder
- [ ] Fill in the answers in [`05-OPEN-QUESTIONS.md`](05-OPEN-QUESTIONS.md)
- [ ] Write down the **three things that surprised you**. Those are the ones that will change
      the design, and they are the first things you forget.

---

## Separately, when you have internet: the decree

Not part of the visit, but the other Phase 0 blocker (Q2).

- [ ] Download *Journal Officiel* no. 40 of 2024 from `joradp.dz`
      — `https://www.joradp.dz/FTP/JO-ARABE/2024/A2024040.pdf`
      — `https://www.joradp.dz/FTP/JO-FRANCAIS/2024/F2024040.pdf`
- [ ] Find the *arrêté* on the driving-instruction training programme
- [ ] Extract: **practical hours per category**, the **full theory module list with hours**,
      and whether **créneau** has its own requirement

Send me the PDF or the relevant pages and I will turn it into seed data.
