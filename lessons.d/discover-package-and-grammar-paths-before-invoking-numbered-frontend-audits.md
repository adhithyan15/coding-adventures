---
category: Repo policy / workflow reminders
---

# Discover package and grammar paths before invoking numbered frontend audits

The Ruby audit guessed `ruby-native-compiler`, `LANG80-native-ruby-vm.md`
and a root-level `ruby.grammar`; each read failed. Discovery showed the actual
package `ruby-iir-compiler`, spec `LANG80-ruby-direct-iir-pilot.md` and grammar
`code/grammars/ruby/ruby.grammar`. A later lesson lookup also guessed a title
instead of using the filename returned by discovery.
Run `rg --files` or `git ls-tree` first and pass the returned paths directly to
reads. Do not combine discovery with a second guessed read in the same command:
the discovery output must inform the next operation.
