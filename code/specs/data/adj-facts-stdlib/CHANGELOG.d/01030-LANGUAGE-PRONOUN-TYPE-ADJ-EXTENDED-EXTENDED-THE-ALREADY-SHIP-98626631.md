- `language/pronoun-type.adj` (extended) — extended the already-shipped `pronoun_type(type,
  description)` table from 3 to 4 rows. Added `distributive_pronoun` -> `refers_to_nouns_as_
  individual_elements_of_larger_groups`, quoted verbatim from the same already-cited Grammarly
  "Pronouns: Definition and Examples" page. Discovered via a careful WebFetch re-check of the
  page's reflexive/intensive/possessive/reciprocal/distributive material the header had always
  named but never fully investigated — two other candidates (intensive_pronoun, reciprocal_pronoun)
  were investigated and REJECTED after multiple WebFetch passes surfaced inconsistent/non-defining
  sentences for them (intensive's own sentence is a comparison that never states its actual
  function; reciprocal's own sentence names a closed word-list rather than a function), while
  distributive_pronoun's sentence was confirmed byte-identical across four separate fetches and
  matches the table's existing "type refers to/is X" pattern. New e2e test
  `pronoun_type_extension_recalls_the_newly_added_distributive_pronoun`, alongside the 3
  pre-existing tests. No new manifest objective (same library, same objective, 168 total unchanged).
