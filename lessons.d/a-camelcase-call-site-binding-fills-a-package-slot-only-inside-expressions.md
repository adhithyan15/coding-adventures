---
category: Mosaic compiler pipeline
---

# A camelCase call-site binding fills a package slot only inside expressions, never a slot: prop

`build_binding_map` in `mosaic-package-resolver` files each call-site binding
under two keys: its own name, and that name in camelCase. That looks as if
`bodyValue:` and `body-value:` are interchangeable at a call site. They are
not.

- A `slot: body-value` **prop** is looked up by its kebab-case name. A
  `bodyValue:` binding never reaches it, so the prop stays unbound.
- Only identifiers **inside an expression** are looked up in camelCase, so
  they do match.

The first version of `SlotBindingMissing` (E6 F1) compared names in
camelCase "to match substitution". It would have accepted `bodyValue:` as
binding `body-value`, and left exactly the unbound prop the check exists to
catch. A test that asserted the substituted value caught it.

What to do: when a check must agree with substitution, test it against
substitution's output (the substituted prop value), not against a reading of
the map. In this case, a binding counts only under the slot's declared
name.
