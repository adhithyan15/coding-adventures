export const meta = {
  name: 'adj52-diagnostic-pipeline',
  description: 'FAILURE-ENRICHED diagnostic re-run of the ADJ52 hands-off adjudication pipeline. Same five arms (prepare+perturb -> domain-blind ingest -> recursive rulebook derive + engine run -> plain-Claude control -> blind judge) but seeded toward the specialties/angles that failed in run-3, and PERSISTS every per-case artifact (rulebook, program, engine output, ground truth, judge rationale) so each wrong case can be root-caused. macOS paths.',
  phases: [
    { title: 'Prepare' },
    { title: 'Ingest' },
    { title: 'DeriveRun' },
    { title: 'Control' },
    { title: 'Judge' },
  ],
}

// ---- Failure-enriched seeds (30) ----
// run-3 failures were spread across these specialties under the misleading-
// presentation angles. We reuse the proven generic seed phrasing (specialty x
// angle, which found 100/100 cases) but curate the 30 combos toward the areas
// that produced `incorrect`/`partial` framework verdicts, so this batch is
// dense in exactly the cases we want to root-cause. Each case writes its OWN
// per-case-id rulebook dir, so concurrent cases never race on a file.
const A1 = 'where the initial working diagnosis turned out to be wrong'
const A2 = 'where the presentation mimicked a far more common condition'
const A3 = 'with an unexpected final diagnosis after an initially misleading workup'
const A4 = 'initially misattributed to a benign or unrelated cause'
const A5 = 'where a rare disease masqueraded as a common one'
const DIAG_SEEDS = [
  `an adult clinical case report in vascular medicine ${A1}`,
  `an adult clinical case report in neurology ${A5}`,
  `an adult clinical case report in rheumatology ${A3}`,
  `an adult clinical case report in infectious disease ${A2}`,
  `an adult clinical case report in urology ${A4}`,
  `an adult clinical case report in gastroenterology ${A3}`,
  `an adult clinical case report in hepatology ${A5}`,
  `an adult clinical case report in pulmonology ${A1}`,
  `an adult clinical case report in otolaryngology ${A4}`,
  `an adult clinical case report in clinical immunology ${A2}`,
  `an adult clinical case report in hematology ${A3}`,
  `an adult clinical case report in endocrinology ${A5}`,
  `an adult clinical case report in cardiology ${A1}`,
  `an adult clinical case report in medical oncology ${A4}`,
  `an adult clinical case report in nephrology ${A2}`,
  `an adult clinical case report in dermatology ${A5}`,
  `an adult clinical case report in gastroenterology ${A3}`,
  `an adult clinical case report in neurology ${A5}`,
  `an adult clinical case report in infectious disease ${A1}`,
  `an adult clinical case report in general internal medicine ${A4}`,
  `an adult clinical case report in urology ${A2}`,
  `an adult clinical case report in otolaryngology ${A5}`,
  `an adult clinical case report in rheumatology ${A3}`,
  `an adult clinical case report in hematology ${A4}`,
  `an adult clinical case report in pulmonology ${A1}`,
  `an adult clinical case report in endocrinology ${A2}`,
  `an adult clinical case report in medical oncology ${A5}`,
  `an adult clinical case report in gastroenterology ${A3}`,
  `an adult clinical case report in cardiology ${A4}`,
  `an adult clinical case report in infectious disease ${A2}`,
]
const seeds = (args && args.seeds && args.seeds.length) ? args.seeds : DIAG_SEEDS
if (!seeds.length) { log('no seeds'); return { error: 'no seeds' } }
log(`diagnostic pipeline over ${seeds.length} failure-enriched seed(s)`)

const ROOT = '/Users/adhithya/Downloads/coding-adventures/code/specs/data/adj52'
const CASE_DIR = `${ROOT}/cases`
const MANIFEST = `${ROOT}/Cargo.toml`

// ---- Schemas (force structured output; no parsing) ----
const PREPARE_SCHEMA = {
  type: 'object',
  required: ['skipped', 'ground_truth', 'prose', 'perturbations', 'diagnosis_unchanged'],
  properties: {
    skipped: { type: 'boolean', description: 'true if no published case with a clearly documented final diagnosis was found for this seed' },
    source_url: { type: 'string', description: 'URL of the case report used (empty if skipped)' },
    ground_truth: { type: 'string', description: 'final confirmed diagnosis + how confirmed + correct disposition; NOT shown to blind agents' },
    prose: { type: 'string', description: 'PERTURBED, sanitised case prose (no diagnosis named, all incidental surface details changed)' },
    perturbations: { type: 'array', items: { type: 'string' }, description: 'list of the diagnosis-irrelevant changes made' },
    diagnosis_unchanged: { type: 'boolean', description: 'true iff every load-bearing finding for the diagnosis was preserved' },
  },
}
const IR_SCHEMA = {
  type: 'object',
  required: ['inferred_domain', 'facts', 'queries'],
  properties: {
    inferred_domain: { type: 'string' },
    facts: { type: 'array', items: { type: 'object' } },
    uncertainties: { type: 'array', items: { type: 'object' } },
    queries: { type: 'array', items: { type: 'object' } },
    discarded: { type: 'array', items: { type: 'object' } },
  },
}
const FW_SCHEMA = {
  type: 'object',
  required: ['domain_key', 'rules_added', 'compiled_ok', 'top_conclusion', 'top_posterior', 'recommended_next_step', 'framework_answer_text'],
  properties: {
    domain_key: { type: 'string', description: 'canonical lowercase_snake clinical-AREA key (the area, NOT the diagnosis)' },
    rules_added: { type: 'number', description: 'number of NEW clauses this case added' },
    compiled_ok: { type: 'boolean' },
    top_conclusion: { type: 'string' },
    top_posterior: { type: 'number' },
    second_conclusion: { type: 'string', description: 'the runner-up diagnosis and its posterior, if any (for differential-coherence analysis)' },
    recommended_next_step: { type: 'string' },
    framework_answer_text: { type: 'string', description: 'neutral rendering for the judge: ranked posteriors + key fired clauses WITH citations + next step + open uncertainties' },
    engine_output_excerpt: { type: 'string', description: 'the FULL stdout of the cargo run (posteriors, fired clauses, mechanism lines, kickback panel)' },
  },
}
const PLAIN_SCHEMA = {
  type: 'object',
  required: ['answer_text'],
  properties: { answer_text: { type: 'string', description: 'most likely diagnosis + recommended next action + confidence + brief reasoning' } },
}
const VERDICT_SCHEMA = {
  type: 'object',
  required: ['winner', 'rationale', 'a_correct', 'b_correct'],
  properties: {
    winner: { type: 'string', enum: ['A', 'B', 'tie'] },
    a_correct: { type: 'string', enum: ['correct', 'partial', 'incorrect'] },
    b_correct: { type: 'string', enum: ['correct', 'partial', 'incorrect'] },
    rationale: { type: 'string', description: 'detailed: what each output got right/wrong vs ground truth, calibration, defensibility' },
  },
}

// ---- Prompts ----
const preparePrompt = (seed) => `Find and prepare a published clinical case for a blinded diagnostic-reasoning experiment. SEED: ${seed}.

1. Use WebSearch/WebFetch to FIND ONE real published case report matching the seed — prefer open-access (e.g. PMC) — with a CLEARLY DOCUMENTED final/confirmed diagnosis (confirmed by biopsy, culture, genetics, imaging, autopsy, or follow-up). Favor cases whose presentation was initially misleading. AVOID the most famous textbook cases. If after a genuine search you cannot find one with a clearly documented diagnosis, return skipped=true and leave the other fields empty/false.
2. Extract the GROUND TRUTH: final confirmed diagnosis, how it was confirmed, and the correct disposition/treatment. Record source_url. (Held aside; not shown to downstream agents.)
3. Produce a PERTURBED, SANITISED vignette for the blind agents:
   - SANITISE: remove any sentence naming the final diagnosis, the confirmatory result that reveals it, expert discussion, or conclusions. Present only the raw clinical course up to the point of diagnostic uncertainty.
   - PERTURB to defeat training-data recall: change EVERY diagnosis-irrelevant surface detail — age (within a dx-preserving range), sex/ethnicity only if not diagnostically relevant, ALL specific lab numbers (shifted but kept on the same side of their reference range and same qualitative magnitude), anecdotes/wording, timeline, drug names/doses, institution, sentence order and phrasing.
   - PRESERVE in substance EVERY load-bearing finding (the discriminators), so the diagnosis is UNCHANGED.
   - The result must read as a fresh, natural vignette that no longer matches the published text verbatim.
Return skipped, source_url, ground_truth, the perturbed prose, the perturbations list, and diagnosis_unchanged (true only if all load-bearing findings preserved).`

const ingestPrompt = (prose) => `You are an ingester. Read EVERY byte of the problem statement below and decompose it into a human-readable IR. You are NOT solving it. Infer the domain yourself (you are NOT told it). Account for every byte: each span is a typed element OR explicitly discarded WITH A REASON (silent omission forbidden). Ambiguity becomes an uncertainty with candidate readings, NEVER a guess. You MAY use WebSearch only to disambiguate terminology; you MUST NOT look up the case outcome or read local files. Raise the queries the problem actually asks.
Return inferred_domain, facts [{id,term,source_span}], uncertainties [{id,about,domain,source_span}], queries [{id,predicate,rationale}], discarded [{source_span,reason}]. Terms are snake_case atoms or single-arg compounds.

=== PROBLEM STATEMENT ===
${prose}
=== END ===`

const deriveRunPrompt = (id, irJson) => `You derive a rulebook for a deterministic Bayesian logic engine and RUN one case against it. You are given an ingested IR (no answer). Use your tools for every step.

1. Choose a canonical domain key: a stable lowercase_snake_case name for the CLINICAL AREA (the AREA, never a specific diagnosis) — for reporting only.
2. Decide the candidate differential from the IR. Use WebSearch/WebFetch for REAL citations. Recurse into subcategories. Add a rule ONLY because a citable source supports it — NEVER because it would make this case come out a particular way (anti-overfit).
3. adj-lang grammar: every identifier matches /[a-z_][a-z0-9_]*/ (lowercase, NO uppercase, NO leading digit); encode magnitudes QUALITATIVELY (creatine_kinase(markedly_elevated), age(over_50)). Every clause preceded by "% rationale:" and annotated source "<citation>" + trust <consensus|authoritative|empirical|inferred|unattributed>. contributes/interacts are multiplicative LRs (>1 raises, <1 lowers). Mark decision-relevant unresolved items as uncertain { ... } for <conclusion> AND write contributes FROM each candidate test-result term so resolving it moves the posterior.
4. CORRELATED EVIDENCE (important): when several findings are correlated manifestations of ONE underlying mechanism (the sources describe them as a syndrome / shared cause), do NOT write an independent contributes for each — that double-counts and saturates the posterior. Instead group them under ONE directive comment line: "% mechanism <m> for <conclusion> lr <L> : <finding1>, <finding2>, ..." which fires the combined likelihood ratio L ONCE if any manifestation is observed. <m> is a lowercase_snake mechanism name. Keep only genuinely independent findings as separate contributes.
5. Write the rulebook (Write tool, ABSOLUTE path): ${CASE_DIR}/${id}/rulebook.adj — clauses + mechanism directives ONLY, no observe/query lines.
6. Write this case's PROGRAM (Write tool, ABSOLUTE path): ${CASE_DIR}/${id}/program.adj — ONLY observe <term> lines for THIS patient (same vocabulary as the rulebook) and ? <conclusion> query lines for each candidate + the next step. NO rules in the program.
7. Run via the Bash tool (exact): ADJ52_RULEBOOK=cases/${id}/rulebook.adj ADJ52_PROGRAM=cases/${id}/program.adj cargo run --quiet --manifest-path "${MANIFEST}" --bin adj52 . If the output contains "COMPILE ERROR", fix the offending term (usually an identifier with uppercase or a leading digit) and re-run until it compiles.
8. Return: domain_key, rules_added (number of clauses you wrote), compiled_ok, top_conclusion + top_posterior, second_conclusion (runner-up + its posterior), recommended_next_step, engine_output_excerpt (the FULL stdout of the successful run), and framework_answer_text — a neutral judge-facing rendering with the ranked posteriors, the KEY fired clauses WITH citations (especially any discriminator that fired negative against a tempting wrong answer), the recommended next step, and any open uncertainties.

IR:
${irJson}`

const plainPrompt = (prose) => `Read this problem statement and answer the question(s) it raises as you normally would. Do not look up any specific published case; do not read local files. Give the most likely diagnosis, the recommended next action, your confidence, and brief reasoning. Return answer_text with all of that.

=== PROBLEM STATEMENT ===
${prose}
=== END ===`

const judgePrompt = (gt, a, b) => `You are an impartial judge. Score two responses (OUTPUT A, OUTPUT B) from systems whose identities are hidden, against the ground truth. Judge only content; do not guess which system is which. Assess correctness vs ground truth, hallucination, calibration (appropriate confidence + right confirmatory step), and defensibility (traceable/verifiable reasoning). In your rationale, be SPECIFIC about what each output got right and wrong relative to ground truth and how its confidence compared to its correctness. Pick a winner.

=== GROUND TRUTH ===
${gt}

=== OUTPUT A ===
${a}

=== OUTPUT B ===
${b}`

// ---- Pipeline: each case flows through all stages independently ----
const results = await pipeline(
  seeds,
  (seed, _orig, idx) => agent(preparePrompt(seed), { phase: 'Prepare', label: `prepare:case-${idx + 1}`, agentType: 'general-purpose', schema: PREPARE_SCHEMA })
    .then((p) => {
      if (p.skipped || !p.prose) { throw new Error(`no suitable case for seed ${idx + 1}`) }
      return { seed, source_url: p.source_url, id: `case-${idx + 1}`, ground_truth: p.ground_truth, prose: p.prose, perturbations: p.perturbations, diagnosis_unchanged: p.diagnosis_unchanged }
    }),
  (o) => agent(ingestPrompt(o.prose), { phase: 'Ingest', label: `ingest:${o.id}`, agentType: 'general-purpose', schema: IR_SCHEMA })
    .then((ir) => ({ ...o, ir })),
  (o) => agent(deriveRunPrompt(o.id, JSON.stringify(o.ir)), { phase: 'DeriveRun', label: `derive:${o.id}`, agentType: 'general-purpose', schema: FW_SCHEMA })
    .then((fw) => ({ ...o, fw })),
  (o) => agent(plainPrompt(o.prose), { phase: 'Control', label: `plain:${o.id}`, agentType: 'general-purpose', schema: PLAIN_SCHEMA })
    .then((plain) => ({ ...o, plain })),
  (o, _orig, idx) => {
    // Deterministic A/B blinding: even index -> framework is A; odd -> framework is B.
    const fwIsA = (idx % 2) === 0
    const A = fwIsA ? o.fw.framework_answer_text : o.plain.answer_text
    const B = fwIsA ? o.plain.answer_text : o.fw.framework_answer_text
    return agent(judgePrompt(o.ground_truth, A, B), { phase: 'Judge', label: `judge:${o.id}`, agentType: 'general-purpose', schema: VERDICT_SCHEMA })
      .then((v) => ({
        id: o.id,
        seed: o.seed,
        source_url: o.source_url,
        diagnosis_unchanged: o.diagnosis_unchanged,
        perturbations: o.perturbations,
        // DIAGNOSTIC: keep ground truth + full texts so wrong cases can be root-caused.
        ground_truth: o.ground_truth,
        fw_domain: o.fw.domain_key,
        fw_rules_added: o.fw.rules_added,
        fw_compiled: o.fw.compiled_ok,
        fw_top: `${o.fw.top_conclusion} @ ${o.fw.top_posterior}`,
        fw_second: o.fw.second_conclusion || '',
        fw_next_step: o.fw.recommended_next_step,
        fw_answer_text: o.fw.framework_answer_text,
        fw_engine_output: o.fw.engine_output_excerpt || '',
        plain_answer_text: o.plain.answer_text,
        fw_is: fwIsA ? 'A' : 'B',
        winner: v.winner,
        framework_correct: fwIsA ? v.a_correct : v.b_correct,
        plain_correct: fwIsA ? v.b_correct : v.a_correct,
        framework_won: v.winner === (fwIsA ? 'A' : 'B'),
        plain_won: v.winner === (fwIsA ? 'B' : 'A'),
        rationale: v.rationale,
      }))
  }
)

// ---- Aggregate (read this, not each case) ----
const done = results.filter(Boolean)
const tally = {
  seeds_attempted: seeds.length,
  cases_completed: done.length,
  skipped_or_failed: seeds.length - done.length,
  framework_won: done.filter((r) => r.framework_won).length,
  plain_won: done.filter((r) => r.plain_won).length,
  tie: done.filter((r) => r.winner === 'tie').length,
  framework_correct: done.filter((r) => r.framework_correct === 'correct').length,
  framework_partial: done.filter((r) => r.framework_correct === 'partial').length,
  framework_incorrect: done.filter((r) => r.framework_correct === 'incorrect').length,
  plain_correct: done.filter((r) => r.plain_correct === 'correct').length,
  fw_compile_failures: done.filter((r) => !r.fw_compiled).length,
}
// The wrong cases — the whole point of this diagnostic run.
const wrong = done.filter((r) => r.framework_correct !== 'correct').map((r) => r.id)
log(`AGGREGATE: ${tally.cases_completed}/${tally.seeds_attempted} completed; fw correct ${tally.framework_correct} / partial ${tally.framework_partial} / incorrect ${tally.framework_incorrect}; compile failures ${tally.fw_compile_failures}; WRONG cases to root-cause: ${wrong.join(', ')}`)
return { tally, wrong, per_case: done }
