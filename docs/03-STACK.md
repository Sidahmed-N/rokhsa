# Stack Decisions — Tariqi

**Status:** Recommendation, awaiting PM sign-off
**Date:** 2026-10-17

The constraints that drive every choice below: **offline forever**, **weak Windows PCs**,
**small installer**, **correct Arabic including in print**, **sold as a licensed product
so piracy resistance matters**, **one developer**, and **an eventual path to a student
mobile app and optional cloud sync**.

---

## Decision 1 — Application shell: **Tauri 2**

### Recommended
**Tauri 2** (Rust core, system WebView2 on Windows) with a React frontend.

### Why

| Factor | Tauri 2 | Electron | .NET 8 + WPF | Flutter desktop |
|---|---|---|---|---|
| Installer size | **~10–20 MB** | 80–150 MB | ~60–150 MB | ~40–60 MB |
| RAM on a weak PC | **Low** (shared WebView) | High (bundled Chromium) | Low | Low |
| Arabic text shaping | **Excellent** (WebView) | Excellent | Good (WPF `FlowDirection`) | Good |
| RTL layout effort | **Low** (CSS logical props) | Low | Medium | Medium |
| Print / HTML layouts | **Excellent** | Excellent | Needs separate reporting stack | Weak |
| Tamper resistance | **Good** (compiled Rust core) | Poor (`.asar` trivially unpacked) | Medium (IL, decompilable) | Good |
| Path to mobile later | **Good** (Tauri 2 mobile, or reuse React in a web app) | Poor | Poor | **Excellent** |
| Ecosystem for business UI | Good (React) | **Excellent** | Good | Medium |
| My implementation speed | Good | **Best** | Medium | Medium |

**The deciding factors:**

1. **Installer size genuinely matters here.** Distribution to Algerian schools will often be
   by USB stick, WhatsApp, or a slow ADSL link. A 15 MB installer is a different product
   from a 130 MB one, commercially.
2. **Tamper resistance is a commercial requirement, not a nicety.** Software piracy is the
   main threat to this business model. Electron's bundle can be unpacked and the licence
   check deleted by anyone with ten minutes and a search engine. A compiled Rust binary
   raises that bar substantially — not insurmountably, but meaningfully.
3. **HTML-based printing is the only sane route to correct Arabic output** (see Decision 4),
   which rules .NET's and Flutter's reporting stories down relative to a WebView.
4. **Low RAM on 4 GB HDD machines** is a real constraint that Electron handles worst.

### Cost of this choice, stated plainly
- I will write Rust. More of the work lands in a stricter language, so each feature takes
  somewhat longer than the Electron equivalent — but the compiler catches a class of bug
  that would otherwise reach a school's cash drawer.
- Tauri depends on **WebView2**, which is preinstalled on Windows 11 and current Windows 10
  but may be missing on a neglected machine. Mitigation: ship the offline WebView2
  bootstrapper inside the installer. Non-negotiable for this market.
- **Windows 7/8 are not supported.** Modern WebView2 and modern Chromium do not target them.
  If pilot schools turn out to run Windows 7, that forces a reassessment toward .NET.
  **PM to verify what the target schools actually run.**

### Rejected
- **Electron** — faster for me, worse for the product on size, RAM, and piracy. Reconsider only
  if delivery speed becomes the overriding concern.
- **.NET 8 + WPF** — genuinely strong for Windows business apps and excellent at printing,
  but Windows-only forever, no path to the student mobile app, and a weaker story for the
  bilingual web-style UI. The serious fallback if Windows 7 support proves mandatory.
- **Flutter** — best mobile path, but desktop printing and dense data-table UI are its weak
  spots, and those are most of this app.

---

## Decision 2 — Database: **SQLite**

Not really a decision; it is the only sensible local-first store. Single file, transactional,
zero administration, trivially backed up by copying a file.

Configuration that matters:
- **WAL mode** for concurrent reads during writes.
- `synchronous = FULL` on money-writing transactions. This app will be running on machines
  that lose power; durability beats throughput.
- `foreign_keys = ON` (SQLite leaves it off by default — a classic silent-corruption source).
- Periodic `VACUUM` and `PRAGMA integrity_check` on a schedule, surfaced in a health screen.

### Access layer: `sqlx` with compile-time checked queries
Queries are verified against the real schema at **build** time, so a typo in a column name
fails the build rather than a school's receipt printing. Migrations via `sqlx migrate`,
forward-only, embedded in the binary.

**Rejected:** `tauri-plugin-sql` (raw SQL strings from the frontend — loses type safety and
puts business logic in the untrusted layer, contradicting the architecture); Diesel (heavier
for this scale); SeaORM (pleasant, but I prefer explicit SQL where money is involved).

### Encryption at rest
Deferred, with a hook left for it. The data is sensitive under law 18-07, but SQLCipher
complicates backup and recovery, and a lost key in a school with no IT support means
total data loss — a worse outcome than the threat it prevents. **Backups are encrypted;
the live database is not, in v1.** Revisit if legal review requires otherwise.

---

## Decision 3 — Frontend: **React + TypeScript + Vite + Tailwind + shadcn/ui**

- **React + TypeScript (strict)** — the largest ecosystem for exactly what this app is:
  dense forms, data tables, a calendar. Also the direct path to reusing the UI in a web
  dashboard later.
- **Vite** — fast builds, standard with Tauri.
- **Tailwind CSS** — critically, its **logical properties** (`ms-*`, `pe-*`, `start-*`) make
  RTL a `dir="rtl"` attribute flip rather than a parallel stylesheet. This is the single
  biggest reason not to hand-write CSS here.
- **shadcn/ui on Radix** — accessible, unstyled primitives I own the source of, so RTL and
  Arabic typography fixes are edits rather than upstream bug reports.
- **TanStack Table** for the data grids, **TanStack Query** for command caching and
  invalidation, **React Hook Form + Zod** for the many validated forms.
- **Calendar**: a custom day/week grid rather than a heavy third-party scheduler. The
  conflict rules and the instructor/vehicle axes are specific enough that a library fights
  us more than it helps.

### i18n: `i18next` + `react-i18next`
- `fr.json` and `ar.json`, namespaced per feature.
- Language switch sets `<html dir>` and `lang`.
- **CI fails on a key present in `fr` and missing in `ar`.** This is how "both languages at
  launch" stays true instead of decaying into a half-translated Arabic UI.
- Arabic UI font: a proper Arabic face (Noto Naskh Arabic or IBM Plex Sans Arabic), bundled,
  not a system fallback. Latin and Arabic numeral display is a per-school setting.

---

## Decision 4 — Printing: **HTML layouts rendered by the WebView**

### The reasoning, because this one is counter-intuitive
The obvious instinct is to generate PDFs in Rust. **Do not.** Rust PDF crates generally do
not perform Arabic text shaping — they lay out glyphs left to right in isolated forms, so
Arabic comes out disconnected and reversed. Getting it right means integrating HarfBuzz and
a bidi algorithm by hand.

The WebView already contains a complete, correct text-shaping and bidirectional-layout
engine. So: build print documents as **HTML + print CSS**, render them in a hidden window,
and print through WebView2. Correct Arabic for free, and layouts become editable templates
rather than compiled code — which matters because the official forms *will* change.

- Page geometry in print CSS (`@page`, A4, margins), one stylesheet per document type.
- School letterhead and logo injected from settings.
- **Thermal receipt printers (ESC/POS)** are common at Algerian counters. Deferred to a
  later phase via a Rust serial/USB path, with A4 and A5 receipts in v1.
  **PM to confirm what printers schools actually have.**

---

## Decision 5 — Rust↔TypeScript boundary: **Tauri commands + `tauri-specta`**

Every operation is a typed Tauri command. `tauri-specta` generates TypeScript types and
client functions from the Rust signatures, so a changed Rust struct becomes a TypeScript
compile error rather than a runtime surprise.

Layering inside `src-tauri`:
```
commands/   thin — deserialize, authorize, call domain, serialize
domain/     pure business rules, no DB, no IO — exhaustively unit-tested
db/         queries and transactions
license/    offline licence verification
```

`domain/` being pure and IO-free is what makes pricing, balance, hour-completion, exam
eligibility, and cash reconciliation cheap to test exhaustively. Those five functions are
where a bug costs a school real money.

---

## Decision 6 — Licensing: Ed25519-signed offline licence file

**The business model is a one-time perpetual sale — no subscription, no renewal.** That shapes
this design; see `08-PACKAGING.md` for the commercial reasoning.

The licence file carries, signed with a private key the PM holds and verified in Rust against
an embedded public key:

| Field | Purpose |
|---|---|
| School name | Identification, printed in About |
| `edition` + `entitlements` | `basic` / `pro`, `max_instructors`, `features: [...]` — **never a bare tier name**, so features can move between editions by licence rather than by code |
| Machine fingerprint hash | Stops one purchase becoming a whole wilaya's copies |
| **`max_major_version`** | The version this licence covers, e.g. `1.x` |
| **`expires_at` (nullable)** | **Empty for every licence we issue.** See below. |

### Three deliberate choices

**Licences never expire in practice, but the field exists.** `expires_at` is written as empty
on every licence issued, and the product is a one-time perpetual sale. The field is in the
format — and read by the verification code — purely so that the PM's own stated option of
moving to a yearly model for *future* customers later costs nothing. Filling in a field beats
re-licensing every existing customer.

When `expires_at` is empty, which is always today, there is no expiry check at all. If one is
ever populated, it warns and degrades — it never locks a school out mid-day.

**A version ceiling from the first release.** `max_major_version` is how v2 gets sold later:
the customer keeps v1 working forever, and v2 is a new purchase. Honest, visible, no
subscription. **It must exist in the licence format from v1.0** even though v2 is years away —
retrofitting it would mean asking every existing customer to re-license.

**Reissue is easy and free.** A school that changes or reinstalls its PC gets a new licence,
no argument. Refusing is how you convert a paying customer into a pirate on the worst day of
their week.

### Issuance is offline and manual — automate it early
The owner reads a machine ID out of the app and sends it by WhatsApp; the PM returns a `.lic`
file. Workable by hand for the first dozen schools. **Write the key-generator script before
launch**, because doing this manually at scale becomes a full-time job.

### Nothing in the licence path blocks work
An unlicensed, mis-bound or tampered state may warn, may disable Pro features, and may refuse
to start a *new* school — it must **never** block taking a payment, printing a receipt, or
making a backup. Tamper handling is a warning, never self-destruction. Never put a school's
data at risk to protect a licence.

**Stated honestly:** a competent reverse engineer defeats this, and in a perpetual model there
is no service to withhold afterwards — a cracked copy is a permanently lost sale. The real
defence is therefore **price**, not DRM: cheap enough that paying is obviously easier than
hunting for a crack. That argument lives in `08-PACKAGING.md §3`, and it matters more than
anything in this section.

---

## Decision 7 — Updates and distribution

- **NSIS installer** (Tauri default on Windows) with the **offline WebView2 bootstrapper**
  bundled. Must install with no internet.
- **Tauri updater** for schools that do have connectivity, plus a downloadable full installer
  for everyone else — in practice, the USB-stick path.
- **Code signing certificate strongly recommended.** Without it, Windows SmartScreen shows a
  scary warning on every install, which for a paid product aimed at non-technical buyers is a
  serious conversion problem. This is a real annual cost; **PM decision.**

---

## Decision 8 — Testing

- **Rust unit tests on `domain/`** — mandatory, highest value in the project.
- `sqlx` compile-time query checking — catches schema drift at build.
- **Vitest** for frontend logic (formatters, validators, i18n completeness).
- **WebdriverIO + `tauri-driver`** for a small end-to-end suite over the critical paths:
  register a candidate, take a payment, record a result, close the cash session.
- A **power-cut test**: kill the process mid-transaction and assert the database is
  consistent and no committed receipt is lost. Run this before every release.

---

## Summary

| Layer | Choice |
|---|---|
| Shell | Tauri 2 (Rust) |
| UI | React 19 + TypeScript strict + Vite |
| Styling | Tailwind CSS (logical properties) + shadcn/ui |
| i18n | i18next + react-i18next, CI-enforced parity |
| Data | SQLite (WAL, FK on) via `sqlx`, forward-only migrations |
| Boundary | Tauri commands + `tauri-specta` generated types |
| Print | HTML + print CSS via WebView2 |
| Licence | Ed25519-signed offline file, perpetual, machine-bound, version-ceilinged |
| Package | NSIS + offline WebView2 bootstrapper |
| Test | Rust unit tests, Vitest, WebdriverIO, power-cut test |

### What would change this recommendation
1. **Pilot schools run Windows 7** → switch to .NET 8 + WPF.
2. **Speed to pilot overrides everything** → switch to Electron, accept size and piracy cost.
3. **Student mobile app becomes a v1 requirement** → reconsider Flutter for one codebase.
