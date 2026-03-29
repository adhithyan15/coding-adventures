package CodingAdventures::Cache;

# ============================================================================
# CodingAdventures::Cache — Pure-Perl Cache Implementations
# ============================================================================
#
# This module provides three cache data structures:
#
#   1. LRUCache          — Least Recently Used eviction policy (key-value)
#   2. DirectMappedCache — Hardware-style direct-mapped cache
#   3. SetAssociativeCache — Hardware-style N-way set-associative cache
#
# === WHY DO WE NEED CACHES? ===
#
# Modern CPUs run at ~3 GHz. DRAM takes ~100 ns to respond — that's ~300
# cycles of waiting. Caches solve this by keeping frequently-used data
# close to the CPU in fast SRAM.
#
# A cache stores (key → value) pairs. When you request a key:
#   HIT  — the key is in the cache → fast return
#   MISS — the key is not in cache → fetch from slow memory, store in cache
#
# When the cache is full and you need space, you EVICT the least recently
# used entry (LRU policy).
#
# === LRU CACHE ===
#
# The LRU (Least Recently Used) cache is the most commonly used cache policy.
# The entry that was accessed longest ago is evicted first when the cache
# is full. This is implemented using a doubly-linked list + hash:
#
#   Hash:  key → node (O(1) lookup)
#   List:  most-recent ... least-recent (O(1) insert/remove)
#
# Example with capacity=3:
#   put(a, 1) → cache: [a]
#   put(b, 2) → cache: [b, a]
#   put(c, 3) → cache: [c, b, a]
#   get(a)    → cache: [a, c, b]  (a moved to front — recently used)
#   put(d, 4) → cache: [d, a, c]  (b evicted — it's the LRU)
#
# === DIRECT-MAPPED CACHE ===
#
# Each memory address maps to exactly ONE slot in the cache.
# Simple and fast, but suffers from "conflict misses":
# two addresses might compete for the same slot.
#
#   Slot index = address % num_sets
#
# Example with 4 sets:
#   address 0 → slot 0
#   address 4 → slot 0  (conflicts with address 0!)
#   address 1 → slot 1
#
# === N-WAY SET-ASSOCIATIVE CACHE ===
#
# A compromise between direct-mapped (1 way) and fully-associative (∞ ways).
# Each address maps to a SET of N slots, and any slot within that set can
# hold it. This reduces conflict misses.
#
#   Set index = address % num_sets
#   Within the set: use LRU among the N ways
#
# === USAGE ===
#
#   use CodingAdventures::Cache;
#
#   my $lru = CodingAdventures::Cache::LRUCache->new(capacity => 3);
#   $lru->put('a', 1);
#   $lru->put('b', 2);
#   my $val = $lru->get('a');  # 1
#
#   my $dm = CodingAdventures::Cache::DirectMappedCache->new(sets => 4);
#   $dm->access(0);   # miss
#   $dm->access(0);   # hit
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

# ============================================================================
# LRU CACHE
# ============================================================================
#
# LRUCache — a fixed-capacity key-value cache with Least Recently Used
# eviction.
#
# Implementation uses a doubly-linked list to track access order and a
# hash for O(1) lookups.
#
# The list is arranged with the MOST recently used entry at the HEAD
# and the LEAST recently used entry at the TAIL.
#
#   HEAD ←→ node_A ←→ node_B ←→ node_C ←→ TAIL
#   (most recent)              (least recent)
#
# On get(key): move the node to the HEAD (mark as recently used).
# On put(key): if full, remove the TAIL node; add new node at HEAD.

package CodingAdventures::Cache::LRUCache;

# Create a new LRUCache with the given capacity.
#
# @param capacity  integer — maximum number of (key, value) entries
sub new {
    my ($class, %args) = @_;

    my $capacity = $args{capacity};
    die "capacity must be a positive integer\n"
        unless defined($capacity) && $capacity =~ /^\d+$/ && $capacity > 0;

    my $self = bless {
        capacity => $capacity,
        _map     => {},     # key → node hashref
        _hits    => 0,
        _misses  => 0,
        # Sentinel nodes: _head is MRU side, _tail is LRU side
        # Using sentinel nodes avoids edge-case handling for empty list.
        _head => undef,
        _tail => undef,
    }, $class;

    # Initialize sentinel nodes
    my $head = { key => undef, val => undef, prev => undef, next => undef };
    my $tail = { key => undef, val => undef, prev => undef, next => undef };
    $head->{next} = $tail;
    $tail->{prev} = $head;
    $self->{_head} = $head;
    $self->{_tail} = $tail;

    return $self;
}

# get(key) — retrieve the value for the key, or undef if not present.
#
# Side effects:
#   - Increments hit counter on a cache hit
#   - Increments miss counter on a cache miss
#   - Moves the accessed node to the front (MRU position)
sub get {
    my ($self, $key) = @_;
    my $node = $self->{_map}{$key};
    if ( !defined $node ) {
        $self->{_misses}++;
        return undef;
    }
    $self->{_hits}++;
    # Move this node to the MRU position (just after head sentinel)
    $self->_remove_node($node);
    $self->_insert_after_head($node);
    return $node->{val};
}

# put(key, value) — store a key-value pair in the cache.
#
# If the key already exists, update its value and move to MRU.
# If the cache is full, evict the LRU entry.
sub put {
    my ($self, $key, $val) = @_;
    my $node = $self->{_map}{$key};

    if ( defined $node ) {
        # Update existing entry and move to MRU
        $node->{val} = $val;
        $self->_remove_node($node);
        $self->_insert_after_head($node);
        return;
    }

    # Evict LRU entry if at capacity
    if ( scalar(keys %{ $self->{_map} }) >= $self->{capacity} ) {
        my $lru = $self->{_tail}{prev};  # node before tail sentinel
        $self->_remove_node($lru);
        delete $self->{_map}{ $lru->{key} };
    }

    # Insert new node at MRU position
    my $new_node = { key => $key, val => $val, prev => undef, next => undef };
    $self->{_map}{$key} = $new_node;
    $self->_insert_after_head($new_node);
}

# hits() — number of cache hits since creation.
sub hits   { return $_[0]->{_hits}   }

# misses() — number of cache misses since creation.
sub misses { return $_[0]->{_misses} }

# size() — current number of entries in the cache.
sub size   { return scalar(keys %{ $_[0]->{_map} }) }

# capacity() — maximum number of entries.
sub capacity { return $_[0]->{capacity} }

# keys_list() — list of keys from MRU to LRU order (for inspection/testing).
sub keys_list {
    my ($self) = @_;
    my @result;
    my $node = $self->{_head}{next};
    while ( defined $node && defined $node->{key} ) {
        push @result, $node->{key};
        $node = $node->{next};
    }
    return @result;
}

# --- Private doubly-linked list helpers ---

# Remove a node from the list (doesn't touch the map).
sub _remove_node {
    my ($self, $node) = @_;
    $node->{prev}{next} = $node->{next};
    $node->{next}{prev} = $node->{prev};
}

# Insert a node immediately after the head sentinel (MRU position).
sub _insert_after_head {
    my ($self, $node) = @_;
    my $head = $self->{_head};
    $node->{next} = $head->{next};
    $node->{prev} = $head;
    $head->{next}{prev} = $node;
    $head->{next} = $node;
}

# ============================================================================
# DIRECT-MAPPED CACHE
# ============================================================================
#
# DirectMappedCache — a hardware-style cache where each address maps to
# exactly one slot.
#
#   Slot index = address % num_sets
#
# If the slot is occupied by a different address, it's evicted (conflict
# miss). This is the simplest possible cache hardware design.
#
# Think of it like a parking lot where each car (address) has exactly
# one reserved spot (slot). If someone else is in your spot, they get
# towed (evicted).

package CodingAdventures::Cache::DirectMappedCache;

# Create a new DirectMappedCache.
#
# @param sets  integer — number of cache slots (must be a power of 2 for
#              real hardware, but we allow any positive integer here)
sub new {
    my ($class, %args) = @_;

    my $sets = $args{sets};
    die "sets must be a positive integer\n"
        unless defined($sets) && $sets =~ /^\d+$/ && $sets > 0;

    return bless {
        sets    => $sets,
        _slots  => {},  # slot_index => { tag => address, valid => 1 }
        _hits   => 0,
        _misses => 0,
    }, $class;
}

# access(address) — simulate a cache access for the given address.
#
# Returns 1 on hit, 0 on miss.
# On miss, the slot is filled with the new address (evicting the old one).
sub access {
    my ($self, $address) = @_;
    my $slot = $address % $self->{sets};
    my $entry = $self->{_slots}{$slot};

    if ( defined $entry && $entry->{valid} && $entry->{tag} == $address ) {
        $self->{_hits}++;
        return 1;  # HIT
    }

    # Miss — fill the slot with this address
    $self->{_misses}++;
    $self->{_slots}{$slot} = { tag => $address, valid => 1 };
    return 0;  # MISS
}

# hits() — total number of hits.
sub hits   { return $_[0]->{_hits}   }

# misses() — total number of misses.
sub misses { return $_[0]->{_misses} }

# sets() — number of slots in this cache.
sub sets   { return $_[0]->{sets}    }

# ============================================================================
# SET-ASSOCIATIVE CACHE
# ============================================================================
#
# SetAssociativeCache — a hardware-style N-way set-associative cache.
#
# The cache is divided into SETS. Each set has WAYS slots. An address maps
# to a specific set, and within that set any of the N ways can hold it.
# When all N ways are full, we evict the LRU entry in that set.
#
#   Set index = address % num_sets
#   Within set: LRU eviction among the `ways` slots
#
# This generalizes both direct-mapped (ways=1) and fully-associative
# (ways = total_capacity) caches.
#
# Visual for 4 sets, 2 ways:
#
#   Set 0: [ way0: addr=0  | way1: addr=4  ]
#   Set 1: [ way0: addr=1  | way1: addr=5  ]
#   Set 2: [ way0: addr=2  | way1: empty   ]
#   Set 3: [ way0: addr=3  | way1: empty   ]

package CodingAdventures::Cache::SetAssociativeCache;

# Create a new SetAssociativeCache.
#
# @param sets  integer — number of sets
# @param ways  integer — number of ways (slots) per set
sub new {
    my ($class, %args) = @_;

    my $sets = $args{sets};
    my $ways = $args{ways};
    die "sets must be a positive integer\n"
        unless defined($sets) && $sets =~ /^\d+$/ && $sets > 0;
    die "ways must be a positive integer\n"
        unless defined($ways) && $ways =~ /^\d+$/ && $ways > 0;

    # Initialize each set as an empty array of way-slots
    # Each slot: { tag => addr, valid => bool, lru_time => int }
    my @cache_sets;
    for my $i ( 0 .. $sets - 1 ) {
        $cache_sets[$i] = [];
    }

    return bless {
        sets     => $sets,
        ways     => $ways,
        _sets    => \@cache_sets,
        _hits    => 0,
        _misses  => 0,
        _clock   => 0,   # logical clock for LRU tracking
    }, $class;
}

# access(address) — simulate a cache access for the given address.
#
# Returns 1 on hit, 0 on miss.
sub access {
    my ($self, $address) = @_;
    my $set_idx = $address % $self->{sets};
    my $set     = $self->{_sets}[$set_idx];
    $self->{_clock}++;

    # Search for a hit in this set
    for my $way ( @$set ) {
        if ( $way->{valid} && $way->{tag} == $address ) {
            # HIT — update LRU time
            $way->{lru_time} = $self->{_clock};
            $self->{_hits}++;
            return 1;
        }
    }

    # MISS
    $self->{_misses}++;

    # Find an empty slot, or evict LRU
    my $victim_slot;
    if ( scalar(@$set) < $self->{ways} ) {
        # There's an empty slot — just append
        push @$set, { tag => $address, valid => 1, lru_time => $self->{_clock} };
    } else {
        # All ways full — find the LRU slot (lowest lru_time)
        my $lru_idx  = 0;
        my $lru_time = $set->[0]{lru_time};
        for my $i ( 1 .. $#$set ) {
            if ( $set->[$i]{lru_time} < $lru_time ) {
                $lru_time = $set->[$i]{lru_time};
                $lru_idx  = $i;
            }
        }
        $set->[$lru_idx] = { tag => $address, valid => 1, lru_time => $self->{_clock} };
    }

    return 0;
}

# hits() — total number of hits.
sub hits   { return $_[0]->{_hits}   }

# misses() — total number of misses.
sub misses { return $_[0]->{_misses} }

# sets() — number of sets.
sub sets   { return $_[0]->{sets}    }

# ways() — number of ways per set.
sub ways   { return $_[0]->{ways}    }

# ============================================================================
# BACK TO MAIN PACKAGE
# ============================================================================

package CodingAdventures::Cache;

1;

__END__

=head1 NAME

CodingAdventures::Cache - Pure-Perl cache implementations (LRU, Direct-Mapped, Set-Associative)

=head1 SYNOPSIS

    use CodingAdventures::Cache;

    # LRU Cache
    my $lru = CodingAdventures::Cache::LRUCache->new(capacity => 3);
    $lru->put('a', 1);
    $lru->put('b', 2);
    my $val = $lru->get('a');  # 1
    print $lru->hits;          # 1
    print $lru->misses;        # 0

    # Direct-Mapped Cache
    my $dm = CodingAdventures::Cache::DirectMappedCache->new(sets => 4);
    $dm->access(0);   # miss (0)
    $dm->access(0);   # hit  (1)

    # Set-Associative Cache
    my $sa = CodingAdventures::Cache::SetAssociativeCache->new(sets => 4, ways => 2);
    $sa->access(0);   # miss
    $sa->access(0);   # hit

=head1 DESCRIPTION

Provides three cache implementations for educational purposes:

=over 4

=item L<LRUCache> — key-value cache with LRU eviction (typical software cache)

=item L<DirectMappedCache> — hardware-style direct-mapped cache

=item L<SetAssociativeCache> — hardware-style N-way set-associative cache

=back

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
