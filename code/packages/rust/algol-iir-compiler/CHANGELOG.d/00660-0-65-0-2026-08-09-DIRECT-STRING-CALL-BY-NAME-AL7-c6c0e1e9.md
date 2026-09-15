## 0.65.0 — 2026-08-09 — direct string call-by-name (AL7)

Direct `string` name formals now use the existing specialised sibling path,
including nested forwarding through lexical aliases. Reads re-evaluate the
caller string binding and literal or variable writes preserve normal
string-copy semantics before storing through that binding. Coverage includes a
forwarded formal whose inner parameter deliberately shadows the outer spelling
and rejection of a written literal actual.

