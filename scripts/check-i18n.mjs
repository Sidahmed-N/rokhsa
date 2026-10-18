/**
 * i18n parity check.
 *
 * CLAUDE.md: "A key missing from `ar` fails CI. We do not ship a half-translated
 * screen." This script is that rule. Run via `npm run check:i18n`.
 *
 * It compares the key sets of every locale against French (the reference) and
 * fails on any key present in one and missing from another, or present but empty.
 */

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

const here = path.dirname(fileURLToPath(import.meta.url));
const localesDir = path.join(here, "..", "src", "locales");

const REFERENCE = "fr";
const LOCALES = ["fr", "ar"];

/** Flatten nested translation objects into dotted key paths. */
function flatten(value, prefix = "", out = new Map()) {
  for (const [key, child] of Object.entries(value)) {
    const full = prefix ? `${prefix}.${key}` : key;
    if (child !== null && typeof child === "object" && !Array.isArray(child)) {
      flatten(child, full, out);
    } else {
      out.set(full, child);
    }
  }
  return out;
}

function load(locale) {
  const file = path.join(localesDir, `${locale}.json`);
  try {
    return flatten(JSON.parse(readFileSync(file, "utf8")));
  } catch (error) {
    console.error(`Could not read ${file}: ${error.message}`);
    process.exit(1);
  }
}

const tables = new Map(LOCALES.map((locale) => [locale, load(locale)]));
const reference = tables.get(REFERENCE);

let failures = 0;

function fail(message) {
  console.error(`  ${message}`);
  failures += 1;
}

for (const locale of LOCALES) {
  const table = tables.get(locale);
  const problems = [];

  if (locale !== REFERENCE) {
    for (const key of reference.keys()) {
      if (!table.has(key)) problems.push(`missing: ${key}`);
    }
    for (const key of table.keys()) {
      if (!reference.has(key)) problems.push(`extra (not in ${REFERENCE}): ${key}`);
    }
  }

  for (const [key, value] of table) {
    if (typeof value !== "string") {
      problems.push(`not a string: ${key}`);
    } else if (value.trim() === "") {
      problems.push(`empty: ${key}`);
    }
  }

  if (problems.length > 0) {
    console.error(`\n[${locale}] ${problems.length} problem(s):`);
    for (const problem of problems) fail(problem);
  }
}

if (failures > 0) {
  console.error(
    `\ni18n check FAILED with ${failures} problem(s).\n` +
      `Every screen ships complete in French and Arabic — see CLAUDE.md.\n`,
  );
  process.exit(1);
}

console.log(
  `i18n check passed: ${reference.size} keys, complete in ${LOCALES.join(" and ")}.`,
);
