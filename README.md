# Rokhsa — Auto-École Management System for Algeria

> Rokhsa (رخصة, "licence"). Offline-first Windows desktop software for Algerian driving schools
> (*auto-écoles*), sold as a licensed product to independent schools.

**Status:** Planning. No code yet.

---

## The short version

Algerian auto-écoles run on paper: cardboard candidate folders, a wall planner for lessons,
and notebooks for cash collected in instalments. No owner can answer "how much am I owed
right now?" without adding up pages. No software exists for this market — the available
tools are French and assume French paperwork, card payment, and constant internet.

Rokhsa runs entirely offline on the school's own PC, in French **and** Arabic, and produces
the paperwork the Direction des Transports expects.

## Shape of the product

| | |
|---|---|
| **Platform** | Windows desktop, offline-first, local SQLite database |
| **Distribution** | Licensed per school, annual key, no central server in v1 |
| **Languages** | French and Arabic, both complete at launch, full RTL |
| **Primary user** | The school secretary at the front desk |
| **Stack** | Tauri 2 (Rust) + React + TypeScript + SQLite |

## Documents

Read in order:

| Doc | What it covers |
|---|---|
| [CLAUDE.md](CLAUDE.md) | Engineering contract: hard rules, conventions, domain glossary |
| [docs/01-PRD.md](docs/01-PRD.md) | The problem, users, v1 scope, success criteria, commercial model |
| [docs/02-DOMAIN-MODEL.md](docs/02-DOMAIN-MODEL.md) | Entities, invariants, core workflows |
| [docs/03-STACK.md](docs/03-STACK.md) | Stack decisions with alternatives and trade-offs |
| **[docs/09-FIRST-RELEASE.md](docs/09-FIRST-RELEASE.md)** | 🔴 **The active plan** — the smallest sellable release |
| [docs/04-ROADMAP.md](docs/04-ROADMAP.md) | Long-term phases, after the first release pays for itself |
| [docs/05-OPEN-QUESTIONS.md](docs/05-OPEN-QUESTIONS.md) | What we must find out, and what it blocks |
| [docs/06-FIELD-VISIT.md](docs/06-FIELD-VISIT.md) | Checklist for a school visit — open it on your phone |
| [docs/07-REGULATORY.md](docs/07-REGULATORY.md) | Why regulatory tracking is out of scope |
| [docs/08-PACKAGING.md](docs/08-PACKAGING.md) | One-time pricing, editions, and anti-piracy |
| [docs/10-TESTING.md](docs/10-TESTING.md) | How to run and test the app yourself |
| **[docs/11-PARKED.md](docs/11-PARKED.md)** | ⚠️ Known gaps, parked on purpose — audit trail, logins, licence |
| [tools/survey-pc.ps1](tools/survey-pc.ps1) | Read-only PC survey script to run on school machines |

## What the app is, and is not

**It records what the school does. It does not enforce the law.** Candidates, hours, money in
and out. Categories and their hour targets are plain settings on a price list — the owner types
in what he charges and how many hours he gives, and the app shows progress against that.
Nothing is checked against any regulation, and no action is ever refused because of one.

Regulatory and compliance tracking is **explicitly out of scope** — see
[docs/07-REGULATORY.md](docs/07-REGULATORY.md).

## The three things that decide this project

1. **What Windows version the schools run.** The only genuine build blocker — if a target
   school is on Windows 7, the whole stack decision changes. Answered mechanically by
   [tools/survey-pc.ps1](tools/survey-pc.ps1) ([Q3](docs/05-OPEN-QUESTIONS.md)).
2. **Whether a school will pay, and how much.** The goal is revenue soon, so this is the
   number that matters most and it is cheap to get — show the receivables screen to one owner
   and ask him plainly ([Q9, Q11](docs/05-OPEN-QUESTIONS.md)).
3. **Whether the owner is faster in the app than in his notebook.** If not, the notebook wins
   and nothing else matters. Time both during the school visit.

## Design principles

- **Nothing regulatory is hardcoded.** Hours, categories, fees, and document checklists are
  editable data, because the published figures could not be verified and a new traffic law is
  in discussion.
- **Money is integer centimes, append-only, with gapless receipt numbers.** Corrections are
  reversing entries, never edits.
- **Balances and progress are always derived, never stored.** Stored counters drift; drift in
  a money figure destroys trust in the whole app.
- **The Rust layer owns the business rules.** The frontend is a view. It never computes a
  price, a balance, or an eligibility decision.
- **It must survive a power cut mid-save.** That is a design requirement here, not an edge case.
