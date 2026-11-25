# swf-conformance-corpus

Clean-room synthetic `.swf` binary encoder (`SwfBitWriter` + `SwfBuilder`), deterministic conformance corpus runner (`CorpusRunner`), pinned canonical SHA-256 frame and audio digest verifier (`verify_corpus_digests`), and adversarial security test suite (`SWF08`).

Part of the **Coding Adventures** Flash Player suite (`SWF00` §3.8, §7–§9).

---

## Corpus Matrix (`SWF00` §3.8)

| Corpus ID | Slug | Covered Subsystems |
|---|---|---|
| `CorpusId::Shapes` | `corpus-01-shapes` | `DefineShape1..4` straight edges, quadratic Bézier curves, linear/radial gradients, and `EvenOdd` inner-island hole cutouts (`SWF02`) |
| `CorpusId::Timeline` | `corpus-02-timeline` | `CWS` zlib decompression (`SWF01`), multi-frame `PlaceObject2`/`RemoveObject2`, `DefineSprite` nested clips, `gotoAndPlay`/`gotoAndStop`, `PatchSummary` deltas (`SWF05`) |
| `CorpusId::Buttons` | `corpus-03-buttons` | `SimpleButton` stage hit-testing and `Up`/`Over`/`Down`/`Over`/`Up` `WindowEvent` transitions (`SWF05`, `SWF07`) |
| `CorpusId::Avm1Math` | `corpus-04-avm1-math` | `DoAction` AVM1 arithmetic, variable scoping, branches, `DefineFunction`/`DefineFunction2`, deterministic `trace_output` (`SWF04`) |
| `CorpusId::Avm2Oop` | `corpus-05-avm2-oop` | `DoABC` AVM2 constant pool, class hierarchy, methods, exception table, `vm-core` execution (`SWF04`, `LANG02`) |
| `CorpusId::StreamingAudio` | `corpus-06-streaming-audio` | Frame-synced `SoundStreamHead2` + 4-bit Flash ADPCM `SoundStreamBlock` and `DefineSound`/`StartSound` PCM (`SWF06`) |

---

## Adversarial Security Corpus (`SWF00` §7)

- **CWE-409**: `CWS` decompression bomb exceeding `50:1` ratio beyond 1 MB (`SwfError::DecompressionBombDetected`).
- **CWE-400**: Infinite AVM loop (`PlayerHostError::ScriptTimeout` + audit event) and deep sprite nesting `> 64` (`DisplayListError::HierarchyDepthExceeded`).
- **CWE-125**: Truncated tag length, invalid AVM branch target (`InvalidBranchTarget`), and operand stack underflow (`StackUnderflow`).
- **CWE-1321**: `Object.prototype` pollution attempt (`LifterError::PrototypePollutionBlocked` + audit event).
- **CWE-78 / CWE-94**: `fscommand("exec")` permanently disabled (`BlockedNoOp`) and dangerous URI schemes (`javascript:`, `file:`, `data:`, `vbscript:`) rejected (`BlockedUriScheme`).

---

## Usage

```rust
use swf_conformance_corpus::{run_all_corpus_cases, run_corpus_case, CorpusId, CorpusRunner};

let shapes_report = run_corpus_case(CorpusId::Shapes)?;
assert!(shapes_report.digests_verified);

let all_reports = run_all_corpus_cases()?;
assert_eq!(all_reports.len(), 6);

let adv_outcomes = CorpusRunner::runner_new().runner_execute_all_adversarial()?;
assert!(adv_outcomes.iter().all(|o| o.blocked));
```
