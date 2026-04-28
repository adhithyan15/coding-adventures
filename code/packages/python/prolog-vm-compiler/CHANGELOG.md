# Changelog

## Unreleased

- Add end-to-end stress coverage for recursive search, modules, DCGs,
  arithmetic, collections, dynamic initialization, named answers, and expansion.
- Add named answer helpers for source-level query results.
- Add initialized query helpers that run compiled initialization slots before
  executing source queries.

## 0.1.0

- Add the first Prolog-to-Logic-VM compiler package.
- Compile loaded Prolog clauses, dynamic declarations, initialization goals,
  and source queries into `logic-instructions`.
- Add helpers for loading compiled instruction streams into `logic-vm` and
  running source-level queries.
