---
category: Compiler / VM / language pipeline
---

# Object.const_set takes only an upper-case first letter, so a Ruby backend must map every SIR class name to a constant at every position that names one

**What went wrong.** The Ruby backend defines a class with
`Object.const_set(:<name>, Class.new)`.  Its scan accepted any identifier as
a class name, but Ruby constants must start upper-case, so Python's `class
dog:` (and the Python frontend's invented `__param_a`) died at run time with
`wrong constant name dog (NameError)` (#18778, found writing #18771's
tests).

**Fix.** One map, `ruby_const`, renames each `::` segment: an upper-case
name passes through (built-in classes stay Ruby's own), a lower-case or `_`
name gets the `SirCl` prefix, and an upper-case name already starting with
the `SirC` marker gets `SirCv`.  It is one-to-one by construction.  Every
position that names a class goes through it: `const_set`, the superclass,
`sir_new`, `define_method`, `define_singleton_method`, `public_send` on a
class, `super`, `include`/`extend`, a `Const` read and a `rescue` clause.
The runtime's `sir_show_class_as` puts the source name back for `name`,
`to_s`, `inspect` and Ruby's own messages.

**Do differently.** When a target language restricts a name class (case,
reserved words, characters), map the IR name once, in one helper, and grep
for every emission site of that kind of name; a validator that only checks
the characters is an injection guard, not a correctness check.  Test with a
name the source language allows and the target does not.
