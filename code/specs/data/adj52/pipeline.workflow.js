export const meta = {
  name: 'adj52-case-pipeline',
  description: 'Hands-off adjudication pipeline over published clinical cases: fetch + diagnosis-invariant PERTURB (defeat training-data recall) -> domain-blind ingest -> recursive byte-provenanced rulebook derive into an ACCUMULATING domain rulebook store + engine run on a swappable per-case program -> plain-Claude control -> blind judge -> aggregate. No human in the loop; read the aggregate, not each case.',
  phases: [
    { title: 'Prepare' },
    { title: 'Ingest' },
    { title: 'DeriveRun' },
    { title: 'Control' },
    { title: 'Judge' },
  ],
}

// Cases to run. Override via args.case_urls; defaults below so the script is
// self-contained. These are published "masquerade" cases the model has likely
// seen — the Prepare stage perturbs them so neither arm can recall the text.
const DEFAULT_URLS = [
  'https://pmc.ncbi.nlm.nih.gov/articles/PMC11724029/',
  'https://pmc.ncbi.nlm.nih.gov/articles/PMC12003113/',
  'https://pmc.ncbi.nlm.nih.gov/articles/PMC9097753/',
]
const urls = (args && args.case_urls && args.case_urls.length) ? args.case_urls : DEFAULT_URLS
if (!urls.length) { log('no case_urls'); return { error: 'no case_urls' } }
log(`pipeline over ${urls.length} case url(s)`)

// ---- Schemas (force structured output; no parsing) ----
const PREPARE_SCHEMA = {
  type: 'object',
  required: ['ground_truth', 'prose', 'perturbations', 'diagnosis_unchanged'],
  properties: {
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
    domain_key: { type: 'string', description: 'canonical lowercase_snake clinical-AREA key for the accumulating rulebook (the area, NOT the diagnosis)' },
    rules_added: { type: 'number', description: 'number of NEW clauses this case added to the accumulating rulebook' },
    compiled_ok: { type: 'boolean' },
    top_conclusion: { type: 'string' },
    top_posterior: { type: 'number' },
    recommended_next_step: { type: 'string' },
    framework_answer_text: { type: 'string', description: 'neutral rendering for the judge: ranked posteriors + key fired clauses WITH citations + next step + open uncertainties' },
    engine_output_excerpt: { type: 'string' },
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
    rationale: { type: 'string' },
  },
}

// ---- Prompts ----
const preparePrompt = (url) => `Fetch this published clinical case report and prepare it for a blinded diagnostic-reasoning experiment. URL: ${url}

Do BOTH of these:
1. Extract the GROUND TRUTH: the final confirmed diagnosis, how it was confirmed, and the correct disposition/treatment. (This is held aside; not shown to downstream agents.)
2. Produce a PERTURBED, SANITISED version of the case prose for downstream blind agents. Requirements:
   - SANITISE: remove any sentence that names the final diagnosis, the confirmatory result that reveals it, expert discussion, or the article's conclusions. Present only the raw clinical course up to the point of diagnostic uncertainty.
   - PERTURB to defeat training-data recall (the model has likely seen this published case): change EVERY diagnosis-irrelevant surface detail — patient age (keep within a range that does not change the diagnosis), sex/ethnicity only if not diagnostically relevant, ALL specific lab numbers (shift them but keep them on the same side of their reference range and the same qualitative magnitude), specific anecdotes/wording, the timeline, drug names/doses, institution, sentence order and phrasing.
   - PRESERVE, in substance, EVERY finding that is load-bearing for the true diagnosis (the discriminators), so the diagnosis is UNCHANGED. Do not add or remove diagnostic signal; only reword/renumber it.
   - The result must read as a fresh, natural case vignette that no longer matches the published text verbatim.
Return the ground_truth, the perturbed prose, the list of perturbations you made, and diagnosis_unchanged=true only if you preserved all load-bearing findings.`

const ingestPrompt = (prose) => `You are an ingester. Read EVERY byte of the problem statement below and decompose it into a human-readable IR. You are NOT solving it. Infer the domain yourself (you are NOT told it). Account for every byte: each span is a typed element OR explicitly discarded WITH A REASON (silent omission forbidden). Ambiguity becomes an uncertainty with candidate readings, NEVER a guess. You MAY use WebSearch only to disambiguate terminology; you MUST NOT look up the case outcome or read local files. Raise the queries the problem actually asks.
Return inferred_domain, facts [{id,term,source_span}], uncertainties [{id,about,domain,source_span}], queries [{id,predicate,rationale}], discarded [{source_span,reason}]. Terms are snake_case atoms or single-arg compounds.

=== PROBLEM STATEMENT ===
${prose}
=== END ===`

const RB_DIR = 'C:/Users/adhit/Downloads/coding-adventures/code/specs/data/adj52/rulebooks'
const CASE_DIR = 'C:/Users/adhit/Downloads/coding-adventures/code/specs/data/adj52/cases'
const deriveRunPrompt = (id, irJson) => `You maintain an ACCUMULATING rulebook for a deterministic Bayesian logic engine and RUN one case against it. You are given an ingested IR (no answer). Use your tools for every step.

1. Choose a canonical domain key: a stable lowercase_snake_case name for the CLINICAL AREA this case belongs to (e.g. proximal_myopathy_workup, anterior_neck_mass_workup) — the AREA, never a specific diagnosis.
2. Read the existing rulebook if present (Read tool): ${RB_DIR}/<domain_key>.adj . If it does not exist, start fresh.
3. Decide the candidate differential from the IR. Use WebSearch/WebFetch for REAL citations. Recurse into subcategories. Then ADD ONLY THE RULES THE EXISTING RULEBOOK IS MISSING (new conclusions / new evidence edges); KEEP every existing clause unchanged. Add a rule ONLY because a citable source supports it — NEVER because it would make this case come out a particular way (anti-overfit).
4. adj-lang grammar: every identifier matches /[a-z_][a-z0-9_]*/ (lowercase, NO uppercase, NO leading digit); encode magnitudes QUALITATIVELY (creatine_kinase(markedly_elevated), age(over_50)). Every clause preceded by "% rationale:" and annotated source "<citation>" + trust <consensus|authoritative|empirical|inferred|unattributed>. contributes/interacts are multiplicative LRs (>1 raises, <1 lowers). Mark decision-relevant unresolved items as uncertain { ... } for <conclusion> AND write contributes clauses FROM each candidate test-result term so resolving it moves the posterior.
5. Write the GROWN rulebook back (Write tool, absolute): ${RB_DIR}/<domain_key>.adj  (RULES ONLY — no observe/query lines).
6. Write this case's PROGRAM, separate and swappable (Write tool, absolute): ${CASE_DIR}/${id}/program.adj — containing ONLY observe <term> lines for THIS patient (same vocabulary as the rulebook) and ? <conclusion> query lines for each candidate + the next step. NO rules in the program.
7. Run via the Bash tool (exact): ADJ52_RULEBOOK=rulebooks/<domain_key>.adj ADJ52_PROGRAM=cases/${id}/program.adj cargo run --quiet --manifest-path "C:/Users/adhit/Downloads/coding-adventures/code/specs/data/adj52/Cargo.toml"
   If the output contains "COMPILE ERROR", fix the offending term (usually an identifier with uppercase or a leading digit) and re-run until it compiles.
8. Return: domain_key, rules_added (count you added this case), compiled_ok, top_conclusion + top_posterior, recommended_next_step, engine_output_excerpt (per-query posteriors), and framework_answer_text — a neutral judge-facing rendering with the RANKED posteriors, the KEY fired clauses WITH citations (especially any discriminator that fired negative against a tempting wrong answer), the recommended next step, and any open uncertainties.

IR:
${irJson}`

const plainPrompt = (prose) => `Read this problem statement and answer the question(s) it raises as you normally would. Do not look up any specific published case; do not read local files. Give the most likely diagnosis, the recommended next action, your confidence, and brief reasoning. Return answer_text with all of that.

=== PROBLEM STATEMENT ===
${prose}
=== END ===`

const judgePrompt = (gt, a, b) => `You are an impartial judge. Score two responses (OUTPUT A, OUTPUT B) from systems whose identities are hidden, against the ground truth. Judge only content; do not guess which system is which. Assess correctness vs ground truth, hallucination, calibration (appropriate confidence + right confirmatory step), and defensibility (traceable/verifiable reasoning). Pick a winner.

=== GROUND TRUTH ===
${gt}

=== OUTPUT A ===
${a}

=== OUTPUT B ===
${b}`

// ---- Pipeline: each case flows through all stages independently ----
// Accumulation note: cases sharing a domain_key grow the SAME rulebook file.
// pipeline() runs cases concurrently, so genuine same-domain accumulation must
// be run SEQUENTIALLY (or with a lock) to avoid read-modify-write races on the
// shared rulebook. The default cases here are different clinical areas (so they
// seed separate rulebooks and don't race); a same-domain accumulation run
// should pass a single-area url list and be processed sequentially.
const results = await pipeline(
  urls,
  (url, _orig, idx) => agent(preparePrompt(url), { phase: 'Prepare', label: `prepare:case-${idx + 1}`, agentType: 'general-purpose', schema: PREPARE_SCHEMA })
    .then((p) => ({ url, id: `case-${idx + 1}`, ground_truth: p.ground_truth, prose: p.prose, perturbations: p.perturbations, diagnosis_unchanged: p.diagnosis_unchanged })),
  (o) => agent(ingestPrompt(o.prose), { phase: 'Ingest', label: `ingest:${o.id}`, agentType: 'general-purpose', schema: IR_SCHEMA })
    .then((ir) => ({ ...o, ir })),
  (o) => agent(deriveRunPrompt(o.id, JSON.stringify(o.ir)), { phase: 'DeriveRun', label: `derive:${o.id}`, agentType: 'general-purpose', schema: FW_SCHEMA })
    .then((fw) => ({ ...o, fw })),
  (o) => agent(plainPrompt(o.prose), { phase: 'Control', label: `plain:${o.id}`, agentType: 'general-purpose', schema: PLAIN_SCHEMA })
    .then((plain) => ({ ...o, plain })),
  (o, _orig, idx) => {
    // Deterministic A/B blinding (Math.random is unavailable in workflow scripts):
    // even index -> framework is A; odd -> framework is B.
    const fwIsA = (idx % 2) === 0
    const A = fwIsA ? o.fw.framework_answer_text : o.plain.answer_text
    const B = fwIsA ? o.plain.answer_text : o.fw.framework_answer_text
    return agent(judgePrompt(o.ground_truth, A, B), { phase: 'Judge', label: `judge:${o.id}`, agentType: 'general-purpose', schema: VERDICT_SCHEMA })
      .then((v) => ({
        id: o.id,
        diagnosis_unchanged: o.diagnosis_unchanged,
        perturbations: o.perturbations,
        fw_domain: o.fw.domain_key,
        fw_rules_added: o.fw.rules_added,
        fw_compiled: o.fw.compiled_ok,
        fw_top: `${o.fw.top_conclusion} @ ${o.fw.top_posterior}`,
        fw_next_step: o.fw.recommended_next_step,
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
  cases: done.length,
  framework_won: done.filter((r) => r.framework_won).length,
  plain_won: done.filter((r) => r.plain_won).length,
  tie: done.filter((r) => r.winner === 'tie').length,
  framework_correct: done.filter((r) => r.framework_correct === 'correct').length,
  plain_correct: done.filter((r) => r.plain_correct === 'correct').length,
  fw_compile_failures: done.filter((r) => !r.fw_compiled).length,
}
log(`AGGREGATE: framework won ${tally.framework_won}/${tally.cases}, plain ${tally.plain_won}, tie ${tally.tie}; framework correct ${tally.framework_correct}, plain correct ${tally.plain_correct}; compile failures ${tally.fw_compile_failures}`)
return { tally, per_case: done }
