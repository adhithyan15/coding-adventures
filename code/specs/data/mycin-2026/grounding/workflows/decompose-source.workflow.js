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

// The pending-citation frontier — every cited source not yet in the CAS (output of
// `ground_sources.py --list`), decomposed to clear the ledger's pending column.
const EMBEDDED = [
  "https://academic.oup.com/cid/article/52/5/e103/388285",
  "https://www.ncbi.nlm.nih.gov/books/NBK482367/",
  "https://pmc.ncbi.nlm.nih.gov/articles/PMC5644167/",
  "https://www.aafp.org/pubs/afp/issues/2011/1001/p771.html",
  "https://pubmed.ncbi.nlm.nih.gov/15701325/",
  "https://pmc.ncbi.nlm.nih.gov/articles/PMC10031580/",
  "https://pmc.ncbi.nlm.nih.gov/articles/PMC2708523/",
  "https://pmc.ncbi.nlm.nih.gov/articles/PMC12217021/",
  "https://pubmed.ncbi.nlm.nih.gov/6696301/",
  "https://pmc.ncbi.nlm.nih.gov/articles/PMC5880328/",
  "https://www.cdc.gov/std/treatment-guidelines/penicillin-allergy.htm",
  "https://dailymed.nlm.nih.gov/dailymed/drugInfo.cfm?setid=3c442cca-47f5-48e5-b718-c09413911687",
  "https://dailymed.nlm.nih.gov/dailymed/lookup.cfm?setid=793c75eb-dd65-4b6d-ac46-6e57ce1334f7",
  "https://dailymed.nlm.nih.gov/dailymed/drugInfo.cfm?setid=65e5f2ee-934c-40d7-967c-00d085f84ffd",
  "https://dailymed.nlm.nih.gov/dailymed/fda/fdaDrugXsl.cfm?setid=f604d399-fded-4f4f-8efe-af558ed07b9d",
  "https://dailymed.nlm.nih.gov/dailymed/fda/fdaDrugXsl.cfm?setid=99e523d8-9bde-43cb-8434-497015e5dcbd",
  "https://pubmed.ncbi.nlm.nih.gov/15306996/",
  "https://pmc.ncbi.nlm.nih.gov/articles/PMC8785473/",
  "https://www.cambridge.org/core/journals/epidemiology-and-infection/article/burden-of-communityonset-bloodstream-infection-a-populationbased-assessment/4C3033D11C42B1D2B68B3580C283DB9A",
  "https://pmc.ncbi.nlm.nih.gov/articles/PMC10699684/",
  "https://pubmed.ncbi.nlm.nih.gov/3284952/",
  "https://pmc.ncbi.nlm.nih.gov/articles/PMC10350481/",
  "https://www.ncbi.nlm.nih.gov/books/NBK430891/",
  "https://academic.oup.com/cid/article/49/1/1/369414",
  "https://academic.oup.com/cid/article/72/7/1211/5836974",
  "https://pmc.ncbi.nlm.nih.gov/articles/PMC12902366/",
  "https://pmc.ncbi.nlm.nih.gov/articles/PMC4451395/",
  "https://pmc.ncbi.nlm.nih.gov/articles/PMC12258469/",
  "https://pmc.ncbi.nlm.nih.gov/articles/PMC3892635/",
  "https://pubmed.ncbi.nlm.nih.gov/18691485/",
  "https://www.ebi.ac.uk/europepmc/webservices/rest/PMC1584240/fullTextXML",
]
const sources = (args && args.sources) || EMBEDDED.map((u) => ({ source_id: u, resolved_url: u }))

const objs = await pipeline(
  sources,
  (s) => agent(
    `Fetch this source and DECOMPOSE it (nothing on blind trust). WebFetch the URL and read it. ` +
    `Extract the KEY factual claims it makes about bacterial-infection diagnosis + antibiotic treatment: ORGANISM EPIDEMIOLOGY (which pathogens, what proportions, in which populations — meningitis, UTI, or bloodstream/bacteremia), HOST / RISK FACTORS (age band, immune status, exposures, device/neurosurgery, crowding, rash, neutropenia, injection drug use — which factor RAISES which organism), GRAM-STAIN / URINALYSIS MORPHOLOGY, ANTIBIOTIC DOSING (drug, dose, interval, adult vs pediatric / CNS vs general indication), CONTRAINDICATIONS + INTERACTIONS (allergy cross-reactivity, pregnancy, nephrotoxicity, QT), and BLOODSTREAM-INFECTION SOURCE→ORGANISM associations (which portal of entry predicts which organism). ` +
    `For EACH claim give a short normalized \`text\` and a VERBATIM \`byte_quote\` copied exactly from the fetched page (never paraphrase or invent — if you cannot fetch the page, return an empty claims array). ` +
    `Also list in \`cites\` any source THIS page attributes a figure to (e.g. "As per Thigpen et al." → "Thigpen et al. NEJM 2011") — that is the recursion frontier. ` +
    `URL: ${s.resolved_url}`,
    { schema: SCHEMA, label: `decompose:${(s.resolved_url || '').slice(0, 48)}`, phase: 'Decompose' }
  ).then((o) => o ? { ...o, source_id: s.source_id, resolved_url: s.resolved_url } : null)
)

return objs.filter(Boolean)
