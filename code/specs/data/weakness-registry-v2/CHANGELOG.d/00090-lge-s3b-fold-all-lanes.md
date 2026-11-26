# 00090 - LGE-S3b: fold all 18 lane threat lists

* **Added** the 14 remaining lane files beside java/typescript/python/c, so `lanes/` now holds all 18
  (c, cpp, csharp, dart, elixir, fsharp, go, haskell, java, kotlin, lua, ocaml, perl, python, ruby, rust,
  swift, typescript). Newly folded: ruby (43 threats), go (43), cpp (54), kotlin (46), csharp (46),
  fsharp (46), lua (50), elixir (42), perl (46), haskell (41), swift (46), dart (47), ocaml (42),
  rust (44). Every row in every lane is `open`; no lane evidence is `live` (VER15a 5.2).
* **Merged**: the cpp branch's amendments to VER15a (5.5, 5.6.1, rules 7 and 10) are kept; the backlog
  rows added by several lanes are kept once each. `LGE-DOTNET-SDK` was added by both the C# and F#
  lanes (priorities 4 and 2): one row remains, priority 3, with the union of the descriptions.
* **Renumbered** colliding changelog shards (kotlin 00040 -> 00070, swift 00040 -> 00080).
* **Citation drift repaired** after merging origin/main: the `.github/workflows/ci.yml` line numbers
  for the .NET SDK pin (4989-4992 -> 5010-5013), the Ruby setup step (1415-1418 -> 1420-1423) and
  dependency-policy check (906 -> 911), the Rust toolchain step (4443 -> 4464) and the Go build tool's
  `-clippy` steps (6329 -> 6350, 6375 -> 6396), each confirmed against the cited text.
* **Checked**: all 18 files validate against `lane-threat-list.schema.json`; the lane validator reports
  0 errors and 0 unresolved planned or `becomes_when` ids; every `path:line` citation resolves to a file
  with that many lines, except the Kotlin lane's `NaryGatesTest.kt` refs, which cite the local branch
  `claude/kotlin-logic-gates-nary` (not on main) and say so.
