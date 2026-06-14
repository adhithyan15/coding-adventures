// decompose-source.workflow.js — the recursive source-grounding spider.
//
// "Nothing on blind trust": a grounded fact cites a source with a byte-quote, but a
// quote can be cherry-picked or misread. So we FETCH the cited source itself and
// DECOMPOSE it into byte-provenanced claims — a CAS *source object* — and capture the
// sources IT cites (the recursion frontier). The driver (ground_sources.py) then
// commits each source object to cas/sources/ and VERIFIES every fact's byte-quote
// against its decomposed source (does the source actually say what the fact implies?).
//
// Run via the Workflow tool. It consumes `args.sources` (the worklist emitted by
// `ground_sources.py --list`); the EMBEDDED list is the organism-id frontier used the
// first time (args delivery can be flaky, so we fall back to it). Write the returned
// array to grounding/source-objects.json, then run `python3 ground_sources.py`.
//
// One agent per source (pipeline, single decompose pass). On transient rate limits,
// resume — cached agents return instantly:
//   Workflow({scriptPath: ".../decompose-source.workflow.js", resumeFromRunId: "<id>"})

export const meta = {
  name: 'decompose-source',
  description: 'Fetch each cited source and decompose it into byte-provenanced claims (a CAS source object), capturing child citations for recursion — nothing on blind trust',
  phases: [{ title: 'Decompose', detail: 'one agent per source: WebFetch + extract verbatim byte-quoted claims + child citations' }],
}

const SCHEMA = {
  type: 'object',
  required: ['source_id', 'resolved_url', 'title', 'claims', 'cites'],
  properties: {
    source_id: { type: 'string' },
    resolved_url: { type: 'string' },
    title: { type: 'string' },
    claims: {
      type: 'array',
      description: 'the key factual claims the source makes about bacterial-meningitis organism epidemiology / Gram-stain morphology',
      items: {
        type: 'object', required: ['id', 'text', 'byte_quote'],
        properties: {
          id: { type: 'string', description: 'short slug, e.g. c1' },
          text: { type: 'string', description: 'the claim in a short normalized sentence' },
          byte_quote: { type: 'string', description: 'VERBATIM span copied from the fetched page that states it — never paraphrased or fabricated' },
        },
      },
    },
    cites: {
      type: 'array', items: { type: 'string' },
      description: 'sources THIS source attributes a figure to (e.g. "Thigpen et al. NEJM 2011") — the recursion frontier; identifier or citation string each',
    },
  },
}

// The organism-id citation frontier (output of `ground_sources.py --list`).
// The most recent grounding frontier (G2 host-factor sources). Pass args.sources to
// override; earlier organism-id epidemiology/morphology sources are already in the CAS.
const EMBEDDED = [
  'https://pmc.ncbi.nlm.nih.gov/articles/PMC6508726/',
  'https://www.merckmanuals.com/professional/pediatrics/infections-in-neonates/neonatal-bacterial-meningitis',
  'https://pmc.ncbi.nlm.nih.gov/articles/PMC5121369/',
  'https://pmc.ncbi.nlm.nih.gov/articles/PMC2995464/',
  'https://pubmed.ncbi.nlm.nih.gov/23446215/',
  'https://www.ncbi.nlm.nih.gov/books/NBK549849/',
  'https://pmc.ncbi.nlm.nih.gov/articles/PMC5405091/',
  'https://pubmed.ncbi.nlm.nih.gov/16451418/',
  'https://www.who.int/news-room/fact-sheets/detail/listeriosis',
  'https://pmc.ncbi.nlm.nih.gov/articles/PMC3372798/',
  'https://academic.oup.com/cid/article/64/6/e34/2996079',
  'https://www.cdc.gov/mmwr/preview/mmwrhtml/rr4907a2.htm',
  'https://pmc.ncbi.nlm.nih.gov/articles/PMC3719428/',
]
const sources = (args && args.sources) || EMBEDDED.map((u) => ({ source_id: u, resolved_url: u }))

const objs = await pipeline(
  sources,
  (s) => agent(
    `Fetch this source and DECOMPOSE it (nothing on blind trust). WebFetch the URL and read it. ` +
    `Extract the KEY factual claims it makes about bacterial-meningitis ORGANISM EPIDEMIOLOGY (which pathogens, what proportions, in which populations), HOST / RISK FACTORS (age band, immune status, exposures, recent neurosurgery or CSF device, crowding, rash — which host factor RAISES which organism), and CSF GRAM-STAIN MORPHOLOGY. ` +
    `For EACH claim give a short normalized \`text\` and a VERBATIM \`byte_quote\` copied exactly from the fetched page (never paraphrase or invent — if you cannot fetch the page, return an empty claims array). ` +
    `Also list in \`cites\` any source THIS page attributes a figure to (e.g. "As per Thigpen et al." → "Thigpen et al. NEJM 2011") — that is the recursion frontier. ` +
    `URL: ${s.resolved_url}`,
    { schema: SCHEMA, label: `decompose:${(s.resolved_url || '').slice(0, 48)}`, phase: 'Decompose' }
  ).then((o) => o ? { ...o, source_id: s.source_id, resolved_url: s.resolved_url } : null)
)

return objs.filter(Boolean)
