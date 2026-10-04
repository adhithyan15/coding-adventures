## 0.352.0 - Runtime real scalar output

Local ALGOL real scalar variables whose latest straight-line assignment is a
direct runtime real procedure result now print through the shared portable
six-significant-digit formatter. Reassignment, control flow, calls, and
composed dynamic real expressions invalidate or bypass that provenance.
