## [0.2.0] - 2026-11-24

### Changed (breaking for some callers)

- Every gate is `[[nodiscard]]`: discarding a result is now a warning (an error
  under `-Werror`).
- All symbols moved into the inline namespace `logic_gates::v0_2`.
  `logic_gates::x` and `using namespace logic_gates` still work; forward
  declarations of `logic_gates::x` outside the header, and ABI/symbol names,
  change.
- Braced-init-list arguments (`and_gate({true}, 1)`, `{c}`, even `{1}`) are now
  deleted; before, they bypassed the deleted overloads and were accepted.
- Internal helpers renamed (`lg_detail::lg_check`/`lg_reject`) and parameters
  prefixed `lg_`, called through parentheses, so user macros (`check`, `min`,
  `a`, ...) defined before the include cannot break the header.

### Fixed / documented

- Header and README no longer claim "no allocation" unconditionally: the
  rejection path allocates (std::string, exception object; may throw
  std::bad_alloc).
- Documented that deleted overloads guard direct calls only; indirect calls
  convert in the caller. `std::function<int(int,int)> g = and_gate` does not
  compile. Gates called from a `noexcept` caller terminate on invalid input.

### Tests / tooling

- `tools/run.sh`: added `-O2` build+run, macro-hygiene TU
  (`tests/macro_hygiene_test.cpp`), 483 compile-must-fail snippets per
  compiler (deleted overloads x every gate and position, braced lists, invalid
  constant expressions, discarded results, `std::function`) with a positive
  control, a noexcept-terminates child-process test, and an indirect
  function-pointer type-confusion matrix (enums, wchar_t, char16_t, char32_t,
  unsigned short, `operator int`, 2^32+1, 2^64-1).
- The ASan/UBSan run is now mandatory (failure when unavailable) unless
  `LG_ALLOW_NO_SANITIZER=1`, which prints a loud warning.
- `tools/run.ps1`: `/O2` added (MSVC has no ASan+UBSan combination).
- Internal helpers take no pointers (`lg_check(int which, int value)`); messages byte-identical, verified for every gate/parameter.
- Review follow-ups: negative-compile oracle requires a category-specific diagnostic plus the gate name as a whole word and rejects undeclared-identifier typos; noexcept test requires exit status 134 (SIGABRT); empty braces `and_gate({}, 1)` now deleted; include guard versioned (`LOGIC_GATES_LOGIC_GATES_V0_2_HPP`), `LOGIC_GATES_VERSION` (200) defined and a conflicting version is an `#error`; coverage enforced at 100%; run.ps1 object paths robust to spaces.
