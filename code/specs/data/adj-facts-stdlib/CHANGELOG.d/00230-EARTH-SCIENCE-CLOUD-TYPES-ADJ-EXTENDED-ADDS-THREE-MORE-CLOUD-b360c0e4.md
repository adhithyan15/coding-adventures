- `earth-science/cloud-types.adj` (extended) -- adds three more clouds
  (cirrostratus->high, cirrocumulus->high, altocumulus->middle) to the
  already-shipped four-row table (cirrus/altostratus/stratus/cumulus), all drawn
  directly from this table's OWN already-quoted `source` sentence -- "The three
  main types of high clouds are cirrus, cirrostratus, and cirrocumulus. The two
  main type of mid-level clouds are altostratus and altocumulus..." -- which had
  already named all seven clouds even though only four were tabled, so extending
  from four rows to seven required no new WebFetch, just reading the span already
  captured in the file. Distinct from the sibling `meteorology/cloud-type.adj`,
  which answers a completely different question (what WEATHER does a cloud's
  presence indicate?) from the SAME NWS page's separate weather-indication
  sentences, not the altitude-deck sentence this table uses -- the two tables do
  not overlap even though `altocumulus` happens to be named (as an abstention
  target, for an unrelated reason) in both files. Extended e2e test
  `facts_clouds_e2e.rs` (now 2 tests: the original altitude-recall test plus a
  new test binding both newly added rows).
