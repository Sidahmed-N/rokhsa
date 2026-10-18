/**
 * RTL safety check.
 *
 * CLAUDE.md: "Use CSS logical properties only: ms-/me-/ps-/pe-/start-/end-.
 * ml-, pr-, left-, right- are banned in app code and enforced by lint."
 *
 * Physical direction utilities do not mirror when the document flips to RTL, so
 * a single `mr-4` is an Arabic layout bug that nobody notices in French. This
 * script catches them. Run via `npm run check:rtl`.
 *
 * Escape hatch: put `rtl-allow` in a comment on the same line when a physical
 * property is genuinely correct (something that must not mirror, such as a
 * logo lockup or a chart axis).
 */

import { readdirSync, readFileSync, statSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

const here = path.dirname(fileURLToPath(import.meta.url));
const srcDir = path.join(here, "..", "src");

const EXTENSIONS = new Set([".ts", ".tsx"]);

/** Tailwind class token characters, so we can require a clean boundary. */
const TOKEN = String.raw`[\w.[\]()/%-]`;
const BOUNDARY_BEFORE = String.raw`(?<![\w-])`;

const RULES = [
  {
    name: "margin-left / margin-right",
    suggestion: "use ms-* / me-*",
    pattern: new RegExp(`${BOUNDARY_BEFORE}m[lr]-${TOKEN}+`, "g"),
  },
  {
    name: "padding-left / padding-right",
    suggestion: "use ps-* / pe-*",
    pattern: new RegExp(`${BOUNDARY_BEFORE}p[lr]-${TOKEN}+`, "g"),
  },
  {
    name: "left-* / right-*",
    suggestion: "use start-* / end-*",
    pattern: new RegExp(`${BOUNDARY_BEFORE}(?:left|right)-${TOKEN}+`, "g"),
  },
  {
    name: "text-left / text-right",
    suggestion: "use text-start / text-end",
    pattern: new RegExp(`${BOUNDARY_BEFORE}text-(?:left|right)(?![\\w-])`, "g"),
  },
  {
    name: "border-l / border-r",
    suggestion: "use border-s-* / border-e-*",
    pattern: new RegExp(`${BOUNDARY_BEFORE}border-[lr](?:-${TOKEN}+)?(?![\\w-])`, "g"),
  },
  {
    name: "rounded corner on a physical side",
    suggestion: "use rounded-s-* / rounded-e-* / rounded-ss-* etc.",
    pattern: new RegExp(
      `${BOUNDARY_BEFORE}rounded-(?:[lr]|tl|tr|bl|br)(?:-${TOKEN}+)?(?![\\w-])`,
      "g",
    ),
  },
];

function walk(dir, files = []) {
  for (const entry of readdirSync(dir)) {
    const full = path.join(dir, entry);
    if (statSync(full).isDirectory()) {
      walk(full, files);
    } else if (EXTENSIONS.has(path.extname(entry))) {
      files.push(full);
    }
  }
  return files;
}

/**
 * Only scan string literals.
 *
 * Class names always live in quotes; prose in comments does not. Scanning the
 * raw line instead would flag English like "right-to-left" in a comment, which
 * is exactly the false positive this avoids.
 */
const STRING_LITERAL = /(['"`])((?:\\.|(?!\1)[^\\])*)\1/g;

function stringLiteralsIn(line) {
  const found = [];
  STRING_LITERAL.lastIndex = 0;
  let match;
  while ((match = STRING_LITERAL.exec(line)) !== null) {
    found.push(match[2]);
  }
  return found;
}

/**
 * Rules checked against the raw line rather than string literals.
 *
 * Interpolating a formatted number straight into JSX skips the bidi isolation
 * that <Num> / <Money> apply, so the value renders backwards in Arabic and
 * correctly in French — a bug nobody sees until a school prints a receipt with
 * a reversed national ID on it. This happened in the first draft of the
 * receipt, hence the guard.
 */
const JSX_RULES = [
  {
    name: "money formatted directly into JSX",
    suggestion: "use <Money centimes={...} /> from @/components/Num",
    pattern: /\{\s*format(?:DZD|Dinars)\s*\(/g,
  },
];

let failures = 0;

for (const file of walk(srcDir)) {
  const lines = readFileSync(file, "utf8").split(/\r?\n/);
  const relative = path.relative(path.join(here, ".."), file).replace(/\\/g, "/");

  lines.forEach((line, index) => {
    if (line.includes("rtl-allow")) return;

    for (const rule of JSX_RULES) {
      rule.pattern.lastIndex = 0;
      const matches = line.match(rule.pattern);
      if (matches) {
        for (const match of matches) {
          console.error(
            `${relative}:${index + 1}  "${match.trim()}"  — ${rule.name}; ${rule.suggestion}`,
          );
          failures += 1;
        }
      }
    }

    for (const literal of stringLiteralsIn(line)) {
      for (const rule of RULES) {
        rule.pattern.lastIndex = 0;
        const matches = literal.match(rule.pattern);
        if (matches) {
          for (const match of matches) {
            console.error(
              `${relative}:${index + 1}  "${match}"  — ${rule.name}; ${rule.suggestion}`,
            );
            failures += 1;
          }
        }
      }
    }
  });
}

if (failures > 0) {
  console.error(
    `\nRTL check FAILED with ${failures} physical direction utility/utilities.\n` +
      `These do not mirror for Arabic. Use logical properties — see CLAUDE.md.\n` +
      `If a case genuinely must not mirror, add a "rtl-allow" comment on that line.\n`,
  );
  process.exit(1);
}

console.log("RTL check passed: no physical direction utilities in src/.");
