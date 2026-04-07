"""Correlation Vector — append-only provenance tracking for any data pipeline.

A Correlation Vector (CV) is a lightweight, append-only provenance record that
follows a piece of data through every transformation it undergoes.  Assign a CV
to anything when it is born.  Every system, stage, or function that touches it
appends its contribution.  At any point you can ask "where did this come from
and what happened to it?" and get a complete, ordered answer.

The concept originated in distributed-systems tracing, where a request flows
through dozens of microservices and you need to reconstruct what happened across
all of them.  This implementation generalises the idea to *any* pipeline —
compiler passes, data ETL, document transformations, build systems, ML
preprocessing, or anywhere that data flows through a sequence of
transformations.

Key ideas
---------

* **CV ID** — a stable, globally unique string assigned at birth.  It never
  changes.  The entity can be transformed, renamed, merged, split, or deleted;
  the CV ID is the one constant.

* **Contribution** — every stage appends one contribution: who processed it,
  what happened, and any detail metadata.

* **CVEntry** — the full record: ID, parent IDs, origin, contributions, and an
  optional deletion record.

* **CVLog** — the map that holds all entries.  An ``enabled`` flag lets you
  disable tracing in production at near-zero cost.

Example — compiler pipeline::

    from coding_adventures_correlation_vector import CVLog, Origin

    log = CVLog()
    cv_id = log.create(Origin(source="app.ts", location="5:12"))
    log.contribute(cv_id, "parser",           "created", {"token": "IDENTIFIER"})
    log.contribute(cv_id, "scope_analysis",   "resolved", {"binding": "local"})
    log.contribute(cv_id, "variable_renamer", "renamed",  {"from": "count", "to": "a"})
    print(log.history(cv_id))

Example — ETL pipeline::

    log = CVLog()
    cv_id = log.create(Origin(source="orders_table", location="row_id:8472"))
    log.contribute(cv_id, "validator",   "schema_checked",  {"schema": "orders_v2"})
    log.contribute(cv_id, "normalizer",  "date_converted",  {"format": "ISO8601"})
    log.contribute(cv_id, "geo_enricher","geo_appended",    {"country": "US"})
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

# ---------------------------------------------------------------------------
# Import the repo's SHA-256 package.
#
# We deliberately use the repo's own SHA-256 implementation rather than
# Python's stdlib hashlib.  This lets us stress-test our own package and
# keeps the dependency graph internal to the monorepo.
#
# sha256_hex(data: bytes) -> str  returns the 64-char lowercase hex digest.
# ---------------------------------------------------------------------------
from coding_adventures_sha256 import sha256_hex

# ---------------------------------------------------------------------------
# Import the repo's JSON infrastructure.
#
# json_value.parse_native parses a JSON string directly into native Python
# types (dict / list / str / int / float / bool / None).
#
# json_serializer.stringify serialises native Python types to compact JSON.
# ---------------------------------------------------------------------------
from json_serializer import stringify
from json_value import parse_native

__all__ = [
    "Origin",
    "Contribution",
    "DeletionRecord",
    "CVEntry",
    "CVLog",
]

# ---------------------------------------------------------------------------
# Data classes
# ---------------------------------------------------------------------------


@dataclass
class Origin:
    """The birth record of an entity — where and when it came into existence.

    Attributes
    ----------
    source:
        Identifies the origin system or file.  Examples: ``"app.ts"``,
        ``"orders_table"``, ``"compiler_frontend"``.
    location:
        Position within the source.  Examples: ``"5:12"`` (line:col),
        ``"row_id:8472"`` (database row), ``"byte:0"`` (byte offset).
    timestamp:
        Optional ISO 8601 timestamp, useful when time ordering matters across
        systems.  ``None`` when irrelevant.
    meta:
        Arbitrary extra origin context, e.g. schema version, file hash.
    """

    source: str
    location: str
    timestamp: str | None = None
    meta: dict[str, Any] = field(default_factory=dict)


@dataclass
class Contribution:
    """A single stage's record of having processed an entity.

    The CV library imposes no constraints on the values of ``source`` or
    ``tag`` — those are entirely defined by the consumer domain.

    Examples across domains::

        # Compiler:
        Contribution("variable_renamer", "renamed", {"from": "userPrefs", "to": "a"})

        # ETL:
        Contribution("date_normalizer", "converted", {"from_format": "MM/DD/YYYY"})

        # Build system:
        Contribution("gcc", "compiled", {"flags": "-O2", "output": "main.o"})

    Attributes
    ----------
    source:
        Who or what contributed — stage name, service name, pass name.
    tag:
        What happened — a domain-defined label such as ``"resolved"``,
        ``"renamed"``, ``"compiled"``, ``"deleted"``.
    meta:
        Arbitrary key-value detail.  Domain-defined; the CV library ignores it.
    """

    source: str
    tag: str
    meta: dict[str, Any] = field(default_factory=dict)


@dataclass
class DeletionRecord:
    """Records that an entity was intentionally removed from the pipeline.

    The CV entry itself stays in the log permanently — this is how you can
    answer "why did this disappear?" long after the fact.

    Attributes
    ----------
    source:
        Who performed the deletion (e.g. ``"dead_code_eliminator"``).
    reason:
        Human-readable reason (e.g. ``"unreachable from entry point"``).
    meta:
        Optional extra detail, e.g. the entry-point CV ID.
    """

    source: str
    reason: str
    meta: dict[str, Any] = field(default_factory=dict)


@dataclass
class CVEntry:
    """The full provenance record for a single tracked entity.

    Attributes
    ----------
    id:
        Stable, globally unique CV ID.  Assigned at birth and never changed.
    parent_ids:
        Empty list for root CVs; one or more parents for derived/merged CVs.
        Reading the dots in the ID gives you the lineage at a glance:
        ``"a3f1.1.2"`` is a grandchild of base ``"a3f1"``.
    origin:
        Where and when this entity was born.  ``None`` for synthetic entities
        created without a natural origin.
    contributions:
        Append-only list of every stage that touched this entity, in order.
    deleted:
        Non-``None`` if this entity was intentionally removed.  The record
        stays in the log forever so the deletion is always queryable.
    """

    id: str
    parent_ids: list[str] = field(default_factory=list)
    origin: Origin | None = None
    contributions: list[Contribution] = field(default_factory=list)
    deleted: DeletionRecord | None = None


# ---------------------------------------------------------------------------
# CVLog
# ---------------------------------------------------------------------------


class CVLog:
    """The central provenance map for a pipeline run.

    The CVLog travels alongside the data being processed, accumulating the
    history of every entity.  It exposes six mutating operations and five
    query operations, plus JSON serialisation helpers.

    The ``enabled`` flag is the tracing switch.  When ``False``, every
    mutating operation returns immediately without allocating or writing
    anything.  CV IDs are still generated and returned so entities can carry
    them — they just never get any history populated.  This means production
    code pays near-zero overhead when tracing is off, and full provenance when
    it is on.

    ID scheme
    ---------
    CV IDs use a dot-extension scheme that encodes parentage::

        base.N        — root CV  (N increments per base)
        base.N.M      — derived from base.N  (M increments per parent)
        base.N.M.K    — derived from base.N.M

    The base is an 8-hex-character prefix of the SHA-256 of the origin string
    ``f"{origin.source}:{origin.location}"``.  Synthetic entities (no origin)
    use the fixed base ``"00000000"``.

    Example::

        log = CVLog()
        root = log.create(Origin("app.ts", "1:0"))
        child1 = log.derive(root)
        child2 = log.derive(root)
        log.contribute(child1, "renamer", "renamed", {"from": "x", "to": "a"})
        print(log.lineage(child1))
    """

    def __init__(self, enabled: bool = True) -> None:
        """Initialise a new, empty CVLog.

        Parameters
        ----------
        enabled:
            When ``True`` (default) all operations record history.
            When ``False`` CV IDs are still generated but nothing is stored.
        """
        self.enabled: bool = enabled
        # Map from cv_id → CVEntry
        self._entries: dict[str, CVEntry] = {}
        # Ordered list of source names that have ever contributed (deduped).
        self.pass_order: list[str] = []
        # Counters for root ID generation: base → next sequence number.
        self._base_counters: dict[str, int] = {}
        # Counters for derived ID generation: parent_cv_id → next child seq.
        self._child_counters: dict[str, int] = {}

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------

    @staticmethod
    def _base_from_origin(origin: Origin | None) -> str:
        """Compute the 8-character hex base segment for a root CV.

        If *origin* is ``None`` (synthetic entity), the base is always
        ``"00000000"``.  Otherwise we hash ``"{source}:{location}"`` with
        SHA-256 and take the first 8 hex characters.

        Why SHA-256?  It gives us a deterministic, collision-resistant mapping
        from any origin string to a short identifier.  Two entities born at the
        same source position share the same base — they are distinguished by
        the sequence suffix ``.N``.

        Parameters
        ----------
        origin:
            The birth record; ``None`` for synthetic entities.

        Returns
        -------
        str
            An 8-character lowercase hex string.
        """
        if origin is None:
            return "00000000"
        raw = f"{origin.source}:{origin.location}"
        return sha256_hex(raw.encode())[:8]

    def _next_root_id(self, base: str) -> str:
        """Return the next root CV ID for *base*, incrementing the counter.

        Sequence numbers start at 1 and are monotonically increasing per base::

            "a3f1b2c4.1", "a3f1b2c4.2", "a3f1b2c4.3", ...
        """
        n = self._base_counters.get(base, 0) + 1
        self._base_counters[base] = n
        return f"{base}.{n}"

    def _next_child_id(self, parent_cv_id: str) -> str:
        """Return the next derived CV ID under *parent_cv_id*.

        Child sequence numbers start at 1 and are monotonically increasing per
        parent::

            "a3f1.1.1", "a3f1.1.2", "a3f1.1.3", ...
        """
        m = self._child_counters.get(parent_cv_id, 0) + 1
        self._child_counters[parent_cv_id] = m
        return f"{parent_cv_id}.{m}"

    def _record_pass(self, source: str) -> None:
        """Append *source* to ``pass_order`` if not already present."""
        if source not in self.pass_order:
            self.pass_order.append(source)

    # ------------------------------------------------------------------
    # Mutating operations
    # ------------------------------------------------------------------

    def create(self, origin: Origin | None = None) -> str:
        """Born a new root CV — an entity with no parents.

        Use this when an entity enters the pipeline for the first time: a token
        parsed from source, a row read from a database, a file discovered in a
        directory scan.

        If tracing is disabled the CV ID is still computed and returned (the
        entity needs it) but no entry is stored.

        Parameters
        ----------
        origin:
            Where and when the entity was born.  Pass ``None`` for synthetic
            entities that have no natural source position.

        Returns
        -------
        str
            The new CV ID, e.g. ``"a3f1b2c4.1"``.
        """
        base = self._base_from_origin(origin)
        cv_id = self._next_root_id(base)
        if self.enabled:
            entry = CVEntry(id=cv_id, parent_ids=[], origin=origin)
            self._entries[cv_id] = entry
        return cv_id

    def contribute(
        self,
        cv_id: str,
        source: str,
        tag: str,
        meta: dict[str, Any] | None = None,
    ) -> None:
        """Record that a stage processed the entity identified by *cv_id*.

        Contributions are appended in call order — the sequence is semantically
        meaningful because it reflects the order in which stages touched the
        entity.

        Raises ``ValueError`` if *cv_id* has been deleted (you cannot add
        history to a tombstoned entity).

        If tracing is disabled this is a no-op.

        Parameters
        ----------
        cv_id:
            The entity being contributed to.
        source:
            Who is contributing (stage name, service name).
        tag:
            What happened (domain-defined label).
        meta:
            Optional extra detail.  Defaults to an empty dict.

        Raises
        ------
        ValueError
            If *cv_id* refers to an entity that has already been deleted.
        """
        if not self.enabled:
            return
        entry = self._entries.get(cv_id)
        if entry is not None and entry.deleted is not None:
            raise ValueError(
                f"Cannot contribute to deleted CV entry '{cv_id}': "
                f"deleted by '{entry.deleted.source}' "
                f"with reason '{entry.deleted.reason}'."
            )
        if entry is not None:
            contribution = Contribution(
                source=source, tag=tag, meta=meta if meta is not None else {}
            )
            entry.contributions.append(contribution)
            self._record_pass(source)

    def derive(
        self, parent_cv_id: str, origin: Origin | None = None
    ) -> str:
        """Create a new CV descended from an existing one.

        Use this when one entity is split into multiple outputs, or when a
        transformation produces a new entity that is conceptually "the same
        thing" expressed differently::

            # Destructuring {a, b} = x produces two derived CVs:
            cv_a = log.derive(original_cv_id)
            cv_b = log.derive(original_cv_id)

        The derived CV's ID appends a new sequence number to the parent ID::

            parent  "a3f1.1"
            child1  "a3f1.1.1"
            child2  "a3f1.1.2"

        If tracing is disabled the CV ID is still computed and returned.

        Parameters
        ----------
        parent_cv_id:
            The CV ID of the entity being split or transformed.
        origin:
            Optional new origin record for the derived entity.

        Returns
        -------
        str
            The new derived CV ID.
        """
        cv_id = self._next_child_id(parent_cv_id)
        if self.enabled:
            entry = CVEntry(id=cv_id, parent_ids=[parent_cv_id], origin=origin)
            self._entries[cv_id] = entry
        return cv_id

    def merge(
        self, parent_cv_ids: list[str], origin: Origin | None = None
    ) -> str:
        """Create a new CV descended from multiple existing CVs.

        Use this when multiple entities are combined into one output::

            # Inlining a function: call site + function body → one expression
            merged = log.merge([call_site_cv, function_body_cv])

        The merged CV's ``parent_ids`` lists all parents.  Its ID is generated
        using the ``00000000`` synthetic base (no single natural origin) unless
        an explicit *origin* is provided.

        If tracing is disabled the CV ID is still computed and returned.

        Parameters
        ----------
        parent_cv_ids:
            All CV IDs being merged together.
        origin:
            Optional origin record for the merged entity.  When provided the
            base is derived from the origin; otherwise ``"00000000"`` is used.

        Returns
        -------
        str
            The new merged CV ID.
        """
        base = self._base_from_origin(origin)
        cv_id = self._next_root_id(base)
        if self.enabled:
            entry = CVEntry(
                id=cv_id, parent_ids=list(parent_cv_ids), origin=origin
            )
            self._entries[cv_id] = entry
        return cv_id

    def delete(
        self,
        cv_id: str,
        source: str,
        reason: str,
        meta: dict[str, Any] | None = None,
    ) -> None:
        """Record that an entity was intentionally removed.

        The CV entry stays in the log permanently — this is how you can answer
        "why did this disappear?" long after the fact.  After deletion any call
        to ``contribute`` for this *cv_id* will raise ``ValueError``.

        If tracing is disabled this is a no-op.

        Parameters
        ----------
        cv_id:
            The entity being deleted.
        source:
            Who is performing the deletion.
        reason:
            Human-readable explanation.
        meta:
            Optional extra detail.
        """
        if not self.enabled:
            return
        entry = self._entries.get(cv_id)
        if entry is not None:
            entry.deleted = DeletionRecord(
                source=source, reason=reason, meta=meta if meta is not None else {}
            )
            self._record_pass(source)

    def passthrough(self, cv_id: str, source: str) -> None:
        """Record that a stage examined this entity but made no changes.

        This is important for reconstructing which stages an entity passed
        through even when nothing was transformed.  It is the identity
        contribution — semantically equivalent to::

            log.contribute(cv_id, source, "passthrough")

        In performance-sensitive pipelines, ``passthrough`` may be omitted for
        known-clean stages to reduce log size.  The tradeoff is that the stage
        will be invisible in the history for unaffected entities.

        If tracing is disabled this is a no-op.

        Parameters
        ----------
        cv_id:
            The entity being examined.
        source:
            The stage that examined it.
        """
        if not self.enabled:
            return
        entry = self._entries.get(cv_id)
        if entry is not None:
            contribution = Contribution(source=source, tag="passthrough", meta={})
            entry.contributions.append(contribution)
            self._record_pass(source)

    # ------------------------------------------------------------------
    # Query operations
    # ------------------------------------------------------------------

    def get(self, cv_id: str) -> CVEntry | None:
        """Return the full entry for *cv_id*, or ``None`` if not found.

        When tracing is disabled this always returns ``None`` because nothing
        was stored.

        Parameters
        ----------
        cv_id:
            The CV ID to look up.

        Returns
        -------
        CVEntry | None
            The entry if it exists, else ``None``.
        """
        return self._entries.get(cv_id)

    def ancestors(self, cv_id: str) -> list[str]:
        """Walk the parent chain and return all ancestor CV IDs.

        The result is ordered from immediate parent to most distant ancestor
        (breadth-first by generation, but since the graph is a DAG the order
        is: immediate parent(s) first, then their parents, etc.).

        Cycles are impossible by construction (a CV cannot be its own
        ancestor), but the implementation guards against pathological inputs
        via a visited set.

        When tracing is disabled returns ``[]``.

        Parameters
        ----------
        cv_id:
            Starting point for the ancestor walk.

        Returns
        -------
        list[str]
            Ancestor CV IDs, nearest parent first, most distant ancestor last.
        """
        result: list[str] = []
        visited: set[str] = set()
        # BFS queue — process parents before grandparents.
        if cv_id in self._entries:
            queue: list[str] = list(self._entries[cv_id].parent_ids)
        else:
            queue = []
        while queue:
            current = queue.pop(0)
            if current in visited:
                continue
            visited.add(current)
            result.append(current)
            entry = self._entries.get(current)
            if entry:
                queue.extend(entry.parent_ids)
        return result

    def descendants(self, cv_id: str) -> list[str]:
        """Return all CV IDs that have *cv_id* in their ancestor chain.

        This is the inverse of ``ancestors``.  It is computed by scanning all
        entries — for large logs, consider indexing by parent_id.

        When tracing is disabled returns ``[]``.

        Parameters
        ----------
        cv_id:
            The CV ID whose descendants are wanted.

        Returns
        -------
        list[str]
            All descendant CV IDs (unordered).
        """
        result: list[str] = []
        for entry_id in self._entries:
            if entry_id == cv_id:
                continue
            if cv_id in self.ancestors(entry_id):
                result.append(entry_id)
        return result

    def history(self, cv_id: str) -> list[Contribution]:
        """Return the contributions for *cv_id* in order.

        When tracing is disabled returns ``[]``.

        Parameters
        ----------
        cv_id:
            The entity whose history is wanted.

        Returns
        -------
        list[Contribution]
            Contributions in append order.  If the entity was deleted the
            deletion record is NOT included here (see the ``deleted`` field of
            the CVEntry instead).
        """
        entry = self._entries.get(cv_id)
        if entry is None:
            return []
        return list(entry.contributions)

    def lineage(self, cv_id: str) -> list[CVEntry]:
        """Return the full CV entries for the entity and all its ancestors.

        Ordered oldest ancestor first, the entity itself last.  This is the
        complete provenance chain — reading it top to bottom tells the whole
        story of where the entity came from.

        When tracing is disabled returns ``[]``.

        Parameters
        ----------
        cv_id:
            The entity whose lineage is wanted.

        Returns
        -------
        list[CVEntry]
            Oldest ancestor first, entity itself last.
        """
        if cv_id not in self._entries:
            return []
        ancestor_ids = self.ancestors(cv_id)
        # ancestors() returns nearest-first; we want oldest-first.
        ancestor_ids_oldest_first = list(reversed(ancestor_ids))
        entries: list[CVEntry] = []
        for aid in ancestor_ids_oldest_first:
            entry = self._entries.get(aid)
            if entry is not None:
                entries.append(entry)
        self_entry = self._entries.get(cv_id)
        if self_entry is not None:
            entries.append(self_entry)
        return entries

    # ------------------------------------------------------------------
    # Serialisation
    # ------------------------------------------------------------------

    def serialize(self) -> dict[str, Any]:
        """Serialise the full CVLog to a plain Python dict.

        The output matches the canonical JSON schema defined in the spec::

            {
              "entries": { "<cv_id>": { ... }, ... },
              "pass_order": [ "parser", "renamer", ... ],
              "enabled": true
            }

        Numeric types in ``meta`` fields are preserved as-is (int or float).

        Returns
        -------
        dict
            A plain dict ready for ``json.dumps`` or the repo's json-serializer.
        """

        def _serialize_origin(o: Origin | None) -> dict[str, Any] | None:
            if o is None:
                return None
            return {
                "source": o.source,
                "location": o.location,
                "timestamp": o.timestamp,
                "meta": o.meta,
            }

        def _serialize_contribution(c: Contribution) -> dict[str, Any]:
            return {"source": c.source, "tag": c.tag, "meta": c.meta}

        def _serialize_deletion(d: DeletionRecord | None) -> dict[str, Any] | None:
            if d is None:
                return None
            return {"source": d.source, "reason": d.reason, "meta": d.meta}

        def _serialize_entry(e: CVEntry) -> dict[str, Any]:
            return {
                "id": e.id,
                "parent_ids": e.parent_ids,
                "origin": _serialize_origin(e.origin),
                "contributions": [_serialize_contribution(c) for c in e.contributions],
                "deleted": _serialize_deletion(e.deleted),
            }

        return {
            "entries": {eid: _serialize_entry(e) for eid, e in self._entries.items()},
            "pass_order": list(self.pass_order),
            "enabled": self.enabled,
        }

    def to_json_string(self) -> str:
        """Serialise the CVLog to a compact JSON string.

        Uses the repo's ``json-serializer`` package (``stringify``) so the
        output format is consistent across all packages in the monorepo.

        Returns
        -------
        str
            Compact JSON string representation of the CVLog.
        """
        return stringify(self.serialize())

    @classmethod
    def from_json_string(cls, s: str) -> CVLog:
        """Reconstruct a CVLog from a JSON string.

        Uses the repo's ``json-value`` package (``parse_native``) to parse the
        string into native Python types, then delegates to ``deserialize``.

        Parameters
        ----------
        s:
            JSON string produced by ``to_json_string``.

        Returns
        -------
        CVLog
            A fully restored CVLog instance.
        """
        data: dict[str, Any] = parse_native(s)  # type: ignore[assignment]
        return cls.deserialize(data)

    @classmethod
    def deserialize(cls, data: dict[str, Any]) -> CVLog:
        """Reconstruct a CVLog from a plain Python dict.

        The inverse of ``serialize``.  The *data* dict must match the canonical
        schema (as produced by ``serialize``).

        Parameters
        ----------
        data:
            Plain dict representation of a CVLog.

        Returns
        -------
        CVLog
            A fully restored CVLog instance with all internal counters
            re-derived from the stored IDs.
        """

        def _deserialize_origin(o: dict[str, Any] | None) -> Origin | None:
            if o is None:
                return None
            return Origin(
                source=o["source"],
                location=o["location"],
                timestamp=o.get("timestamp"),
                meta=o.get("meta") or {},
            )

        def _deserialize_contribution(c: dict[str, Any]) -> Contribution:
            return Contribution(
                source=c["source"],
                tag=c["tag"],
                meta=c.get("meta") or {},
            )

        def _deserialize_deletion(d: dict[str, Any] | None) -> DeletionRecord | None:
            if d is None:
                return None
            return DeletionRecord(
                source=d["source"],
                reason=d["reason"],
                meta=d.get("meta") or {},
            )

        enabled: bool = bool(data.get("enabled", True))
        log = cls(enabled=enabled)
        log.pass_order = list(data.get("pass_order") or [])

        entries_data: dict[str, Any] = data.get("entries") or {}
        for cv_id, edata in entries_data.items():
            entry = CVEntry(
                id=cv_id,
                parent_ids=list(edata.get("parent_ids") or []),
                origin=_deserialize_origin(edata.get("origin")),
                contributions=[
                    _deserialize_contribution(c)
                    for c in (edata.get("contributions") or [])
                ],
                deleted=_deserialize_deletion(edata.get("deleted")),
            )
            log._entries[cv_id] = entry

        # Re-derive _base_counters and _child_counters from the stored IDs
        # so that any subsequent create/derive/merge calls produce non-
        # colliding IDs.
        #
        # An ID looks like:  base.N   or   base.N.M   or   base.N.M.K …
        # The rule:
        #   _base_counters[base] = max(N across all ids sharing that base)
        #   _child_counters[parent] = max(M across all ids sharing that parent)
        for cv_id in log._entries:
            parts = cv_id.split(".")
            if len(parts) >= 2:
                base = parts[0]
                n = int(parts[1])
                if log._base_counters.get(base, 0) < n:
                    log._base_counters[base] = n
            if len(parts) >= 3:
                # parent = everything up to the last segment
                parent = ".".join(parts[:-1])
                m = int(parts[-1])
                if log._child_counters.get(parent, 0) < m:
                    log._child_counters[parent] = m

        return log
