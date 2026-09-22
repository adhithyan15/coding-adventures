// mock-stem-coverage-cli — print the question words the book has not taught.
//
// REPORT ONLY, and that is not a stage on the way to a gate.
//
// A gate here would have to decide, per word, whether a candidate can read it.
// That decision is exactly what went wrong three times over (6 → 15 → 17 → 32 →
// 48, every correction in the same direction), and automating a judgement that
// has been wrong three times does not make it right — it makes it fast and
// unreviewable. What the tool can do honestly is put the same list in front of
// a reader every time, so the next pass starts where the last one finished
// instead of from somebody's memory of it.
//
//   npm run report:mock-stem-coverage              # A2, both mocks
//   npm run report:mock-stem-coverage -- --level A1
//   npm run report:mock-stem-coverage -- --all     # every bucket, not just the two
//
// Exit code is 0 unless the papers or the vocabulary cannot be read. A report
// that finds fifty unaccounted words is doing its job, not failing.
import { pathToFileURL } from "node:url";
import { defaultCurriculumRoot } from "./loader.js";
import { stripControlCharacters } from "./constants.js";
import { reportSpanishMockStemCoverage } from "./mock-stem-coverage.js";
import type { MockAuditLevel } from "./spanish-a1-mock-audit-cli.js";

export function runMockStemCoverageCli(
  args = process.argv.slice(2),
  root = defaultCurriculumRoot(),
  write: (text: string) => void = (text) => process.stdout.write(text),
): number {
  const levelArg = args.includes("--level") ? args[args.indexOf("--level") + 1] : "A2";
  const level: MockAuditLevel | undefined =
    levelArg === "pre-A1" || levelArg === "A1" || levelArg === "A2" ? levelArg : undefined;
  if (level === undefined) {
    process.stderr.write("usage: mock-stem-coverage-cli [--level pre-A1|A1|A2] [--all]\n");
    return 2;
  }
  const showAll = args.includes("--all");

  for (const { mock, forms } of reportSpanishMockStemCoverage(root, level)) {
    // `unaccounted` first: no declared list explains these and no taught word is
    // even a plausible relative, so they are the ones a reader must decide about.
    // `derivable` second, each beside the word it matched, because that is where
    // a wrong match hides — and a wrong match there is what put `espacio` in the
    // stem of a passing item.
    const order = showAll
      ? (["unaccounted", "derivable", "in-requires", "taught", "function-word", "apparatus", "proper-noun"] as const)
      : (["unaccounted", "derivable"] as const);
    write(`\n${level} mock ${mock}\n`);
    for (const bucket of order) {
      const rows = forms.filter((form) => form.bucket === bucket);
      write(`  ${bucket} (${rows.length})\n`);
      for (const row of rows) {
        const matched = row.matched === undefined ? "" : `  ~ ${stripControlCharacters(row.matched)}`;
        write(
          `    ${stripControlCharacters(row.form).padEnd(18)}` +
            `${row.places.slice(0, 4).join(", ")}${matched}\n`,
        );
      }
    }
  }
  return 0;
}

/* c8 ignore start -- the shebang path, exercised by the package script only. */
if (process.argv[1] !== undefined && import.meta.url === pathToFileURL(process.argv[1]).href) {
  process.exitCode = runMockStemCoverageCli();
}
/* c8 ignore stop */
