## Unreleased — panic-template anchor rule and hardened denylist (ADL-001a security review)

- `check_panic_template`: a template must begin with at least 8 ASCII alphanumerics before its first `{}` hole
  (`{}: {}`, `a{}`, `{} !` and `{}.` are refused); only printable ASCII is allowed (NBSP, zero-width, bidi
  and U+2028/2029 refused); the compiler-generated denylist is matched after whitespace normalisation and
  with holes removed, and now covers Java, Go, Python and JS/TS wordings, `assertion failed`, `out of memory`,
  `Result::unwrap`/`Option::unwrap`. The lifter check (ADL-032/033) is the long-term control.
- `ModelModule` has private fields and `ModelModule::new(&file, &checked)` reads the profile from the parsed
  file; `validate_realizations` refuses duplicate model module names (W0806).
- The algorithm-profile word scan starts at the module name, so `warrant state v1 profile algorithm` is W0805.
- Corpus: new negative cases for each; `bind-pos-realizes-with-format-hole` now has a long enough anchor.
