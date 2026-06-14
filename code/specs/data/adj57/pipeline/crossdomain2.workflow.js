export const meta = {
  name: 'adj58-crossdomain-headtohead-round2',
  description: 'Test the byte-provenance framework OUTSIDE medicine. For each of 3 non-medical domains: find a real documented case, run the framework pipeline (decompose into a byte-covering IR -> derive leading hypothesis + total fact coverage -> spider links to ROOT sources), AND separately run plain Claude (no framework). The parent computes the framework verdict/trail (run.py) and a blind judge decides which report got it right.',
  phases: [{ title: 'Prepare' }, { title: 'Derive' }, { title: 'SpiderToRoot' }, { title: 'PlainClaude' }],
}

// 3 clearly non-medical domains with documented, ground-truthable cases.
const DOMAINS = [
  { id: 'geology', find: 'a documented MINERAL or ROCK IDENTIFICATION where an unknown specimen was identified from its observed physical/optical properties (hardness, streak, cleavage, crystal habit, density, luster, optical/XRD/chemistry). The "answer" is the mineral/rock species.' },
  { id: 'paleontology', find: 'a published FOSSIL IDENTIFICATION / taxonomic assignment where a specimen was assigned to a taxon from described morphology (bone/tooth/shell features, dimensions, stratigraphy). The "answer" is the taxon.' },
  { id: 'linguistics', find: 'a documented case identifying the LANGUAGE, SCRIPT, or ORIGIN of an unattributed text or inscription from internal features (orthography, lexicon, grammar, script) — e.g. a decipherment or forensic language-ID/authorship case. The "answer" is the language/script/origin/author.' },
]

const INGEST_SCHEMA = {
  type: 'object',
  required: ['source_url', 'ground_truth', 'case_text', 'segments', 'candidate_diagnoses'],
  properties: {
    source_url: { type: 'string' },
    ground_truth: { type: 'string', description: 'the confirmed ANSWER + how it was established. Held aside.' },
    case_text: { type: 'string', description: 'the EXACT scenario text decomposed (~500-1500 chars), the observed facts up to the decision point — NOT the answer/analysis. segments MUST concatenate to this exactly.' },
    segments: {
      type: 'array',
      description: 'ordered partition of case_text; concatenating segment.text reproduces case_text character-for-character.',
      items: {
        type: 'object', required: ['text', 'kind'],
        properties: {
          text: { type: 'string' }, kind: { type: 'string', enum: ['fact', 'discard'] },
          term: { type: 'string', description: 'for fact: a snake_case predicate, e.g. fracture_surface(beach_marks)' },
          reason: { type: 'string', description: 'for discard: why no diagnostic fact' },
        },
      },
    },
    candidate_diagnoses: { type: 'array', items: { type: 'string' }, description: 'the candidate answers the case raises (snake_case)' },
  },
}
const DERIVE_SCHEMA = {
  type: 'object',
  required: ['leading_diagnosis', 'prior_population', 'fact_dispositions', 'finding_links'],
  properties: {
    leading_diagnosis: { type: 'string', description: 'the single most likely ANSWER the FACTS point to (snake_case) — not ground truth' },
    prior_population: { type: 'string', description: 'the population/reference class whose base rate of leading_diagnosis is the right prior' },
    fact_dispositions: {
      type: 'array',
      description: 'account for EVERY fact term exactly once: USED (role) or DISCARDED (reason). No silent drops.',
      items: {
        type: 'object', required: ['fact', 'used'],
        properties: { fact: { type: 'string' }, used: { type: 'boolean' }, role: { type: 'string' }, reason: { type: 'string' } },
      },
    },
    finding_links: {
      type: 'array',
      description: 'the subset of USED facts that are likelihood-ratio links to ground for the leading answer (verbatim finding term).',
      items: { type: 'object', required: ['finding', 'expected_metric', 'decisive'],
        properties: { finding: { type: 'string' }, expected_metric: { type: 'string' }, decisive: { type: 'boolean' } } },
    },
  },
}
const SPIDER_SCHEMA = {
  type: 'object',
  required: ['prior', 'grounded_findings'],
  properties: {
    prior: { type: 'object', required: ['value', 'verdict', 'chain'],
      properties: { value: { type: 'number' }, verdict: { type: 'string', enum: ['grounded', 'direction_only', 'fabricated'] }, chain: { type: 'array', items: { type: 'object' } } } },
    grounded_findings: {
      type: 'array',
      items: {
        type: 'object', required: ['finding', 'computed_lr', 'verdict', 'chain'],
        properties: {
          finding: { type: 'string' }, computed_lr: { type: 'number' }, lr_formula: { type: 'string' },
          verdict: { type: 'string', enum: ['grounded', 'direction_only', 'fabricated'] }, reached_root: { type: 'boolean' },
          chain: { type: 'array', items: { type: 'object', required: ['hop', 'source_url', 'content_excerpt', 'cited_quote'],
            properties: { hop: { type: 'number' }, source_url: { type: 'string' }, source_title: { type: 'string' },
              content_excerpt: { type: 'string' }, cited_quote: { type: 'string' }, gives_root_data: { type: 'boolean' }, onward_citation: { type: 'string' } } } },
        },
      },
    },
  },
}
const PLAIN_SCHEMA = {
  type: 'object', required: ['answer', 'reasoning', 'confidence'],
  properties: {
    answer: { type: 'string', description: 'the single most likely answer' },
    reasoning: { type: 'string' },
    confidence: { type: 'string', description: 'qualitative or % confidence' },
  },
}

const ingestPrompt = (d) => `Find ONE real documented case and decompose its SCENARIO into a byte-covering IR. DOMAIN: ${d.find}

1. Choose a self-contained scenario passage (~500-1500 chars) — the observed facts/evidence up to the decision point. Do NOT include the final answer/analysis. Copy VERBATIM into case_text.
2. Partition case_text into ordered segments. CRITICAL: concatenating segment.text in order must reproduce case_text EXACTLY, character for character. No paraphrase, no omission.
3. Each segment is a fact (a snake_case term) or a discard (a reason).
4. Return candidate_diagnoses (the candidate ANSWERS), source_url, and held-aside ground_truth (the confirmed answer + how established).`

const derivePrompt = (o) => `From the facts of a case, identify the leading ANSWER and the links needed to score it. Derive from the FACTS.

CANDIDATE ANSWERS: ${JSON.stringify(o.candidate_diagnoses)}
FACTS (account for EVERY one):
${o.segments.filter((s) => s.kind === 'fact').map((s) => `  - ${s.term}`).join('\n')}

Account for EVERY fact in fact_dispositions (USED with a role, or DISCARDED with a reason — no silent drops). Return leading_diagnosis (the leading answer), prior_population (reference class for the base rate), fact_dispositions, and finding_links (the USED facts that are likelihood-ratio links to ground).`

const spiderPrompt = (lead, pop, links) => `You are a byte-provenance spider. Ground the numbers for a Bayesian verdict on the answer "${lead}", FOLLOWING CITATIONS BACK TO ROOT SOURCES.

(1) PRIOR: the base rate of "${lead}" within: ${pop}. Report as a probability with a chain to a root source.
(2) For EACH decisive link, compute the likelihood ratio = P(feature | ${lead}) / P(feature | not) from primary data:
${links.map((l) => `  - ${l.finding}  (${l.expected_metric})`).join('\n')}

For every source: content_excerpt (verbatim passage read) + cited_quote (literal sentence within it). If a source borrows the number, follow the citation to a ROOT source with the primary measured number. Grade verdict: grounded only if it traces to root primary data; direction_only if no usable number exists in the literature; fabricated if unsupported. Many non-medical domains lack published likelihood ratios — if so, return direction_only honestly rather than inventing a number.`

const plainPrompt = (caseText, domain) => `You are an expert. Read this ${domain} case and give your best answer. Reason as you normally would. Do NOT look up the specific published case. What is the single most likely answer, your reasoning, and your confidence?

=== CASE ===
${caseText}
=== END ===`

const results = await pipeline(
  DOMAINS,
  (d) => agent(ingestPrompt(d), { phase: 'Prepare', label: `prepare:${d.id}`, agentType: 'general-purpose', schema: INGEST_SCHEMA }).then((ingest) => ({ d, ingest })),
  (o) => agent(derivePrompt(o.ingest), { phase: 'Derive', label: `derive:${o.d.id}`, agentType: 'general-purpose', schema: DERIVE_SCHEMA }).then((derived) => ({ ...o, derived })),
  (o) => {
    const decisive = o.derived.finding_links.filter((l) => l.decisive)
    return agent(spiderPrompt(o.derived.leading_diagnosis, o.derived.prior_population, (decisive.length ? decisive : o.derived.finding_links).slice(0, 5)),
      { phase: 'SpiderToRoot', label: `spider:${o.d.id}`, agentType: 'general-purpose', schema: SPIDER_SCHEMA }).then((spidered) => ({ ...o, spidered }))
  },
  (o) => agent(plainPrompt(o.ingest.case_text, o.d.id), { phase: 'PlainClaude', label: `plain:${o.d.id}`, agentType: 'general-purpose', schema: PLAIN_SCHEMA })
    .then((plain) => ({ domain: o.d.id, ingest: o.ingest, derived: o.derived, spidered: o.spidered, plain })),
)
return { per_domain: results.filter(Boolean) }
