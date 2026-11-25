# 00010 - LGE-S3-FIX: planned-item casing, source-ref evidence, CWE-1321 whitelist, eval subset

Driven by spec gaps found while writing the java, typescript and python lane threat lists.

* **Schema `planned_item`** pattern widened to `^[A-Z][A-Za-z0-9-]*$`; VER15a section 9 rule 13
  now matches backlog ids case-insensitively and treats `<lane>` rows (`LGE-L-<lane>`,
  `LGE-R-<lane>`) as templates matching any lane id.
* **New evidence kind `source-ref`** (VER15a section 5.2, schema enum): a package file:line or
  documentation claim. Never `live`, requires `ref`, never in a route list, contributes no label.
  Validator rule 15.
* **Section 5.5 whitelist:** `(lang, 1321)` for cpp, java, kotlin, csharp, fsharp, go, rust,
  haskell, ocaml, swift, dart, elixir, each `signed_off: false` (owner signs). python, ruby, perl,
  lua and typescript are deliberately excluded from the list.
* **Registry v2 additive deltas** (VER15a section 1.3; `weakness-registry-v1.json` untouched):
  CWE-94 `safe_subsets` += `no_dynamic_eval_reachable`; CWE-1321 `safe_subsets` +=
  `no_computed_key_writes_to_shared_objects` and `applies_to` += python, ruby, perl, lua. The v1
  equality check now allows exactly these deltas.
* **`becomes_when`** schema pattern: a closure id (`kind:slug`) or a backlog id, never prose.
* **Section 5.3:** a row's `reached` must come from that row's own evidence (worked example).
* **`lanes/c.json`:** one `becomes_when` entry changed from a sentence to `LGE-S3b`; still valid.
