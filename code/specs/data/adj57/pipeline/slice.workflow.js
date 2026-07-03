export const meta = {
  name: 'adj57-vertical-slice',
  description: 'ADJ57 thin vertical slice on a fresh case: (L1) ingest a real published case into a byte-COVERING partition of typed facts + reasoned discards; (L2) derive the finding->diagnosis links the rulebook needs from those facts; (L3) spider the decisive links, forcing byte provenance on each source and following citations back to a ROOT source. The parent runs the deterministic coverage check + interns every source into the CAS.',
  phases: [{ title: 'Ingest' }, { title: 'Derive' }, { title: 'SpiderToRoot' }],
}

const INGEST_SCHEMA = {
  type: 'object',
  required: ['source_url', 'ground_truth', 'case_text', 'segments', 'candidate_diagnoses'],
  properties: {
    source_url: { type: 'string' },
    ground_truth: { type: 'string', description: 'final confirmed diagnosis + how established. Held aside from later layers.' },
    case_text: { type: 'string', description: 'the EXACT case-presentation text you decomposed (a self-contained vignette, ~500-1500 chars, up to the diagnostic decision point; no final answer). The segments below MUST concatenate to this string character-for-character.' },
    segments: {
      type: 'array',
      description: 'an ORDERED partition of case_text. Concatenating every segment.text in order must reproduce case_text EXACTLY (no paraphrase, no reordering, no gaps). Each character of case_text belongs to exactly one segment.',
      items: {
        type: 'object',
        required: ['text', 'kind'],
        properties: {
          text: { type: 'string', description: 'the literal substring of case_text (verbatim)' },
          kind: { type: 'string', enum: ['fact', 'discard'] },
          term: { type: 'string', description: 'for kind=fact: the typed interpretation, a snake_case predicate e.g. heart_rate(over_100)' },
          reason: { type: 'string', description: 'for kind=discard: why this span carries no diagnostic fact (e.g. "whitespace", "narrative connective", "patient name — sanitised")' },
        },
      },
    },
    candidate_diagnoses: { type: 'array', items: { type: 'string' }, description: 'the differential the case raises (snake_case)' },
  },
}
const DERIVE_SCHEMA = {
  type: 'object',
  required: ['links'],
  properties: {
    links: {
      type: 'array',
      description: 'the finding->diagnosis links the rulebook needs, derived FROM the extracted facts (not invented). Each is a magnitude that must later be grounded.',
      items: {
        type: 'object',
        required: ['finding', 'target_diagnosis', 'expected_metric', 'decisive'],
        properties: {
          finding: { type: 'string', description: 'a fact term from the ingested IR' },
          target_diagnosis: { type: 'string' },
          expected_metric: { type: 'string', description: 'LR+ | LR- | prevalence | OR' },
          decisive: { type: 'boolean', description: 'is this link load-bearing for the top of the differential' },
          rationale: { type: 'string' },
        },
      },
    },
  },
}
const SPIDER_SCHEMA = {
  type: 'object',
  required: ['grounded_links'],
  properties: {
    grounded_links: {
      type: 'array',
      items: {
        type: 'object',
        required: ['finding', 'target_diagnosis', 'computed_lr', 'verdict', 'chain'],
        properties: {
          finding: { type: 'string' },
          target_diagnosis: { type: 'string' },
          computed_lr: { type: 'number' },
          lr_formula: { type: 'string' },
          verdict: { type: 'string', enum: ['grounded', 'direction_only', 'fabricated'] },
          reached_root: { type: 'boolean', description: 'did the chain terminate in a primary-data (root) source' },
          chain: {
            type: 'array',
            description: 'ordered hops from the first source down to the ROOT source',
            items: {
              type: 'object',
              required: ['hop', 'source_url', 'content_excerpt', 'cited_quote'],
              properties: {
                hop: { type: 'number' },
                source_url: { type: 'string' },
                source_title: { type: 'string' },
                content_excerpt: { type: 'string', description: 'the passage you actually read from this source (a few sentences to a few paragraphs) — this is interned into the CAS verbatim' },
                cited_quote: { type: 'string', description: 'the LITERAL sentence(s) within content_excerpt that bear on the claim (must be a verbatim substring of content_excerpt)' },
                gives_root_data: { type: 'boolean', description: 'does this source state the PRIMARY measured number (sens/spec/OR/prevalence with an N), vs borrowing it' },
                onward_citation: { type: 'string', description: 'if it borrows the number, the citation to follow next (empty if this is the root)' },
              },
            },
          },
        },
      },
    },
  },
}

const ingestPrompt = `Find ONE real published clinical case (prefer open-access PMC) with a documented final diagnosis, and decompose its PRESENTATION into a byte-covering IR.

RULES:
1. Choose a self-contained case-presentation passage (~500-1500 characters), up to the diagnostic decision point. Do NOT include the final answer/discussion. Copy it VERBATIM into case_text.
2. Partition case_text into an ordered list of segments. CRITICAL: concatenating every segment.text in order must reproduce case_text EXACTLY, character for character — every space, comma, and digit. No paraphrasing, no reordering, no omission.
3. Each segment is either a fact (a diagnostically meaningful span — give its typed snake_case term) or a discard (carries no fact — give a reason, e.g. "whitespace", "narrative connective"). Punctuation/whitespace between facts is usually a discard.
4. Also return the candidate_diagnoses the case raises, the source_url, and the held-aside ground_truth.

Self-check before returning: mentally concatenate your segments and confirm they equal case_text byte-for-byte.`

const derivePrompt = (o) => `From the extracted facts of a case, derive the finding->diagnosis links a Bayesian rulebook would need. Derive from the FACTS — do not invent links the facts don't support.

CANDIDATE DIAGNOSES: ${JSON.stringify(o.candidate_diagnoses)}
EXTRACTED FACTS (typed terms):
${o.segments.filter((s) => s.kind === 'fact').map((s) => `  - ${s.term}`).join('\n')}

For each load-bearing finding->diagnosis pair, emit a link with the expected metric (LR+/LR-/prevalence/OR) and whether it is decisive for the top of the differential.`

const spiderPrompt = (links) => `You are a byte-provenance spider. For EACH decisive finding->diagnosis link below, find the primary data and COMPUTE the likelihood ratio — and crucially, FOLLOW CITATIONS BACK TO A ROOT SOURCE.

LINKS:
${links.map((l) => `  - ${l.finding} -> ${l.target_diagnosis}  (${l.expected_metric})`).join('\n')}

For each link, build a chain of hops (WebSearch/WebFetch):
- hop 1: the source you first find. Record content_excerpt (the passage you read, verbatim) and cited_quote (the literal sentence within it bearing on the claim).
- if that source BORROWS the number from a study it cites, set gives_root_data=false, give onward_citation, and add another hop fetching that citation.
- continue until you reach a ROOT source that states the PRIMARY measured number (sens/spec/OR/prevalence with an N) — set gives_root_data=true, reached_root=true.
Then COMPUTE the LR (LR+ = sens/(1-spec); LR- = (1-sens)/spec; prevalence as a probability) and grade verdict (grounded only if it traces to root primary data; direction_only if no usable number; fabricated if unsupported).

The content_excerpt of every hop will be interned verbatim into a content-addressed store, and cited_quote must be a verbatim substring of it.`

const ingest = await agent(ingestPrompt, { phase: 'Ingest', label: 'ingest:case', agentType: 'general-purpose', schema: INGEST_SCHEMA })
const derived = await agent(derivePrompt(ingest), { phase: 'Derive', label: 'derive:links', agentType: 'general-purpose', schema: DERIVE_SCHEMA })
const decisive = derived.links.filter((l) => l.decisive).slice(0, 3)
const spidered = await agent(spiderPrompt(decisive.length ? decisive : derived.links.slice(0, 3)), { phase: 'SpiderToRoot', label: 'spider:root', agentType: 'general-purpose', schema: SPIDER_SCHEMA })

return { ingest, derived, decisive, spidered }
