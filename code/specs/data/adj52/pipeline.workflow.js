export const meta = {
  name: 'adj52-case-pipeline',
  description: 'Hands-off adjudication pipeline over published clinical cases: fetch + diagnosis-invariant PERTURB (defeat training-data recall) -> domain-blind ingest -> recursive byte-provenanced rulebook derive + engine run -> plain-Claude control -> blind judge -> aggregate. No human in the loop; read the aggregate, not each case.',
  phases: [
    { title: 'Prepare' },
    { title: 'Ingest' },
    { title: 'DeriveRun' },
    { title: 'Control' },
    { title: 'Judge' },
  ],
}

// args: { case_urls: ["https://pmc.ncbi.nlm.nih.gov/articles/PMCxxxxx/", ...] }
const urls = (args && args.case_urls) || []
if (!urls.length) { log('no case_urls provided in args'); return { error: 'no case_urls' } }
log(`pipeline over ${urls.length} case url(s)`)

// ---- Schemas (force structured output; no parsing) ----
const PREPARE_SCHEMA = {
  type: 'object',
  required: ['id', 'ground_truth', 'prose', 'perturbations', 'diagnosis_unchanged'],
  properties: {
    id: { type: 'string', description: 'short kebab-case slug for this case' },
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
  required: ['compiled_ok', 'top_conclusion', 'top_posterior', 'recommended_next_step', 'framework_answer_text'],
  properties: {
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

const deriveRunPrompt = (id, irJson) => `You build a rulebook for a deterministic Bayesian logic engine and then RUN it. You are given an ingested IR (no answer). Do all steps with your tools:

1. From the IR alone, decide the candidate differential. Use WebSearch/WebFetch for REAL citations. Recurse into subcategories where evidence profiles differ; do not flatten.
2. Write an adj-lang rulebook. CRITICAL grammar rule: every identifier (atom and compound functor/arg) must match /[a-z_][a-z0-9_]*/ — lowercase only, NO uppercase, and it must NOT start with a digit. Encode magnitudes QUALITATIVELY (e.g. creatine_kinase(markedly_elevated), age(over_50)) — never numbers/units inside terms. Every clause is preceded by a "% rationale:" line and annotated with source "<real citation>" and trust <consensus|authoritative|empirical|inferred|unattributed>. contributes/interacts magnitudes are multiplicative likelihood ratios (>1 raises, <1 lowers). Mark decision-relevant unresolved items as uncertain { ... } for <conclusion>, AND write contributes clauses FROM each candidate test-result term so resolving it actually moves the posterior.
3. Write two files with the Write tool (ABSOLUTE paths required):
   - C:/Users/adhit/Downloads/coding-adventures/code/specs/data/adj52/cases/${id}/03-derived-rulebook.adj  (the clauses)
   - C:/Users/adhit/Downloads/coding-adventures/code/specs/data/adj52/cases/${id}/04-vignette.adj  (observe <term> lines for THIS patient using the SAME vocabulary, then ? <conclusion> query lines for each candidate + the next step)
4. Run via the Bash tool (exact command): ADJ52_DIR=cases/${id} cargo run --quiet --manifest-path "C:/Users/adhit/Downloads/coding-adventures/code/specs/data/adj52/Cargo.toml"
   If the output contains "COMPILE ERROR", fix the offending terms (usually an identifier with uppercase or a leading digit) and re-run until it compiles.
5. Return: compiled_ok, top_conclusion + top_posterior (the highest-posterior diagnosis), recommended_next_step (highest next_step query), engine_output_excerpt (the per-query posteriors), and framework_answer_text — a neutral rendering for a blind judge containing the RANKED posteriors, the KEY fired clauses WITH their citations (especially any discriminator that fired negative against a tempting wrong answer), the recommended next step, and any open uncertainties.

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
const results = await pipeline(
  urls,
  (url) => agent(preparePrompt(url), { phase: 'Prepare', agentType: 'general-purpose', schema: PREPARE_SCHEMA })
    .then((p) => ({ url, ...p })),
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
