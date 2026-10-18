# Packaging and Pricing — Tariqi

**Status:** Recommendation for PM decision
**Date:** 2026-10-17
**Model fixed by PM: one-time perpetual sale. No subscription, no annual renewal.**

The PM's position is that one-time purchase is what works in this market. That is accepted as
a given — he knows the market. This document works out what follows from it, because quite a
lot does.

---

## 1. What a one-time sale actually means

You are paid **once per school, ever.** Two consequences dominate everything else:

### Every support call is a permanent loss
In a subscription model, support is funded by next year's payment. Here it is funded by a
payment you already spent. A customer who calls four times has cost you more than they paid.

**So the product must be self-sufficient. This is an engineering requirement, not a nicety:**

- A **first-run wizard** that gets a school configured without you on the phone
- A **printable user manual in French and Arabic**, shipped with the installer
- **Self-service backup and restore** — the single most common support request in this
  category of software, and it must never require you
- **Error messages that say what to do**, not what went wrong
- In-app help on every screen that isn't obvious
- A **built-in diagnostics report** the owner can send you in one click when he does call

Budget real effort for this. It is cheaper than the support calls it prevents.

### Piracy has no backstop
With a subscription, a cracked copy stops being useful when it stops receiving service. With a
perpetual licence there is nothing to withhold later. **A cracked copy is a permanently lost
sale and a free competitor to you.** One patched copy crosses a wilaya by WhatsApp in a week.

The defence is **not stronger DRM.** It is §3.

---

## 2. How you actually make money in this model

Perpetual-licence software has always made money the same ways. None of these is a subscription.

| Route | How it works | Strength |
|---|---|---|
| **Volume at a low price** | Reach as many of Algeria's auto-écoles as possible | 🔴 **The main engine.** See §3. |
| **Paid major versions** | v1 now; v2 in two or three years is a **new product sold again** to the same schools | 🔴 Strong, culturally normal — *"la nouvelle version"* |
| **Paid add-on modules** | Code-de-la-route module, student app — each sold once, each a new sale to an existing customer | 🟠 Good, and no ongoing cost |
| **Pro edition** | A higher one-time price for multi-staff schools; upgrading from Basic is a **top-up payment** | 🟠 Clean second sale when a school grows |
| **Setup and migration services** | Install, configure, type their notebooks into the system, train the staff | 🟠 **Impossible to pirate.** Could be a serious share of revenue. |
| **Custom form templates** | A wilaya's particular paperwork, done for them | 🟡 Small but sticky |
| **Reseller network per wilaya** | They take a cut, they bring local trust and legwork | 🟠 For one-time volume, distribution is the whole game |

### Drop cloud sync, or change it completely
**A one-time payment for a service with perpetual hosting cost loses money on every customer,
forever.** It gets worse the more successful you are. That is not a business.

Replace it with the same benefit at zero cost to you:
- **Backup to the school's own Google Drive / OneDrive folder** — their storage, their account
- **Backup to USB**, already in the plan
- **Backup to another PC on the school's own network**

The owner gets his data protected off that one PC, which is what he actually wants. You carry
no server bill. If cloud sync is ever revived, it must be the one thing sold on a recurring
basis — and the PM has ruled that out, so plan without it.

---

## 3. Price is your anti-piracy strategy

This is the most important decision in the whole commercial plan.

**Cracking software is effort and risk** — finding it, trusting it, malware, no support, no
updates. An owner weighs that against the price. So:

- **Price it low enough that paying is obviously easier than pirating.** If it costs about what
  a school earns from a single candidate, most owners will simply pay. Price it at ten
  candidates and somebody will crack it and share it out of principle.
- **Low price + high volume is the piracy-resistant strategy for perpetual licences.** It is
  also the right strategy given no competitor exists: land grab beats margin.
- A high price in this model buys you a cracked market and a reputation for being expensive.

**Concrete shape to consider:**

| Edition | Who | Price shape |
|---|---|---|
| **Free** | Trial / tiny schools | Capped at ~10–15 active candidates. Pure distribution — costs you nothing, makes owners advocates. |
| **Basic** | One person, one vehicle | Deliberately cheap. The volume product. |
| **Pro** | Schools with employed instructors | Roughly 2–2.5× Basic, as a one-time price |
| **Upgrade Basic → Pro** | A school that grew | The difference, as a top-up |
| **Add-ons** | Anyone | Each sold once |
| **Setup / migration / training** | Anyone | Per-job, priced on effort |

The PM sets the DZD figures. My only strong view is **keep Basic cheap** — in this model, the
number of schools reached matters more than the margin on each.

---

## 4. Tiering: still gate on staff, not features

Unchanged from the earlier analysis, and it fits a one-time model even better: upgrading is a
single top-up payment at the moment a school hires its first instructor.

**Basic = one person does everything. Pro = you have employees.**

It works because **a solo owner physically cannot use the Pro features** — no instructors to
compare, no employee cash handling to audit, no payroll to run. He never sees a locked button
for something he needs today, which is the only real test of a tier boundary.

### In every edition — never paywalled

- Candidates, dossiers, document checklist with expiry, attachments
- Full French **and** Arabic with RTL — language is the product, not a feature
- Enrolments, packages, pricing
- Scheduling, lessons, hours tracked vs required
- **Payments, numbered receipts, balances, receivables** — the most valuable thing in the product
- Daily cash session
- Exam sessions, eligibility, results, retakes
- All printed documents
- Expenses, vehicles, document-expiry alerts
- **Backup and restore, including to USB and to the school's own cloud folder**
- Alerts dashboard
- **The user manual and the first-run wizard**

> **Never paywall backup or restore.** In a one-time model you cannot even recover the
> relationship afterwards. A Basic school that loses a year of records ends your reputation in
> that wilaya permanently.

### Pro only

Multiple instructors · multiple logins with roles · per-instructor pass rates and productivity ·
cash accountability by person and audit-log browsing · instructor payroll · vehicle cost-per-km
and profitability comparison · full report suite with Excel export · multi-branch (post-v1).

### Never cap candidates on a paid edition
Hitting a cap mid-season locks an owner out of his own business, and it teaches him to stop
recording candidates — which corrupts the exact data the product exists to hold. Caps belong
on the free edition only.

---

## 5. Licence design for perpetual sale

Changes from the earlier subscription-shaped design in `03-STACK.md`:

- **No expiry date.** A perpetual licence does not expire. Remove expiry checks and the grace
  period entirely — there is nothing to be graceful about.
- **Keep the machine binding.** It is what stops one purchase becoming a whole wilaya's copies.
  Make reissue for a new or reinstalled PC **easy and free** — a lost machine is already a bad
  day and refusing to reissue converts a customer into a pirate.
- **Add a version ceiling.** The licence states the major version it covers, e.g. `1.x`. This is
  the mechanism for selling v2 later: honest, visible, no subscription, and the customer keeps
  v1 working forever. **This single field is what makes paid major versions possible, so it must
  be in the licence format from the first release**, even though v2 is years away.
- **Entitlements, not a tier name** — `edition: "basic"`, `max_instructors: 1`,
  `features: [...]`. Moving a feature between editions later is then a licence change, never a
  code change.
- **Offline issuance.** The owner reads a machine ID out of the app and sends it by WhatsApp;
  you send back a `.lic` file. Fine by hand at low volume — **write the key-generator script
  early**, because doing this manually at a few hundred schools will become your whole week.
- **Nothing in the licence check ever blocks taking a payment, printing a receipt, or making a
  backup.** Degrade, never detonate.

---

## 6. What this model needs you to find out

- 🔴 **How many auto-écoles are there in Algeria?** In a one-time model, total market size *is*
  your total lifetime revenue. This number decides whether the price should be low-and-wide or
  higher-and-narrower, and I don't know it. The Ministry or the CNRC may publish it.
- 🔴 **What would a school owner pay, once, for this?** Ask it plainly during the visit, late in
  the conversation.
- 🟠 How many schools would pay separately for setup and having their notebooks typed in?
- 🟠 Is a per-wilaya reseller network realistic for you, and at what cut?
- 🟠 Would owners pay again for a v2 in two or three years? Ask how they've treated other
  software they've bought.
- 🟡 Does Q15 (`05-OPEN-QUESTIONS.md`) make Pro worth having at all? If nearly every school is
  solo, a staff-gated Pro sells to almost nobody, and you should put that effort into add-on
  modules instead.
