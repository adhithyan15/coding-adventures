# 00030 - LGE-S3b: java, typescript and python lane threat lists

* **Added** `lanes/java.json` (43 threats, 43 rows), `lanes/typescript.json` (44 threats, 44 rows) and
  `lanes/python.json` (45 threats, 45 rows), each covering all 29 registry classes plus the lane's own
  threats. Every row is `open`: no lane evidence is `live` yet (VER15a 5.2).
* **Conformed to the post-LGE-S3-FIX spec:**
  * Java: the Javadoc/doclint item is now `source-ref:java.javadoc-documents-thrown-exceptions`
    (`claimed`, with `ref`, in no route); CWE-1321 carries the section 5.5 `language_exclusion`
    (`signed_off: false`) and stays open.
  * Python and TypeScript: CWE-94 and CWE-1321 now use the v2 subsets `no_dynamic_eval_reachable` and
    `no_computed_key_writes_to_shared_objects` with a planned analyzer rule plus a planned seeded
    negative-control `lane-corpus` (planned item `LGE-A-PYTHON` / `LGE-A-TYPESCRIPT`); the old
    discharge-route workarounds are removed.
  * `becomes_when` holds identifiers only (free text moved into `reason`); planned items use the
    backlog's exact case (`LGE-L-python`, `LGE-L-java`).
  * Rows whose only evidence cannot earn a label now say `reached: none` (rule in 5.3).
* **Unresolved planned ids** (no backlog row yet; a separate backlog PR adds them): `LGE-A-PYTHON`,
  `LGE-A-TYPESCRIPT`, `LGE-PY-HARDEN`, `LGE-TS-HARDEN`.
