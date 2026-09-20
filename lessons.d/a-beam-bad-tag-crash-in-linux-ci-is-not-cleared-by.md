---
category: Compiler / VM / language pipeline
---

# A BEAM bad-tag crash in Linux CI is not cleared by a passing Windows probe

PR15780 push CI run35508980800 job106074353057 failed portable_text_stdout_brainfuck_beam_raw_bytes after output byte59 with Erlang size_object: bad tag for 0x0 and exit134. The focused three-program test passed on Windows at the same head; that does not establish Linux correctness or justify ignoring the failure. Retrieve completed job logs through the jobs/logs API when gh run view with log-failed refuses an unfinished workflow. Inspect GC root counts and imported-call register clobbering before deciding a rerun is appropriate. Existing putchar uses test_heap with the entire preallocated X-register range, whereas the input_str path documents compacting real roots before GC; this is a hypothesis requiring a bounded contract and deterministic execution proof, not a confirmed fix.
