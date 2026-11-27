# A repository boundary registry must match its current discovery rule

A closed source-input registry can omit a package root even when its schema and
pinned digest are internally valid. When its purpose is to cover a discovery
surface, validate closure by deriving the eligible roots from the current
source tree and comparing that set with the registered roots; do not rely only
on a remembered package count. When the set changes, update the registry digest
and every fixture that pins it, then run the full source-sensitive corpus tests
with all registered input paths materialized.

category: CI & GitHub Actions
