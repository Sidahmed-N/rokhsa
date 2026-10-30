// Tests for parseDZD — the one place a typed amount becomes money.
// Run with: node scripts/test-money.mjs   (Node 22.6+ strips the TS types).
// A dot read as a decimal point once stored a 12 000 DA payment as 12 DA,
// so every way an amount is written here is pinned down below.
import { parseDZD } from "../src/lib/money.ts";

const cases = [
  // whole dinars, however they are grouped
  ["12000", 1_200_000],
  ["12 000", 1_200_000],
  ["12.000", 1_200_000],
  ["12,000", 1_200_000],
  ["35.000", 3_500_000],
  ["1.500.000", 150_000_000],
  ["1,500,000", 150_000_000],
  ["1 500 000", 150_000_000],
  ["500", 50_000],
  ["0", 0],
  // with centimes
  ["12500,50", 1_250_050],
  ["12500.5", 1_250_050],
  ["12500,05", 1_250_005],
  ["12.500,50", 1_250_050],
  ["12,500.50", 1_250_050],
  ["1.500.000,25", 150_000_025],
  ["1,5", 150],
  // as typed with a currency or on an Arabic keyboard
  ["12 000 DA", 1_200_000],
  ["12000 دج", 1_200_000],
  ["١٢٠٠٠", 1_200_000],
  ["١٢٫٥٠", 1_250],
  ["-3.000", -300_000],
  // ambiguous or broken: refused, never guessed
  ["", null],
  ["-", null],
  ["abc", null],
  ["12.5000", null],
  ["1.50.000", null],
  ["12,345,6", null],
  ["1.234.56", null],
  ["12.", null],
  [".50", null],
  ["12,500,50.0.0", null],
  ["1.000,000,00", null],
  ["12a000", null],
];

let failed = 0;
for (const [input, expected] of cases) {
  const got = parseDZD(input);
  if (got !== expected) {
    failed++;
    console.error(`FAIL  ${JSON.stringify(input)} -> ${got}, expected ${expected}`);
  }
}
console.log(`${cases.length - failed}/${cases.length} parseDZD cases passed`);
process.exitCode = failed ? 1 : 0;
