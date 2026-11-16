/**
 * # Honest coverage labels for exhaustive suites (backlog LP-012)
 *
 * The corpus calls every enumerated suite `kind: "exhaustive"`, and VER04i's
 * manifest has no room for a finer word. But "exhaustive" over `Bytes[347]` is a
 * statement about 347 chosen byte strings, not about every byte string, and
 * adding such totals together hides that (LP-012: the same conflation inflates
 * the Rust figures). So each suite also gets a label *derived from its domain
 * descriptors*, never typed by hand:
 *
 * | label         | meaning                                                         |
 * |---------------|-----------------------------------------------------------------|
 * | `full`        | the descriptor enumerates **every** value of its type (`bool`, `u8_all`, `Bytes<0>`, `Bytes<1>`) |
 * | `bounded-set` | a fixed, deterministic set of boundary and structured values that is a strict subset of the type (`u64_edge`, `bytes_d1`, `bytes_n(N)` for `N >= 2`, `nat_0_64`) |
 * | `sample`      | pseudo-random or otherwise unstructured draws. No portable descriptor produces one yet; the label is reserved so a future sampled suite cannot be mistaken for a bounded set |
 *
 * A suite is `full` only if *every* binder is `full`; otherwise it is as weak as
 * its weakest binder (`sample` over `bounded-set` over `full`).
 *
 * The labels live in a sidecar, `evidence/domain-labels.json`, because the
 * `warrant-lane-evidence-v1` rows are a closed key set that the Rust validator
 * (WD-004a) rejects additions to.
 */

import type { Corpus } from "./corpus.ts";
import { CorpusError } from "./errors.ts";

export const SCHEMA = "warrant-lane-domain-labels-v1";
export const NOTE =
  "Suite-level 'exhaustive' counts conflate full, bounded-set and sample " +
  "domains until backlog LP-012 lands; use cases_by_label, not the " +
  "manifest's exhaustive total, as the headline.";
const RANK: Readonly<Record<string, number>> = { full: 0, "bounded-set": 1, sample: 2 };
const BYTES_N = /^bytes_n\(([0-9]+)\)$/;

/** The label of one domain descriptor (see the module comment). */
export function domainLabel(descriptor: string): string {
  if (descriptor === "bool" || descriptor === "u8_all") return "full";
  if (descriptor === "u64_edge" || descriptor === "bytes_d1" || descriptor === "nat_0_64") return "bounded-set";
  const match = BYTES_N.exec(descriptor);
  if (match) {
    // Bytes<0> has one value and Bytes<1> has 256: both enumerate the type.
    return Number(match[1]) <= 1 ? "full" : "bounded-set";
  }
  throw new CorpusError("unsupported-domain", "no coverage label for this descriptor");
}

/** The label document for `corpus`, bound to one manifest by digest. */
export function deriveLabels(
  corpus: Corpus,
  packageName: string,
  module: string,
  manifestSha256: string,
): Record<string, unknown> {
  const byLabel: Record<string, number> = {};
  const suites = corpus.suites.map((suite) => {
    const binders = suite.binders.map(([name, domain]) => ({ name, domain, label: domainLabel(domain) }));
    const label = binders.reduce(
      (weakest, binder) => ((RANK[binder.label] as number) > (RANK[weakest] as number) ? binder.label : weakest),
      "full",
    );
    byLabel[label] = (byLabel[label] ?? 0) + suite.cases;
    return { id: suite.id, label, domain: suite.domainText, cases: suite.cases, binders };
  });
  return {
    schema: SCHEMA,
    package: packageName,
    module,
    manifest_sha256: manifestSha256,
    note: NOTE,
    cases_by_label: byLabel,
    suites: suites.sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0)),
  };
}
