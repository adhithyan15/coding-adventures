## 0.354.0 - Runtime real conditional provenance

Runtime-real formatter provenance now intersects across conditional statement
exits. A slot remains printable only when every reachable branch proves it;
one-sided reassignment, calls without replacement, and loops still fail closed.
