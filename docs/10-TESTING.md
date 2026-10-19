# Testing the app yourself

**Date:** 2026-10-19

The app already has demo data in it: 8 candidates, a few enrolments, some
payments and a month of expenses. Everything below works on that.

---

## Running it

Double-click the executable. Nothing else needs to be running.

```
C:\Users\SuGa\.tariqi-build\release\tariqi.exe
```

Your data lives at `%APPDATA%\dz.tariqi.desktop\` — the database and a
`backups` folder. Deleting that folder resets the app to empty.

To put the demo data back after wiping it:

```bash
cd src-tauri && cargo run --bin seed
```

---

## What to try, roughly in order of what matters

### 1. Switch to Arabic and stay there

The language buttons are top right. **Then use the whole app in Arabic.**
Everything should mirror — the menu moves to the right, tables flip, labels and
values swap sides — while phone numbers, identity numbers, dates and money stay
readable left-to-right.

This is where I would look hardest. Every RTL bug found so far was found by
looking, not by reading code, and you will spot wrong-sounding Arabic that I
cannot.

### 2. Register a candidate — and time it

Candidats → **+ Nouveau candidat**. Put a watch on it. The target is under
three minutes including the paperwork, because if it is slower than the
notebook the notebook wins.

Things worth trying deliberately:
- Enter **only a surname** and save. It should work — a candidate who has not
  brought their papers must still be registrable.
- Enter the **same name twice**. You should get a warning, not a block.
- Enter a **short NIN**. It should refuse, in your language.
- Type an Arabic name with vowels, then search for it without them.

### 3. The money loop

Open a candidate → **+ Nouvelle inscription** → set a price → **+ Ajouter des
heures** → **Encaisser**.

Taking a payment jumps straight to the receipt and opens the print dialog.
You can print to PDF to see the result without wasting paper.

Then check:
- The balance on the candidate screen
- Whether that same figure appears on **Créances**
- **Annuler** on a payment — it should add a correcting line, not delete
  anything, and the balance should go back

### 4. The two screens that sell it

**Créances** — who owes you money, biggest first, with phone numbers and days
since their last payment. This is the screen to show a school owner first.

**Caisse** — money in and out for a period, with a breakdown. Add an expense
and watch the total move.

### 5. Settings

Change the school name and address, save, then print a receipt again — your
name should be on it.

Try **Sauvegarder sur clé USB** with a real USB stick. **I have not tested this
myself**, so it is the most likely thing to misbehave.

---

## What I already know is missing

Don't spend time reporting these:

- **No installer yet.** It is a bare `.exe`; nothing is registered with Windows,
  there is no Start menu entry, and no licence check.
- **No candidate statement printout.** Only the payment receipt prints.
- **No first-run wizard**, no printable user manual, no diagnostics report.
- **No scheduling calendar, no exams, no fleet.** Deliberately cut from the
  first release.
- **No login or user accounts.** Single user by design for now.
- **The app window may look plain.** Styling has had no pass at all yet.
- **Arabic wording is my best effort**, not a native speaker's. Corrections very
  welcome — they are all in `src/locales/ar.json`.

---

## What I most want to hear back

1. **Where did you hesitate?** Anywhere you had to stop and think is a design
   bug, not a you problem.
2. **Is anything wrong in Arabic** — wording, direction, or a number reading
   backwards?
3. **Does the money match what a school would expect?** Particularly extra
   hours and how a correction should behave.
4. **What would a school owner ask for in the first five minutes** that is not
   there?
