- A condition of `( globalThis.pwned = true )` rendered `aria-pressed="false"`
  and did not run.

### Fixed -- list story fixtures hydrate as JSON arrays (#15428)

`json_value_for_fixture` turns a list fixture into a JSON array in
`main.js`'s fallback props. Before this, a list-driven story rendered with
no rows. A shape that does not match the slot keeps the generated `[]`.

### Fixed -- `HostButton`'s accessible name was never emitted, and an indexed label rendered as literal braces (#15426)

`emit_host_button` never read `a11y-label`, and reported no degradation for
it, so every authored button name was lost on this backend. The literal,
slot, keyword and expression forms now lower to `aria-label`. A literal is
escaped when emitted; the dynamic forms become a `{{path}}` placeholder that
the runtime fills with `escapeHtml`.

The expression form exposed a second, older defect. The runtime fills only
placeholders matching `[A-Za-z0-9_.-]+`, but `label : ( row[0] )` was
emitted as `{{row [ 0 ]}}`, which does not match, so the page showed the
braces. The `Text` content path had the same bug. A new `mustache_path`
rewrites a plain data path (`row [ 16 ]` becomes `row.16`, `a.b[2]` becomes
`a.b.2`) into the dotted form `readPath` walks. Anything else (operators,
variable indices) is not a path:

