# VLT-PM31 — Audited Local CLI Search

## Status

Normative Phase 1A contract for storage-neutral authenticated search through
the local CLI.

## 1. Purpose and boundary

The application already rebuilds a wipe-on-lock search projection from the
authenticated current catalog and exposes a publish-before-release audited
search boundary. This slice composes it into:

```text
vault-pm [--vault NAME] search QUERY
```

The command searches only approved redacted metadata and returns only the
existing redacted item-list row. It does not persist an index, extend a storage
provider, rank results, search secret fields, resolve conflicts, accept query
files or environment variables, or add collection/tag filters.

## 2. Grammar and query ownership

`search` requires exactly one Unicode positional query and accepts no flags,
extra positionals, stdin input, environment reference, or non-Unicode value.
Missing and extra positionals are grammar errors before authentication.

An exact single query is moved immediately into a wipe-on-drop owner. Its
`Debug` representation is redacted, and the CLI wipes its owned allocation on
authentication, host, application, rendering, and normal-return paths. Empty,
control-containing, and over-256-byte values remain semantic inputs so an
active audit epoch can authenticate and durably record the failed access
before the closed error is returned.

The positional form follows VLT-PM00. As with any process argument, the host OS
and invoking shell may retain argv or command history outside the process; the
query is therefore intended for searchable metadata, not secret material. The
later foreground shell can avoid that external argv exposure.

## 3. Search semantics

After one-shot unlock, the application:

1. rebuilds its in-memory projection only from the authenticated current
   catalog;
2. rejects the complete operation if any item has multiple current candidates;
3. validates one 1–256 byte control-free UTF-8 query;
4. normalizes case and Unicode exactly as the storage-neutral application
   search contract defines;
5. intersects whitespace-separated substring tokens across the approved
   metadata allowlist;
6. returns at most 100 matches ordered by normalized title, schema, and exact
   item ID; and
7. drops the projection with the unlocked session.

Searchable metadata is limited to record title/label, login username and URLs,
API-key service, database host and username, and tags. Passwords, private
notes, note bodies, PAN/CVV/postal values, TOTP seeds, API tokens, database
passwords, attachment bytes, opaque payloads, and audit data are never indexed
or returned.

No index bytes, query, normalized token, hit set, score, or result cache are
written to the local state store or any storage provider.

## 4. Audit-first access

New CLI vaults already have an active generation-zero audit chain. Search uses
the existing `audited_search_items` boundary whenever auditing is active:

- valid zero-match and non-empty-match searches publish one succeeded
  itemless `ItemSearch` event before results are released;
- invalid query, result-bound, or current-conflict outcomes publish one failed
  itemless `ItemSearch` event before their closed error;
- audit publication failure releases neither results nor the original error
  and retains the exact recovery journal; and
- events contain no query, normalized value, result count, item identity,
  schema, title, URL, username, provider fact, or arbitrary error text.

Legacy pre-audit vaults retain the existing explicit migration contract. Once
an epoch is active, search cannot bypass it.

## 5. Output and errors

Each match is exactly:

```text
ITEM_ID<TAB>SCHEMA<TAB>"escaped title"
```

The row is identical to `item list`: it contains no matching field, snippet,
score, query echo, or secret-bearing value. Zero matches produce exactly:

```text
No matches.
```

Wrong passphrase returns locked; invalid semantic query returns invalid;
current conflict returns conflict; authenticated corruption returns integrity;
host/storage/audit publication unavailability returns provider. Failures have
empty stdout and fixed payload-free stderr.

## 6. Acceptance gates

The slice is complete only when tests prove:

1. grammar accepts exactly one Unicode query and rejects missing, extra, flag,
   and non-Unicode forms;
2. query debug is redacted and owned memory is wipe-on-drop;
3. title, username, URL, service, host, and tag matches use deterministic
   redacted rows while secret-field queries return no matches;
4. empty, control, oversized, and conflict failures become durable failed
   `ItemSearch` events before their errors;
5. success and zero-match results become durable succeeded events before
   output;
6. named-vault selection remains isolated and storage-provider agnostic;
7. a real-process PTY drill searches after restart, observes no query echo or
   secret bytes, and verifies the advanced audit chain; and
8. formatting, Clippy, rustdoc, application/CLI tests, and executable PTY tests
   pass on the affected dependency closure.
