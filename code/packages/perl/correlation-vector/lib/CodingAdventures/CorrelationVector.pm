package CodingAdventures::CorrelationVector;

# =============================================================================
# CodingAdventures::CorrelationVector — Append-only provenance tracking
# =============================================================================
#
# A Correlation Vector (CV) is a lightweight, append-only provenance record
# that follows a piece of data through every transformation it undergoes.
# Think of it like a "passport" for your data: when data is born it gets a
# unique ID (the CV ID), and every system that touches it stamps that passport
# with a contribution.
#
# At any point you can ask: "Where did this piece of data come from, and what
# happened to it?" The CV log gives you a complete, ordered answer.
#
# # Why provenance matters
#
# Imagine you're debugging a compiler pipeline:
#   - A variable `userPreferences` was renamed to `a`
#   - Then it was eliminated as dead code
#   - Now it's gone and you can't figure out why
#
# With CVs, you can look up the original token, find its CV ID, and see every
# transformation that was applied to it. The history is complete and ordered.
#
# The same concept works for:
#   - ETL pipelines: why did this record fail validation?
#   - Build systems: which source file produced this object file?
#   - ML pipelines: which preprocessing step introduced this outlier?
#   - Distributed tracing: which microservice caused this latency spike?
#
# # CV ID format
#
# Every CV ID is a string with dot-separated segments:
#
#   base.N         — a root CV (born from nothing or an external source)
#   base.N.M       — derived from base.N (a child of base.N)
#   base.N.M.K     — derived from base.N.M (a grandchild)
#
# The base is 8 hex characters derived from a SHA-256 hash of the origin
# string. For synthetic (programmatically created) entities, the base is
# always "00000000". The N is a global sequence counter.
#
# Reading the parentage directly from the ID (without consulting the log) is
# a design goal: the more dots, the deeper the derivation chain.
#
# # The CVLog structure
#
# The CVLog is the container for all CV entries in a pipeline run. It travels
# alongside the data being processed:
#
#   {
#     _enabled  => 1,        # tracing switch
#     _entries  => {},       # hashref: cv_id => entry hashref
#     _counter  => 0,        # global sequence counter for unique IDs
#   }
#
# When `_enabled` is false, all write operations become no-ops. CV IDs are
# still generated (entities still need their IDs), but no history is recorded.
# This means production code pays essentially zero overhead when tracing is off.
#
# # This module is domain-agnostic
#
# The CV library knows nothing about compilers, ETL, or any specific domain.
# Consumers attach meaning through the `source` and `tag` fields of
# contributions, and through arbitrary metadata. The library just stores
# and retrieves.

use strict;
use warnings;
use utf8;

use POSIX qw(strftime);
use CodingAdventures::SHA256;
use CodingAdventures::JsonSerializer;

our $VERSION = '0.1.0';

# =============================================================================
# new(\%opts) — Construct a new CVLog
# =============================================================================
#
# The CVLog is the top-level container. You create one at the start of your
# pipeline and pass it through every stage.
#
# Options:
#   enabled  (bool, default 1) — when 0, all write operations are no-ops.
#            Use this in production when you want zero overhead but still
#            need CV IDs to be assigned to entities.
#
# Example:
#   # Development: full tracing
#   my $log = CodingAdventures::CorrelationVector->new(enabled => 1);
#
#   # Production: IDs generated, no history stored
#   my $log = CodingAdventures::CorrelationVector->new(enabled => 0);

sub new {
    my ($class, %opts) = @_;
    # The `exists` check lets callers pass enabled => 0 explicitly.
    # Without it, `$opts{enabled} // 1` would be correct, but we use
    # exists to be explicit about the default.
    my $enabled = exists $opts{enabled} ? $opts{enabled} : 1;
    return bless {
        _enabled => $enabled,
        _entries => {},    # hashref: cv_id => entry hashref
        _counter => 0,     # global monotonically increasing sequence counter
    }, $class;
}

# =============================================================================
# _now() — Return current UTC timestamp in ISO 8601 format
# =============================================================================
#
# ISO 8601 is the international standard for date/time representation.
# The format "2026-04-05T14:30:00Z" means:
#   - 2026-04-05: year-month-day
#   - T: separator between date and time
#   - 14:30:00: hour:minute:second (24-hour clock)
#   - Z: Zulu time = UTC (no timezone offset)
#
# We use gmtime (not localtime) to ensure the timestamp is always UTC,
# regardless of the server's local timezone.

sub _now {
    return strftime("%Y-%m-%dT%H:%M:%SZ", gmtime);
}

# =============================================================================
# _next_id() — Allocate the next sequence number
# =============================================================================
#
# This is a simple monotonically increasing counter. The counter is global
# to the CVLog instance, so every call to create/derive/merge gets a unique N.
# This guarantees that even if two entities have the same base hash, their IDs
# differ by N.

sub _next_id {
    my ($self) = @_;
    return $self->{_counter}++;
}

# =============================================================================
# create(\%opts) — Born a new root CV
# =============================================================================
#
# A "root" CV has no parents — it was created fresh from an external source
# or from nothing at all. This is the entry point for any new entity entering
# your pipeline.
#
# Options:
#   origin_string  (string, optional) — identifier for the origin entity
#                  (file path + line:col, database row ID, etc.)
#                  Used to compute the base hash. If undef, treated as synthetic.
#   synthetic      (bool, default 0) — force base to "00000000"
#   meta           (hashref, optional) — arbitrary metadata to attach to origin
#
# Returns: $cv_id (string)
#
# If tracing is disabled, an ID is still generated and returned (entities need
# their IDs), but no entry is stored in the log.
#
# ID computation:
#   synthetic? → base = "00000000"
#   otherwise  → base = first 8 hex chars of SHA-256(origin_string)
#   cv_id = "${base}.${N}"
#
# Example:
#   my $cv_id = $log->create(origin_string => "app.ts:5:12");
#   # Returns something like "a3f1b2c4.0"
#
#   my $cv_id = $log->create(synthetic => 1);
#   # Returns "00000000.1"

sub create {
    my ($self, %opts) = @_;

    # Determine the base: synthetic entities always use "00000000".
    # Natural entities hash their origin string to get a stable, reproducible base.
    # The "reproducible" property is important for build systems — the same source
    # file always gets the same base, so IDs are consistent across runs.
    my $base;
    if ($opts{synthetic}) {
        $base = '00000000';
    } else {
        my $origin_str = $opts{origin_string} // '';
        # Take only the first 8 characters of the 64-char hex digest.
        # 8 hex chars = 4 bytes = 32 bits of entropy — enough to make collisions
        # astronomically unlikely in any real pipeline (birthday problem gives
        # ~50% collision probability only after ~65,000 entities with the same origin).
        $base = substr(CodingAdventures::SHA256::sha256_hex($origin_str), 0, 8);
    }

    my $n    = $self->_next_id();
    my $cv_id = "${base}.${n}";

    # If tracing is disabled, return the ID without storing anything.
    # The caller still needs the ID to tag their entity.
    return $cv_id unless $self->{_enabled};

    # Build the origin record. We store it even if origin_string is undef,
    # so the entry has a complete, consistent structure.
    my $origin = undef;
    if (defined $opts{origin_string}) {
        $origin = {
            string    => $opts{origin_string},
            synthetic => $opts{synthetic} ? 1 : 0,
        };
    } elsif ($opts{synthetic}) {
        $origin = {
            string    => undef,
            synthetic => 1,
        };
    }

    # Store the entry in the log.
    # Arrayrefs for contributions, merged_from, and pass_order start empty
    # and grow as the entity moves through the pipeline.
    $self->{_entries}{$cv_id} = {
        cv_id         => $cv_id,
        origin        => $origin,
        parent_cv_id  => undef,    # root: no parent
        merged_from   => [],       # root: not a merge result
        contributions => [],       # starts empty; grows with each contribute()
        deleted       => undef,    # not deleted yet
        pass_order    => [],       # deduplicated list of sources that touched this entity
    };

    return $cv_id;
}

# =============================================================================
# contribute($cv_id, \%opts) — Record that a stage processed this entity
# =============================================================================
#
# This is the most common operation. Call it every time a pipeline stage
# transforms or examines an entity.
#
# Contributions are appended in call order. The order is semantically
# meaningful — it is the sequence in which stages processed the entity.
#
# Options (required unless stated):
#   source  (string, required) — who/what contributed (stage name, service name)
#   tag     (string, required) — what happened (domain-defined label)
#   meta    (hashref, optional) — arbitrary key-value detail
#
# Errors:
#   - Dies if cv_id is not found in the log
#   - Dies if the entity has been deleted (you can't contribute to a tombstone)
#
# Example:
#   $log->contribute($cv_id,
#       source => 'scope_analysis',
#       tag    => 'resolved',
#       meta   => { binding => 'local:count:fn_main' }
#   );

sub contribute {
    my ($self, $cv_id, %opts) = @_;

    # No-op when disabled — no work to do, and we can't look up the entry.
    return unless $self->{_enabled};

    my $entry = $self->{_entries}{$cv_id};
    die "CodingAdventures::CorrelationVector::contribute: cv_id '$cv_id' not found\n"
        unless defined $entry;

    # Deleted entities are tombstones. Contributions to tombstones would create
    # a confusing history where events appear after deletion.
    die "CodingAdventures::CorrelationVector::contribute: cv_id '$cv_id' has been deleted\n"
        if defined $entry->{deleted};

    my $source = $opts{source} // die "contribute: 'source' is required\n";
    my $tag    = $opts{tag}    // die "contribute: 'tag' is required\n";
    my $meta   = $opts{meta}   // {};

    # Append the contribution record.
    push @{ $entry->{contributions} }, {
        source    => $source,
        tag       => $tag,
        meta      => $meta,
        timestamp => _now(),
    };

    # Update pass_order: deduplicated, ordered list of sources.
    # pass_order answers the question: "which stages has this entity passed through?"
    # We use a simple scan-and-skip for deduplication — the list is typically short
    # (< 100 stages in even the most complex pipelines).
    unless (grep { $_ eq $source } @{ $entry->{pass_order} }) {
        push @{ $entry->{pass_order} }, $source;
    }

    return;
}

# =============================================================================
# derive($parent_cv_id, \%opts) — Create a child CV from an existing one
# =============================================================================
#
# Use this when one entity is split into multiple outputs, or when a
# transformation produces a new entity that is conceptually descended from
# an existing one.
#
# Common use cases:
#   - Destructuring: {a, b} = x → two children, one for each binding
#   - ETL splitting: one wide record → two narrower records
#   - Compiler: inlining produces a new node descended from the function body
#
# The derived ID is: parent_cv_id + "." + N
# Example: "a3f1.0" → "a3f1.0.1" (first child), "a3f1.0.2" (second child)
#
# Options:
#   meta  (hashref, optional) — metadata for this derivation
#
# Returns: $new_cv_id (string)

sub derive {
    my ($self, $parent_cv_id, %opts) = @_;

    # N is allocated even when disabled, to keep the counter consistent.
    # (Though in practice, when disabled, the counter value doesn't matter
    # since nothing is stored.)
    my $n        = $self->_next_id();
    my $new_cv_id = "${parent_cv_id}.${n}";

    return $new_cv_id unless $self->{_enabled};

    # Validate that the parent exists. Deriving from a non-existent parent
    # would leave an orphaned entry with an invalid parent reference.
    my $parent = $self->{_entries}{$parent_cv_id};
    die "CodingAdventures::CorrelationVector::derive: parent cv_id '$parent_cv_id' not found\n"
        unless defined $parent;

    $self->{_entries}{$new_cv_id} = {
        cv_id         => $new_cv_id,
        origin        => undef,          # derived entities inherit origin implicitly via ancestry
        parent_cv_id  => $parent_cv_id,  # single parent
        merged_from   => [],             # not a merge
        contributions => [],
        deleted       => undef,
        pass_order    => [],
    };

    return $new_cv_id;
}

# =============================================================================
# merge(\@cv_ids, \%opts) — Create a CV descended from multiple parents
# =============================================================================
#
# Use this when multiple entities are combined into one output.
#
# Common use cases:
#   - Function inlining: call site + function body → merged expression
#   - Database JOIN: two rows → one result row
#   - Compilation: multiple .o files → one binary
#   - ML: multiple feature vectors → one combined feature vector
#
# The merged CV's ID uses a hash-derived base from the sorted, comma-joined
# parent IDs. This makes the merged ID deterministic: the same set of parents
# always produces the same base. The global counter N ensures uniqueness.
#
# Options:
#   meta  (hashref, optional) — metadata for this merge
#
# Parameters:
#   $cv_ids  (arrayref of cv_id strings) — the parents to merge
#
# Returns: $merged_cv_id (string)

sub merge {
    my ($self, $cv_ids, %opts) = @_;

    # Sort the parent IDs so the base hash is stable regardless of call order.
    # "merge([A, B])" and "merge([B, A])" produce the same base.
    my $sorted_ids = join(',', sort @$cv_ids);
    my $base       = substr(CodingAdventures::SHA256::sha256_hex($sorted_ids), 0, 8);
    my $n          = $self->_next_id();
    my $merged_id  = "${base}.${n}";

    return $merged_id unless $self->{_enabled};

    # Validate all parents exist.
    for my $pid (@$cv_ids) {
        die "CodingAdventures::CorrelationVector::merge: parent cv_id '$pid' not found\n"
            unless exists $self->{_entries}{$pid};
    }

    $self->{_entries}{$merged_id} = {
        cv_id         => $merged_id,
        origin        => undef,
        parent_cv_id  => undef,          # multiple parents — use merged_from instead
        merged_from   => [@$cv_ids],     # all parents, in original order
        contributions => [],
        deleted       => undef,
        pass_order    => [],
    };

    return $merged_id;
}

# =============================================================================
# delete($cv_id, \%opts) — Record that an entity was intentionally removed
# =============================================================================
#
# "Deleting" a CV entry does NOT remove it from the log — it marks it with a
# deletion record. This is intentional: the log is append-only. You can always
# ask "why did this entity disappear?" long after the fact.
#
# This is the "tombstone" pattern, common in distributed databases:
# instead of erasing data, you mark it as deleted with metadata explaining why.
# This preserves the complete history, including the reason for deletion.
#
# Calling `contribute()` on a deleted entity is an error.
# Calling `derive()` or `merge()` with a deleted entity as a parent is allowed
# (e.g., "derive a tombstone record from this deleted entity").
#
# Options:
#   by    (string, required) — who/what deleted it
#   meta  (hashref, optional) — arbitrary deletion metadata
#
# Example:
#   $log->delete($cv_id,
#       by   => 'dead_code_eliminator',
#       meta => { reason => 'unreachable from entry point' }
#   );

sub delete {
    my ($self, $cv_id, %opts) = @_;

    return unless $self->{_enabled};

    my $entry = $self->{_entries}{$cv_id};
    die "CodingAdventures::CorrelationVector::delete: cv_id '$cv_id' not found\n"
        unless defined $entry;

    my $by   = $opts{by}   // 'unknown';
    my $meta = $opts{meta} // {};

    # Record the deletion. The timestamp answers: "when was this deleted?"
    $entry->{deleted} = {
        by        => $by,
        at        => _now(),
        meta      => $meta,
    };

    return;
}

# =============================================================================
# passthrough($cv_id, \%opts) — Record that a stage passed without modifying
# =============================================================================
#
# Use this when a pipeline stage examines an entity but makes no changes.
# It is the "identity contribution" — it records that the stage was responsible
# for this entity, even though nothing changed.
#
# Why bother? Traceability. If you want to know "did stage X see entity Y?",
# passthrough lets you answer that even when X made no changes to Y.
#
# In performance-sensitive pipelines, passthrough may be omitted for stages
# that are known to never modify entities. The tradeoff is that those stages
# become invisible in the history.
#
# Options:
#   source  (string, required) — which stage passed through
#
# Example:
#   $log->passthrough($cv_id, source => 'type_checker');

sub passthrough {
    my ($self, $cv_id, %opts) = @_;

    return unless $self->{_enabled};

    my $entry = $self->{_entries}{$cv_id};
    die "CodingAdventures::CorrelationVector::passthrough: cv_id '$cv_id' not found\n"
        unless defined $entry;

    my $source = $opts{source} // die "passthrough: 'source' is required\n";

    # A passthrough is just a contribution with the special tag "passthrough".
    # This keeps the data model simple — contributions are contributions,
    # regardless of whether they changed anything.
    push @{ $entry->{contributions} }, {
        source    => $source,
        tag       => 'passthrough',
        meta      => {},
        timestamp => _now(),
    };

    # Update pass_order (same dedup logic as contribute).
    unless (grep { $_ eq $source } @{ $entry->{pass_order} }) {
        push @{ $entry->{pass_order} }, $source;
    }

    return;
}

# =============================================================================
# get($cv_id) — Return the full entry for a CV ID, or undef
# =============================================================================
#
# Returns the raw entry hashref, or undef if the cv_id is not in the log.
# When tracing is disabled, this always returns undef (nothing was stored).
#
# Example:
#   my $entry = $log->get($cv_id);
#   if (defined $entry) {
#       print "ID: ", $entry->{cv_id}, "\n";
#   }

sub get {
    my ($self, $cv_id) = @_;
    return $self->{_entries}{$cv_id};
}

# =============================================================================
# ancestors($cv_id) — Return all ancestor CV IDs, nearest-first
# =============================================================================
#
# Walks the parent_cv_id / merged_from chain recursively and returns all
# ancestor CV IDs in breadth-first order (immediate parents first, then
# grandparents, etc.).
#
# For a simple derivation chain A → B → C:
#   ancestors(C) = ["B", "A"]
#
# For a merge where A and B were merged into M:
#   ancestors(M) = ["A", "B"]
#
# The implementation uses an iterative queue to avoid stack overflow on
# very deep chains (e.g., a million-node AST).
#
# Cycles are impossible by construction (you can't create a CV that is
# its own ancestor), but we track a `seen` set to guard against
# pathological inputs (e.g., a manually crafted corrupted log).

sub ancestors {
    my ($self, $cv_id) = @_;

    my @result;
    my %seen = ($cv_id => 1);
    my @queue;

    # Start with the immediate parents of the given cv_id.
    my $entry = $self->{_entries}{$cv_id};
    return [] unless defined $entry;

    _enqueue_parents($entry, \@queue, \%seen);

    while (@queue) {
        my $pid = shift @queue;
        push @result, $pid;
        my $parent_entry = $self->{_entries}{$pid};
        next unless defined $parent_entry;
        _enqueue_parents($parent_entry, \@queue, \%seen);
    }

    return \@result;
}

# _enqueue_parents($entry, \@queue, \%seen)
#
# Internal helper: add the parents of $entry to the BFS queue if not seen.
# An entry has parents either via parent_cv_id (derivation) or merged_from
# (merge). A root entry has neither.

sub _enqueue_parents {
    my ($entry, $queue, $seen) = @_;

    # Derivation parent
    if (defined $entry->{parent_cv_id}) {
        my $pid = $entry->{parent_cv_id};
        unless ($seen->{$pid}) {
            $seen->{$pid} = 1;
            push @$queue, $pid;
        }
    }

    # Merge parents (multiple)
    for my $pid (@{ $entry->{merged_from} // [] }) {
        unless ($seen->{$pid}) {
            $seen->{$pid} = 1;
            push @$queue, $pid;
        }
    }
}

# =============================================================================
# descendants($cv_id) — Return all CV IDs that descend from this one
# =============================================================================
#
# The inverse of ancestors. Returns all CV IDs where this cv_id appears
# in their ancestor chain.
#
# Implementation: scans all entries in the log. For large logs, an index
# keyed by parent_cv_id would be faster, but the current O(n) scan is
# correct and sufficient for typical pipeline sizes.
#
# Uses BFS (same as ancestors) to handle transitive descendants:
#   A → B → C → D
#   descendants(A) = ["B", "C", "D"]

sub descendants {
    my ($self, $cv_id) = @_;

    my @result;
    my %seen = ($cv_id => 1);
    my @queue = ($cv_id);

    while (@queue) {
        my $current = shift @queue;

        # Scan all entries looking for children of `current`.
        for my $eid (keys %{ $self->{_entries} }) {
            next if $seen{$eid};
            my $e = $self->{_entries}{$eid};

            # Is this entry a direct child of `current`?
            my $is_child = 0;
            $is_child = 1 if defined $e->{parent_cv_id} && $e->{parent_cv_id} eq $current;
            unless ($is_child) {
                for my $pid (@{ $e->{merged_from} // [] }) {
                    if ($pid eq $current) { $is_child = 1; last }
                }
            }

            if ($is_child) {
                $seen{$eid} = 1;
                push @result, $eid;
                push @queue, $eid;
            }
        }
    }

    return \@result;
}

# =============================================================================
# history($cv_id) — Return the contributions for a CV ID in order
# =============================================================================
#
# Returns an arrayref of contribution hashrefs, in the order they were added.
# If the entity was deleted, the deletion record is NOT included (it's in the
# `deleted` field of the entry, not in the contributions list).
#
# Returns an empty arrayref if:
#   - The cv_id is not found
#   - Tracing was disabled (nothing was stored)
#
# Example:
#   my $history = $log->history($cv_id);
#   for my $contrib (@$history) {
#       printf "%s: %s\n", $contrib->{source}, $contrib->{tag};
#   }

sub history {
    my ($self, $cv_id) = @_;
    my $entry = $self->{_entries}{$cv_id};
    return [] unless defined $entry;
    return $entry->{contributions};
}

# =============================================================================
# lineage($cv_id) — Return full entries for this entity and all its ancestors
# =============================================================================
#
# Returns an arrayref of entry hashrefs, ordered from oldest ancestor to
# the entity itself. This is the complete provenance chain.
#
# For a chain A → B → C:
#   lineage(C) = [entry_A, entry_B, entry_C]
#
# This is the "tell me everything about where this data came from" operation.
# It's the most comprehensive query.

sub lineage {
    my ($self, $cv_id) = @_;

    my $ancestor_ids = $self->ancestors($cv_id);

    # ancestors() returns nearest-first. We want oldest-first, so we reverse.
    # IMPORTANT: use explicit parentheses around reverse's argument to prevent
    # Perl from reversing the entire list including $cv_id.
    # Without parens: (reverse @$ancestor_ids, $cv_id) = reverse(@$ancestor_ids, $cv_id)
    # With parens:    ((reverse @$ancestor_ids), $cv_id) — correct
    my @ordered = ((reverse @$ancestor_ids), $cv_id);

    my @result;
    for my $id (@ordered) {
        my $entry = $self->{_entries}{$id};
        push @result, $entry if defined $entry;
    }

    return \@result;
}

# =============================================================================
# serialize() — Serialize the CVLog to a JSON string
# =============================================================================
#
# Converts the entire CVLog to a JSON string for storage or cross-process
# transmission. The format is the canonical interchange format — all language
# implementations produce and consume the same JSON structure.
#
# Format:
#   {
#     "entries": { "cv_id": { ... entry ... }, ... },
#     "pass_order": [...],
#     "enabled": true,
#     "counter": 42
#   }
#
# Note: we store the counter so that deserialized logs continue to generate
# unique IDs that don't collide with existing ones.

sub serialize {
    my ($self) = @_;

    # Build a plain hashref representation.
    # The JSON serializer handles the actual encoding.
    my %entries_data;
    for my $cv_id (keys %{ $self->{_entries} }) {
        my $e = $self->{_entries}{$cv_id};
        $entries_data{$cv_id} = _entry_to_hash($e);
    }

    my $data = {
        entries   => \%entries_data,
        enabled   => $self->{_enabled} ? 1 : 0,
        counter   => $self->{_counter},
    };

    return CodingAdventures::JsonSerializer::encode($data, { sort_keys => 1 });
}

# _entry_to_hash($entry) → hashref suitable for JSON encoding
#
# Converts an entry hashref to a plain data structure.
# We explicitly include all fields so the JSON output is self-documenting.

sub _entry_to_hash {
    my ($e) = @_;
    return {
        cv_id         => $e->{cv_id},
        origin        => $e->{origin},       # undef → null in JSON
        parent_cv_id  => $e->{parent_cv_id}, # undef → null in JSON
        merged_from   => $e->{merged_from},
        contributions => $e->{contributions},
        deleted       => $e->{deleted},
        pass_order    => $e->{pass_order},
    };
}

# =============================================================================
# _to_perl_undef($v) — Normalize JSON null sentinel to Perl undef
# =============================================================================
#
# When the JSON serializer decodes a JSON `null`, it returns a blessed
# `CodingAdventures::JsonValue::Null` sentinel object — not Perl `undef`.
# This distinction is important for JSON (null vs. absent key), but inside
# this module we want Perl `undef` to mean "not present."
#
# This helper converts the sentinel to undef, leaving all other values intact.

sub _to_perl_undef {
    my ($v) = @_;
    return undef if !defined $v;
    return undef if CodingAdventures::JsonSerializer::is_null($v);
    return $v;
}

# =============================================================================
# deserialize($class, $json_str) — Reconstruct a CVLog from JSON
# =============================================================================
#
# Class method. Parses a JSON string (produced by serialize()) and returns
# a new CVLog object with all entries restored.
#
# Usage:
#   my $log2 = CodingAdventures::CorrelationVector->deserialize($json_str);
#
# The deserialized log is fully functional: you can continue adding
# contributions, deriving new CVs, etc.
#
# Note on JSON null: the JSON decoder returns a blessed Null sentinel for
# JSON `null`, not Perl `undef`. We normalize these back to undef using
# _to_perl_undef() so the deserialized entries behave identically to
# freshly created ones.

sub deserialize {
    my ($class, $json_str) = @_;

    my $data = CodingAdventures::JsonSerializer::decode($json_str);

    my $self = bless {
        _enabled => $data->{enabled} ? 1 : 0,
        _entries => {},
        _counter => $data->{counter} // 0,
    }, $class;

    # Restore each entry.
    my $entries_data = $data->{entries} // {};
    for my $cv_id (keys %$entries_data) {
        my $e = $entries_data->{$cv_id};

        # Normalize JSON nulls to Perl undef.
        # parent_cv_id, origin, and deleted are all nullable fields.
        my $parent_cv_id = _to_perl_undef($e->{parent_cv_id});
        my $origin       = _to_perl_undef($e->{origin});
        my $deleted      = _to_perl_undef($e->{deleted});

        # merged_from and contributions are arrays; if JSON gave us null, use [].
        my $merged_from   = _to_perl_undef($e->{merged_from})   // [];
        my $contributions = _to_perl_undef($e->{contributions}) // [];
        my $pass_order    = _to_perl_undef($e->{pass_order})    // [];

        $self->{_entries}{$cv_id} = {
            cv_id         => $e->{cv_id}  // $cv_id,
            origin        => $origin,
            parent_cv_id  => $parent_cv_id,
            merged_from   => $merged_from,
            contributions => $contributions,
            deleted       => $deleted,
            pass_order    => $pass_order,
        };
    }

    return $self;
}

1;

__END__

=head1 NAME

CodingAdventures::CorrelationVector - Append-only provenance tracking for any data pipeline

=head1 SYNOPSIS

    use CodingAdventures::CorrelationVector;

    # Create a new CVLog for a pipeline run
    my $log = CodingAdventures::CorrelationVector->new(enabled => 1);

    # Assign a CV to a new entity
    my $cv_id = $log->create(origin_string => "app.ts:5:12");

    # Record that a stage processed it
    $log->contribute($cv_id,
        source => 'scope_analysis',
        tag    => 'resolved',
        meta   => { binding => 'local:count:fn_main' },
    );

    # Stage that made no changes
    $log->passthrough($cv_id, source => 'type_checker');

    # One entity splits into two
    my $child_a = $log->derive($cv_id);
    my $child_b = $log->derive($cv_id);

    # Two entities combine into one
    my $merged = $log->merge([$child_a, $child_b]);

    # Entity is removed
    $log->delete($cv_id, by => 'dead_code_eliminator',
        meta => { reason => 'unreachable from entry point' });

    # Query
    my $entry    = $log->get($cv_id);
    my $history  = $log->history($cv_id);
    my $parents  = $log->ancestors($merged);
    my $children = $log->descendants($cv_id);
    my $chain    = $log->lineage($merged);

    # Serialize for storage or cross-process transmission
    my $json = $log->serialize();
    my $log2 = CodingAdventures::CorrelationVector->deserialize($json);

=head1 DESCRIPTION

A Correlation Vector (CV) is a lightweight, append-only provenance record
that follows a piece of data through every transformation it undergoes.
Every entity gets a C<cv_id> at birth; every transformation appends its
contribution.

This module is domain-agnostic — useful for compiler pipelines, ETL,
neural network tracing, build systems, and anywhere data flows through
a sequence of transformations.

=head1 METHODS

=head2 new(%opts)

Constructor. Options: C<enabled> (default 1).

=head2 create(%opts) → $cv_id

Born a new root CV. Options: C<origin_string>, C<synthetic>, C<meta>.

=head2 contribute($cv_id, %opts)

Record a stage contribution. Required: C<source>, C<tag>. Optional: C<meta>.

=head2 derive($parent_cv_id, %opts) → $cv_id

Create a child CV descended from an existing one.

=head2 merge(\@cv_ids, %opts) → $cv_id

Create a CV descended from multiple parents.

=head2 delete($cv_id, %opts)

Mark an entity as deleted. Required: C<by>. Optional: C<meta>.

=head2 passthrough($cv_id, %opts)

Record that a stage examined the entity without changing it. Required: C<source>.

=head2 get($cv_id) → hashref | undef

Return the full entry for a CV ID, or undef.

=head2 ancestors($cv_id) → \@cv_ids

Return all ancestor CV IDs, nearest-first.

=head2 descendants($cv_id) → \@cv_ids

Return all CV IDs that descend from this one.

=head2 history($cv_id) → \@contributions

Return the contribution history for a CV ID.

=head2 lineage($cv_id) → \@entries

Return full entries for this entity and all its ancestors, oldest-first.

=head2 serialize() → $json_str

Serialize the CVLog to a JSON string.

=head2 deserialize($class, $json_str) → CVLog

Class method. Reconstruct a CVLog from a JSON string.

=head1 VERSION

0.1.0

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
