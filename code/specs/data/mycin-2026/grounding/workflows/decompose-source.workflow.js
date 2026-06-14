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
// The most recent grounding frontier (G3 dose sources). Pass args.sources to override;
// earlier organism-id epidemiology/morphology/host-factor sources are already in the CAS.
const EMBEDDED = [
  'https://academic.oup.com/cid/article/39/9/1267/402080',
  'https://dailymed.nlm.nih.gov/dailymed/fda/fdaDrugXsl.cfm?setid=8351aa37-552d-471d-b293-c564dcb6ec29',
  'https://pmc.ncbi.nlm.nih.gov/articles/PMC2760093/',
  'https://www.academia.edu/1171456/Initial_management_of_acute_bacterial_meningitis_in_adults_summary_of_IDSA_guidelines',
  'https://dailymed.nlm.nih.gov/dailymed/fda/fdaDrugXsl.cfm?setid=7d74dfa6-0468-43ad-ad7f-dd90d9ae7706',
  'https://dailymed.nlm.nih.gov/dailymed/fda/fdaDrugXsl.cfm?setid=9a105eaf-ee77-4016-beeb-d425a5565db2&type=display',
]
const sources = (args && args.sources) || EMBEDDED.map((u) => ({ source_id: u, resolved_url: u }))

const objs = await pipeline(
  sources,
  (s) => agent(
    `Fetch this source and DECOMPOSE it (nothing on blind trust). WebFetch the URL and read it. ` +
    `Extract the KEY factual claims it makes about bacterial-meningitis ORGANISM EPIDEMIOLOGY (which pathogens, what proportions, in which populations), HOST / RISK FACTORS (age band, immune status, exposures, recent neurosurgery or CSF device, crowding, rash — which host factor RAISES which organism), CSF GRAM-STAIN MORPHOLOGY, and ANTIBIOTIC DOSING for bacterial meningitis (drug, dose, interval, and whether it is the adult vs pediatric / CNS vs general indication). ` +
    `For EACH claim give a short normalized \`text\` and a VERBATIM \`byte_quote\` copied exactly from the fetched page (never paraphrase or invent — if you cannot fetch the page, return an empty claims array). ` +
    `Also list in \`cites\` any source THIS page attributes a figure to (e.g. "As per Thigpen et al." → "Thigpen et al. NEJM 2011") — that is the recursion frontier. ` +
    `URL: ${s.resolved_url}`,
    { schema: SCHEMA, label: `decompose:${(s.resolved_url || '').slice(0, 48)}`, phase: 'Decompose' }
  ).then((o) => o ? { ...o, source_id: s.source_id, resolved_url: s.resolved_url } : null)
)

return objs.filter(Boolean)
