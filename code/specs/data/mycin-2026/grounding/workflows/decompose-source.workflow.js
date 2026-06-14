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
const EMBEDDED = [
  'https://pubmed.ncbi.nlm.nih.gov/15509818/',
  'https://www.ncbi.nlm.nih.gov/books/NBK470351/',
  'https://pmc.ncbi.nlm.nih.gov/articles/PMC12662076/',
  'https://pmc.ncbi.nlm.nih.gov/articles/PMC5995389/',
  'https://academic.oup.com/ofid/article/3/4/ofw206/2593338',
  'https://www.ncbi.nlm.nih.gov/books/NBK562176/',
  'https://www.cdc.gov/pinkbook/hcp/table-of-contents/chapter-8-haemophilus-influenzae.html',
  'https://www.ncbi.nlm.nih.gov/books/NBK470553/',
]
const sources = (args && args.sources) || EMBEDDED.map((u) => ({ source_id: u, resolved_url: u }))

const objs = await pipeline(
  sources,
  (s) => agent(
    `Fetch this source and DECOMPOSE it (nothing on blind trust). WebFetch the URL and read it. ` +
    `Extract the KEY factual claims it makes about bacterial-meningitis ORGANISM EPIDEMIOLOGY (which pathogens, what proportions, in which populations) and CSF GRAM-STAIN MORPHOLOGY. ` +
    `For EACH claim give a short normalized \`text\` and a VERBATIM \`byte_quote\` copied exactly from the fetched page (never paraphrase or invent — if you cannot fetch the page, return an empty claims array). ` +
    `Also list in \`cites\` any source THIS page attributes a figure to (e.g. "As per Thigpen et al." → "Thigpen et al. NEJM 2011") — that is the recursion frontier. ` +
    `URL: ${s.resolved_url}`,
    { schema: SCHEMA, label: `decompose:${(s.resolved_url || '').slice(0, 48)}`, phase: 'Decompose' }
  ).then((o) => o ? { ...o, source_id: s.source_id, resolved_url: s.resolved_url } : null)
)

return objs.filter(Boolean)
