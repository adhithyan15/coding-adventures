export const meta = {
  name: 'adj57-full-run',
  description: 'Full end-to-end run of the generic byte-provenance framework on one fresh case, no comparison arms. L1: ingest into a byte-COVERING partition of typed facts + reasoned discards. L2: derive the leading diagnosis + the links needed to compute its posterior (prior + observed-finding LRs). L3: spider each link to a ROOT source, byte-provenanced. The parent then runs the deterministic Bayesian verdict, every step traced to a CAS span.',
  phases: [{ title: 'Ingest' }, { title: 'Derive' }, { title: 'SpiderToRoot' }],
}

const INGEST_SCHEMA = {
  type: 'object',
  required: ['source_url', 'ground_truth', 'case_text', 'segments', 'candidate_diagnoses'],
  properties: {
    source_url: { type: 'string' },
    ground_truth: { type: 'string', description: 'final confirmed diagnosis + how established. Held aside.' },
    case_text: { type: 'string', description: 'the EXACT case-presentation text decomposed (~500-1500 chars, to the decision point, no final answer). segments MUST concatenate to this exactly.' },
    segments: {
      type: 'array',
      description: 'ordered partition of case_text; concatenating segment.text in order reproduces case_text character-for-character.',
      items: {
        type: 'object', required: ['text', 'kind'],
        properties: {
          text: { type: 'string', description: 'verbatim substring of case_text' },
          kind: { type: 'string', enum: ['fact', 'discard'] },
          term: { type: 'string', description: 'for fact: a snake_case predicate, e.g. heart_rate(over_100)' },
          reason: { type: 'string', description: 'for discard: why this span carries no diagnostic fact' },
        },
      },
    },
    candidate_diagnoses: { type: 'array', items: { type: 'string' } },
  },
}
const DERIVE_SCHEMA = {
  type: 'object',
  required: ['leading_diagnosis', 'prior_population', 'fact_dispositions', 'finding_links'],
  properties: {
    leading_diagnosis: { type: 'string', description: 'the single most likely diagnosis the FACTS point to (snake_case), derived from the IR — not ground truth' },
    prior_population: { type: 'string', description: 'the population whose pretest prevalence of leading_diagnosis is the right prior (e.g. "adults presenting with X")' },
    fact_dispositions: {
      type: 'array',
      description: 'BYTE-PROVENANCE CONTRACT: account for EVERY fact term in the IR exactly once. Each is either USED (it bears on the leading diagnosis or sets the prior) or DISCARDED (with a reason — e.g. an unrelated comorbidity). No fact may be silently ignored.',
      items: {
        type: 'object', required: ['fact', 'used'],
        properties: {
          fact: { type: 'string', description: 'the fact term, verbatim from the IR (must match an IR fact exactly)' },
          used: { type: 'boolean' },
          role: { type: 'string', description: 'if used: how — e.g. "evidence_for_leading", "sets_prior_population", "confirmatory_test"' },
          reason: { type: 'string', description: 'if discarded: why it does not bear on the leading diagnosis (e.g. "comorbidity unrelated to brucellosis")' },
        },
      },
    },
    finding_links: {
      type: 'array',
      description: 'the subset of USED facts that are LR links to ground for the leading diagnosis. Use the fact term VERBATIM as `finding`.',
      items: {
        type: 'object', required: ['finding', 'expected_metric', 'decisive'],
        properties: {
          finding: { type: 'string', description: 'the fact term, verbatim from the IR' },
          expected_metric: { type: 'string', description: 'LR+ | LR- | OR' },
          decisive: { type: 'boolean' },
        },
      },
    },
  },
}
const SPIDER_SCHEMA = {
  type: 'object',
  required: ['prior', 'grounded_findings'],
  properties: {
    prior: {
      type: 'object', required: ['value', 'verdict', 'chain'],
      properties: {
        value: { type: 'number', description: 'pretest prevalence of the leading diagnosis as a probability (0 if ungroundable)' },
        verdict: { type: 'string', enum: ['grounded', 'direction_only', 'fabricated'] },
        chain: { type: 'array', items: { type: 'object' } },
      },
    },
    grounded_findings: {
      type: 'array',
      items: {
        type: 'object', required: ['finding', 'computed_lr', 'verdict', 'chain'],
        properties: {
          finding: { type: 'string' },
          computed_lr: { type: 'number' },
          lr_formula: { type: 'string' },
          verdict: { type: 'string', enum: ['grounded', 'direction_only', 'fabricated'] },
          reached_root: { type: 'boolean' },
          chain: {
            type: 'array',
            items: {
              type: 'object', required: ['hop', 'source_url', 'content_excerpt', 'cited_quote'],
              properties: {
                hop: { type: 'number' },
                source_url: { type: 'string' }, source_title: { type: 'string' },
                content_excerpt: { type: 'string', description: 'the passage read from this source (interned verbatim into the CAS)' },
                cited_quote: { type: 'string', description: 'the LITERAL sentence within content_excerpt bearing on the claim (verbatim substring)' },
                gives_root_data: { type: 'boolean' },
                onward_citation: { type: 'string' },
              },
            },
          },
        },
      },
    },
  },
}

const ingestPrompt = `Find ONE real published clinical case (prefer open-access PMC) with a documented final diagnosis, and decompose its PRESENTATION into a byte-covering IR. Pick a DIFFERENT case domain each time (vary it).

1. Choose a self-contained presentation passage (~500-1500 chars), to the diagnostic decision point. Do NOT include the final answer. Copy VERBATIM into case_text.
2. Partition case_text into ordered segments. CRITICAL: concatenating every segment.text in order must reproduce case_text EXACTLY, character for character — every space, comma, newline, digit. No paraphrase, no reordering, no omission.
3. Each segment is a fact (diagnostically meaningful — give a snake_case term) or a discard (give a reason). Punctuation/whitespace between facts is usually a discard.
4. Return candidate_diagnoses, source_url, and held-aside ground_truth.
Self-check: mentally concatenate the segments and confirm they equal case_text byte-for-byte (including newlines).`

const derivePrompt = (o) => `From the extracted facts of a case, identify the leading diagnosis and the links needed to compute its posterior. Derive from the FACTS, not from any answer.

CANDIDATE DIAGNOSES: ${JSON.stringify(o.candidate_diagnoses)}
EXTRACTED FACTS (account for EVERY one):
${o.segments.filter((s) => s.kind === 'fact').map((s) => `  - ${s.term}`).join('\n')}

BYTE-PROVENANCE CONTRACT FOR THIS STAGE: this stage consumes the facts above. You must account for EVERY fact term exactly once in fact_dispositions — each is either USED (with a role: evidence_for_leading / sets_prior_population / confirmatory_test) or DISCARDED (with a reason it does not bear on the leading diagnosis, e.g. an unrelated comorbidity). Do NOT silently ignore any fact. Copy each fact term VERBATIM.

Return: leading_diagnosis (the single most likely, from the facts); prior_population (whose prevalence is the right pretest prior); fact_dispositions (every fact, used-or-discarded-with-reason); and finding_links — the subset of USED facts that are LR links to ground (verbatim finding term, expected_metric, decisive).`

const spiderPrompt = (lead, pop, links) => `You are a byte-provenance spider. Ground the numbers for a Bayesian verdict on diagnosis(${lead}), FOLLOWING CITATIONS BACK TO ROOT SOURCES.

(1) PRIOR: find the pretest prevalence of ${lead} in: ${pop}. Report it as a probability with a chain to a root source.
(2) FINDING LRs: for EACH decisive link below, compute the likelihood ratio from primary data.
${links.map((l) => `  - ${l.finding}  (${l.expected_metric})`).join('\n')}

For every source: record content_excerpt (the passage you read, verbatim) and cited_quote (the literal sentence within it). If a source borrows the number, set gives_root_data=false + onward_citation and add a hop fetching that citation; continue to a ROOT source (primary measured number with an N). Compute LR (LR+ = sens/(1-spec); LR- = (1-sens)/spec) and grade verdict (grounded only if it traces to root primary data; direction_only if no usable number; fabricated if unsupported). cited_quote must be a verbatim substring of content_excerpt.`

const ingest = await agent(ingestPrompt, { phase: 'Ingest', label: 'ingest:case', agentType: 'general-purpose', schema: INGEST_SCHEMA })
const derived = await agent(derivePrompt(ingest), { phase: 'Derive', label: 'derive:leading+links', agentType: 'general-purpose', schema: DERIVE_SCHEMA })
const decisive = derived.finding_links.filter((l) => l.decisive)
const spidered = await agent(spiderPrompt(derived.leading_diagnosis, derived.prior_population, (decisive.length ? decisive : derived.finding_links).slice(0, 5)),
  { phase: 'SpiderToRoot', label: 'spider:root', agentType: 'general-purpose', schema: SPIDER_SCHEMA })

return { ingest, derived, spidered }
