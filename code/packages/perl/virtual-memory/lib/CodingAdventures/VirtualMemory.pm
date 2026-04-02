package CodingAdventures::VirtualMemory;

# ============================================================================
# CodingAdventures::VirtualMemory — Virtual memory subsystem in Pure Perl
# ============================================================================
#
# # What is Virtual Memory?
#
# Virtual memory is one of the most important abstractions in computer science.
# Every process believes it owns the whole address space — from address 0 up
# to 4 GB on a 32-bit system — even though it shares physical RAM with dozens
# of other processes.
#
# The hardware Memory Management Unit (MMU) silently translates every memory
# access from a *virtual address* (what the program uses) to a *physical
# address* (the actual DRAM location).
#
# ## Analogy: The Apartment Building
#
#   +------------------+         +------------------+
#   | Process A        |         | Physical Memory  |
#   | "My room 1" -----+-------> | Room 401         |
#   | "My room 2" -----+-------> | Room 207         |
#   +------------------+         +------------------+
#   +------------------+
#   | Process B        |
#   | "My room 1" -----+-------> | Room 712         |
#   +------------------+
#
# Each process has its own "room numbering" (virtual address space). The OS +
# MMU (building manager) knows where each room actually lives in the building
# (physical frame).
#
# ## Address Layout (Sv32 — 32-bit RISC-V)
#
#   32-bit virtual address:
#   ┌────────────┬────────────┬────────────────┐
#   │  VPN[1]    │  VPN[0]    │  Page Offset   │
#   │ bits 31-22 │ bits 21-12 │ bits 11-0      │
#   │ (10 bits)  │ (10 bits)  │ (12 bits)      │
#   └────────────┴────────────┴────────────────┘
#
#   PAGE_SIZE = 2^12 = 4096 bytes (4 KB)
#   physical_address = (frame_number << 12) | offset
#
# ## Translation Pipeline (MMU)
#
#   Virtual Address
#       |
#       v
#   [TLB Lookup]  ──hit──> physical address ✓
#       |
#      miss
#       |
#   [Page Table Walk] (two-level)
#       |
#      found──> install in TLB ──> physical address ✓
#       |
#      fault
#       |
#   [Page Fault Handler]
#       └──> allocate frame, load page, retry
#
# ============================================================================

use strict;
use warnings;
use Carp qw(croak);
use List::Util qw(min);

our $VERSION = '0.01';

# ============================================================================
# Constants
# ============================================================================

# PAGE_SIZE: 4 KB — standard since Intel 80386 (1985)
use constant PAGE_SIZE         => 4096;
use constant PAGE_OFFSET_BITS  => 12;
use constant PAGE_OFFSET_MASK  => 0xFFF;
use constant DEFAULT_TLB_CAP   => 64;

# ============================================================================
# PageTableEntry
# ============================================================================
#
# A PTE maps one virtual page to one physical frame. In RISC-V Sv32, the
# hardware packs all these fields into a single 32-bit word:
#
#   bit 0  V — Valid (present)
#   bit 1  R — Readable
#   bit 2  W — Writable
#   bit 3  X — Executable
#   bit 4  U — User-accessible
#   bit 6  A — Accessed
#   bit 7  D — Dirty
#   bits 31-10  PPN (Physical Page Number = frame number)
#
# We store fields separately for clarity.

# new_pte($frame_number, %opts) → hashref
sub new_pte {
    my ($frame_number, %opts) = @_;
    return {
        frame_number    => $frame_number,
        present         => exists $opts{present}         ? $opts{present}         : 1,
        dirty           => exists $opts{dirty}           ? $opts{dirty}           : 0,
        accessed        => exists $opts{accessed}        ? $opts{accessed}        : 0,
        writable        => exists $opts{writable}        ? $opts{writable}        : 1,
        executable      => exists $opts{executable}      ? $opts{executable}      : 0,
        user_accessible => exists $opts{user_accessible} ? $opts{user_accessible} : 1,
        cow             => exists $opts{cow}             ? $opts{cow}             : 0,
        _type           => 'pte',
    };
}

# ============================================================================
# Single-Level Page Table
# ============================================================================
#
# The simplest page table: a flat array indexed by Virtual Page Number (VPN).
# For a 32-bit address space with 4 KB pages: 2^20 = 1,048,576 entries — one
# megabyte just for PTEs! This is why two-level page tables were invented.
#
# Layout:
#   VPN (20 bits) → PTE
#
#   vpn = virtual_address >> 12

sub new_page_table {
    return { entries => {}, _type => 'page_table' };
}

sub pt_map {
    my ($pt, $vpn, $frame_number, %opts) = @_;
    $pt->{entries}{$vpn} = new_pte($frame_number, %opts);
    return $pt;
}

sub pt_lookup {
    my ($pt, $vpn) = @_;
    return $pt->{entries}{$vpn};
}

sub pt_unmap {
    my ($pt, $vpn) = @_;
    delete $pt->{entries}{$vpn};
    return $pt;
}

sub pt_mapped_count {
    my ($pt) = @_;
    return scalar keys %{$pt->{entries}};
}

sub pt_update_pte {
    my ($pt, $vpn, %updates) = @_;
    my $pte = $pt->{entries}{$vpn} or return;
    for my $key (keys %updates) {
        $pte->{$key} = $updates{$key};
    }
    return $pte;
}

sub pt_all_mappings {
    my ($pt) = @_;
    return { %{$pt->{entries}} };
}

# ============================================================================
# Address Decomposition (Sv32)
# ============================================================================
#
# Given a 32-bit virtual address, extract VPN[1], VPN[0], and page offset.
#
#   ┌────────────┬────────────┬────────────────┐
#   │  VPN[1]    │  VPN[0]    │  Page Offset   │
#   │ bits 31-22 │ bits 21-12 │ bits 11-0      │
#   └────────────┴────────────┴────────────────┘
#
#   vpn1   = (addr >> 22) & 0x3FF  → 10 bits
#   vpn0   = (addr >> 12) & 0x3FF  → 10 bits
#   offset =  addr        & 0xFFF  → 12 bits

sub split_address {
    my ($vaddr) = @_;
    my $vpn1   = ($vaddr >> 22) & 0x3FF;
    my $vpn0   = ($vaddr >> 12) & 0x3FF;
    my $offset =  $vaddr        & 0xFFF;
    return ($vpn1, $vpn0, $offset);
}

sub vpn_of {
    my ($vaddr) = @_;
    return $vaddr >> PAGE_OFFSET_BITS;
}

sub make_physical_address {
    my ($frame_number, $offset) = @_;
    return ($frame_number << PAGE_OFFSET_BITS) | $offset;
}

# ============================================================================
# Two-Level Page Table (Sv32)
# ============================================================================
#
# Instead of one huge flat table, use a directory of page tables:
#
#   Level-1 (PD):  indexed by VPN[1] → pointer to Level-2 table
#   Level-2 (PT):  indexed by VPN[0] → PTE
#
# Memory savings: only allocate L2 tables for VPN[1] values that are actually
# in use. A process that uses only a few MB needs only a handful of L2 tables
# instead of 1 million PTEs.
#
#   Virtual Address
#       │
#       ├─[VPN1]──> L1 entry (pointer to L2 table)
#       │                │
#       └─[VPN0]──> PTE (frame number + flags)
#                        │
#                        └─[offset]──> byte

sub new_two_level_pt {
    # directory: { vpn1 => { vpn0 => pte } }
    return { directory => {}, _type => 'two_level_pt' };
}

sub tpt_map {
    my ($tpt, $vaddr, $frame_number, %opts) = @_;
    my ($vpn1, $vpn0) = split_address($vaddr);
    $tpt->{directory}{$vpn1} //= {};
    $tpt->{directory}{$vpn1}{$vpn0} = new_pte($frame_number, %opts);
    return $tpt;
}

sub tpt_translate {
    my ($tpt, $vaddr) = @_;
    my ($vpn1, $vpn0, $offset) = split_address($vaddr);
    my $l2 = $tpt->{directory}{$vpn1} or return undef;
    my $pte = $l2->{$vpn0}            or return undef;
    return undef unless $pte->{present};
    return make_physical_address($pte->{frame_number}, $offset);
}

sub tpt_lookup_pte {
    my ($tpt, $vaddr) = @_;
    my ($vpn1, $vpn0) = split_address($vaddr);
    my $l2 = $tpt->{directory}{$vpn1} or return undef;
    return $l2->{$vpn0};
}

sub tpt_unmap {
    my ($tpt, $vaddr) = @_;
    my ($vpn1, $vpn0) = split_address($vaddr);
    my $l2 = $tpt->{directory}{$vpn1} or return $tpt;
    delete $l2->{$vpn0};
    delete $tpt->{directory}{$vpn1} unless %$l2;
    return $tpt;
}

sub tpt_update_pte {
    my ($tpt, $vaddr, %updates) = @_;
    my ($vpn1, $vpn0) = split_address($vaddr);
    my $l2  = $tpt->{directory}{$vpn1} or return undef;
    my $pte = $l2->{$vpn0}            or return undef;
    for my $key (keys %updates) {
        $pte->{$key} = $updates{$key};
    }
    return $pte;
}

sub tpt_all_mappings {
    my ($tpt) = @_;
    my %result;
    for my $vpn1 (keys %{$tpt->{directory}}) {
        my $l2 = $tpt->{directory}{$vpn1};
        for my $vpn0 (keys %$l2) {
            my $vaddr = ($vpn1 << 22) | ($vpn0 << 12);
            $result{$vaddr} = $l2->{$vpn0};
        }
    }
    return \%result;
}

sub tpt_mapped_count {
    my ($tpt) = @_;
    my $count = 0;
    for my $l2 (values %{$tpt->{directory}}) {
        $count += scalar keys %$l2;
    }
    return $count;
}

# ============================================================================
# TLB (Translation Lookaside Buffer)
# ============================================================================
#
# The TLB is a small, fast cache inside the CPU for recent virtual→physical
# translations. Instead of doing a page table walk (2+ memory accesses) on
# every load/store, the CPU checks the TLB first.
#
# Modern CPUs have 32-1024 TLB entries per level. A TLB hit costs ~1 cycle;
# a miss costs ~10-100 cycles. Because programs exhibit spatial and temporal
# locality, TLB hit rates are typically 99%+.
#
# Our TLB uses LRU (Least Recently Used) eviction — when the TLB is full,
# we evict the entry that was accessed least recently.
#
# Key: "$pid:$vpn" (string) — supports multiple address spaces.
# The TLB is flushed on context switch (or on ASID change in real hardware).

sub new_tlb {
    my ($capacity) = @_;
    $capacity //= DEFAULT_TLB_CAP;
    return {
        capacity  => $capacity,
        entries   => {},   # key => { frame_number, flags... }
        lru_order => [],   # ordered list of keys, front = oldest
        hits      => 0,
        misses    => 0,
        _type     => 'tlb',
    };
}

sub tlb_lookup {
    my ($tlb, $pid, $vpn) = @_;
    my $key   = "$pid:$vpn";
    my $entry = $tlb->{entries}{$key};
    unless (defined $entry) {
        $tlb->{misses}++;
        return undef;
    }
    $tlb->{hits}++;
    # Move to end of LRU order (most recently used)
    $tlb->{lru_order} = [grep { $_ ne $key } @{$tlb->{lru_order}}];
    push @{$tlb->{lru_order}}, $key;
    return $entry;
}

sub tlb_insert {
    my ($tlb, $pid, $vpn, $pte) = @_;
    my $key = "$pid:$vpn";

    # Evict LRU entry if at capacity
    if (!exists $tlb->{entries}{$key}
        && scalar(keys %{$tlb->{entries}}) >= $tlb->{capacity}) {
        my $evict_key = shift @{$tlb->{lru_order}};
        delete $tlb->{entries}{$evict_key} if defined $evict_key;
    }

    # Remove existing entry from LRU order if updating
    $tlb->{lru_order} = [grep { $_ ne $key } @{$tlb->{lru_order}}];
    push @{$tlb->{lru_order}}, $key;

    $tlb->{entries}{$key} = { %$pte };
    return $tlb;
}

sub tlb_invalidate {
    my ($tlb, $pid, $vpn) = @_;
    my $key = "$pid:$vpn";
    delete $tlb->{entries}{$key};
    $tlb->{lru_order} = [grep { $_ ne $key } @{$tlb->{lru_order}}];
    return $tlb;
}

sub tlb_flush {
    my ($tlb, $pid) = @_;
    if (defined $pid) {
        # Flush only entries for this process
        my $prefix = "$pid:";
        my @to_delete = grep { index($_, $prefix) == 0 } keys %{$tlb->{entries}};
        delete @{$tlb->{entries}}{@to_delete};
        $tlb->{lru_order} = [grep { index($_, $prefix) != 0 } @{$tlb->{lru_order}}];
    } else {
        # Full TLB flush
        $tlb->{entries}   = {};
        $tlb->{lru_order} = [];
    }
    return $tlb;
}

sub tlb_hit_rate {
    my ($tlb) = @_;
    my $total = $tlb->{hits} + $tlb->{misses};
    return $total == 0 ? 0.0 : $tlb->{hits} / $total;
}

sub tlb_size {
    my ($tlb) = @_;
    return scalar keys %{$tlb->{entries}};
}

# ============================================================================
# Frame Allocator
# ============================================================================
#
# Physical memory is divided into fixed-size frames (same size as pages).
# The frame allocator tracks which frames are free.
#
# In a real OS, this is the "buddy allocator" or "zone allocator". We use a
# simple free-list.

sub new_frame_allocator {
    my ($total_frames) = @_;
    return {
        total_frames => $total_frames,
        free_frames  => [0 .. $total_frames - 1],  # all frames start free
        allocated    => {},   # frame_number => 1
        _type        => 'frame_allocator',
    };
}

sub alloc_frame {
    my ($fa) = @_;
    return undef unless @{$fa->{free_frames}};
    my $frame = shift @{$fa->{free_frames}};
    $fa->{allocated}{$frame} = 1;
    return $frame;
}

sub free_frame {
    my ($fa, $frame_number) = @_;
    croak "Double-free of frame $frame_number\n"
        unless $fa->{allocated}{$frame_number};
    delete $fa->{allocated}{$frame_number};
    push @{$fa->{free_frames}}, $frame_number;
    return $fa;
}

sub frame_is_allocated {
    my ($fa, $frame_number) = @_;
    return exists $fa->{allocated}{$frame_number} ? 1 : 0;
}

sub free_frame_count {
    my ($fa) = @_;
    return scalar @{$fa->{free_frames}};
}

sub allocated_frame_count {
    my ($fa) = @_;
    return scalar keys %{$fa->{allocated}};
}

# ============================================================================
# Page Replacement Policies
# ============================================================================
#
# When physical memory is full and a new page must be loaded, the OS must
# choose a victim frame to evict. Three classic policies:
#
# ## FIFO (First-In, First-Out)
#
#   Evict the page that has been in memory the longest. Simple, but can
#   evict heavily-used pages. Suffers from Belady's anomaly.
#
# ## LRU (Least Recently Used)
#
#   Evict the page that was accessed least recently. Excellent hit rate,
#   but expensive to implement in hardware (requires timestamps or
#   reference bits on every access).
#
# ## Clock (Second Chance)
#
#   Approximation of LRU. Each page has a "use bit". The clock hand sweeps
#   the frames:
#     - use bit = 1: clear it, advance hand (give second chance)
#     - use bit = 0: evict this page
#
#   Hardware sets the use bit on access. The OS reads and clears it.
#   Cheap and effective — used in many real OSes.

# --- FIFO ---
sub new_fifo_policy {
    return { queue => [], _type => 'fifo' };
}

sub _fifo_add_frame {
    my ($policy, $frame) = @_;
    push @{$policy->{queue}}, $frame;
}

sub _fifo_record_access {
    # FIFO ignores accesses
}

sub _fifo_select_victim {
    my ($policy) = @_;
    return undef unless @{$policy->{queue}};
    return $policy->{queue}[0];
}

sub _fifo_remove_frame {
    my ($policy, $frame) = @_;
    $policy->{queue} = [grep { $_ != $frame } @{$policy->{queue}}];
}

# --- LRU ---
sub new_lru_policy {
    return { order => [], _type => 'lru' };
}

sub _lru_add_frame {
    my ($policy, $frame) = @_;
    push @{$policy->{order}}, $frame;
}

sub _lru_record_access {
    my ($policy, $frame) = @_;
    $policy->{order} = [grep { $_ != $frame } @{$policy->{order}}];
    push @{$policy->{order}}, $frame;
}

sub _lru_select_victim {
    my ($policy) = @_;
    return @{$policy->{order}} ? $policy->{order}[0] : undef;
}

sub _lru_remove_frame {
    my ($policy, $frame) = @_;
    $policy->{order} = [grep { $_ != $frame } @{$policy->{order}}];
}

# --- Clock ---
sub new_clock_policy {
    return { frames => [], use_bits => {}, hand => 0, _type => 'clock' };
}

sub _clock_add_frame {
    my ($policy, $frame) = @_;
    push @{$policy->{frames}}, $frame;
    $policy->{use_bits}{$frame} = 1;
}

sub _clock_record_access {
    my ($policy, $frame) = @_;
    $policy->{use_bits}{$frame} = 1;
}

sub _clock_select_victim {
    my ($policy) = @_;
    my $frames = $policy->{frames};
    return undef unless @$frames;
    my $n = scalar @$frames;
    for (1 .. 2 * $n) {  # at most 2 full sweeps
        my $h = $policy->{hand} % $n;
        my $f = $frames->[$h];
        if ($policy->{use_bits}{$f}) {
            $policy->{use_bits}{$f} = 0;
            $policy->{hand} = ($h + 1) % $n;
        } else {
            $policy->{hand} = ($h + 1) % $n;
            return $f;
        }
    }
    # All pages had use bits set; return first frame
    return $frames->[0];
}

sub _clock_remove_frame {
    my ($policy, $frame) = @_;
    my @new_frames = grep { $_ != $frame } @{$policy->{frames}};
    $policy->{frames} = \@new_frames;
    delete $policy->{use_bits}{$frame};
    $policy->{hand} = 0 if @new_frames == 0;
    $policy->{hand} = $policy->{hand} % scalar(@new_frames) if @new_frames;
}

# --- Unified policy dispatch ---
sub policy_add_frame {
    my ($policy, $frame) = @_;
    my $t = $policy->{_type};
    if    ($t eq 'fifo')  { _fifo_add_frame($policy, $frame) }
    elsif ($t eq 'lru')   { _lru_add_frame($policy, $frame) }
    elsif ($t eq 'clock') { _clock_add_frame($policy, $frame) }
    else  { croak "Unknown policy type: $t\n" }
}

sub policy_record_access {
    my ($policy, $frame) = @_;
    my $t = $policy->{_type};
    if    ($t eq 'fifo')  { _fifo_record_access($policy, $frame) }
    elsif ($t eq 'lru')   { _lru_record_access($policy, $frame) }
    elsif ($t eq 'clock') { _clock_record_access($policy, $frame) }
}

sub policy_select_victim {
    my ($policy) = @_;
    my $t = $policy->{_type};
    if    ($t eq 'fifo')  { return _fifo_select_victim($policy) }
    elsif ($t eq 'lru')   { return _lru_select_victim($policy) }
    elsif ($t eq 'clock') { return _clock_select_victim($policy) }
    else  { croak "Unknown policy type: $t\n" }
}

sub policy_remove_frame {
    my ($policy, $frame) = @_;
    my $t = $policy->{_type};
    if    ($t eq 'fifo')  { _fifo_remove_frame($policy, $frame) }
    elsif ($t eq 'lru')   { _lru_remove_frame($policy, $frame) }
    elsif ($t eq 'clock') { _clock_remove_frame($policy, $frame) }
}

# ============================================================================
# MMU (Memory Management Unit)
# ============================================================================
#
# The MMU orchestrates all virtual memory machinery:
#   - Maintains per-process two-level page tables
#   - Caches recent translations in the TLB
#   - Allocates/frees physical frames
#   - Handles page faults (load missing pages from "disk")
#   - Implements Copy-on-Write for fork()
#
# Address spaces are identified by an integer PID.
#
# ## Copy-on-Write (COW) Fork
#
# When a process forks(), copying all its memory would be expensive (gigabytes!).
# Instead, both parent and child share the same physical frames, but all pages
# are marked read-only. When either process writes, a page fault occurs, the
# OS copies the frame, and both get their own private copy. Only modified pages
# are duplicated.
#
#   Before write:          After write to page P:
#   Parent──┐              Parent──>  (new frame, own copy)
#           v                                    ^
#        Frame F            Child──>  Frame F  (original)
#           ^
#   Child ──┘

sub new_mmu {
    my (%opts) = @_;
    my $total_frames = $opts{total_frames} // 64;
    my $policy_type  = $opts{policy_type}  // 'lru';
    my $tlb_capacity = $opts{tlb_capacity} // DEFAULT_TLB_CAP;

    my $policy;
    if    ($policy_type eq 'fifo')  { $policy = new_fifo_policy() }
    elsif ($policy_type eq 'lru')   { $policy = new_lru_policy() }
    elsif ($policy_type eq 'clock') { $policy = new_clock_policy() }
    else  { croak "Unknown policy type: $policy_type\n" }

    return {
        frame_allocator => new_frame_allocator($total_frames),
        page_tables     => {},    # pid => two_level_pt
        tlb             => new_tlb($tlb_capacity),
        policy          => $policy,
        refcounts       => {},    # frame_number => count
        _type           => 'mmu',
    };
}

# Create a new address space for PID
sub mmu_create_address_space {
    my ($mmu, $pid) = @_;
    $mmu->{page_tables}{$pid} = new_two_level_pt();
    return $mmu;
}

# Destroy address space and free all its frames
sub mmu_destroy_address_space {
    my ($mmu, $pid) = @_;
    my $pt = $mmu->{page_tables}{$pid} or return $mmu;
    my $mappings = tpt_all_mappings($pt);
    for my $pte (values %$mappings) {
        next unless $pte->{present};
        _mmu_decref($mmu, $pte->{frame_number});
    }
    tlb_flush($mmu->{tlb}, $pid);
    delete $mmu->{page_tables}{$pid};
    return $mmu;
}

# Map a virtual page to a specific frame (or auto-allocate)
sub mmu_map_page {
    my ($mmu, $pid, $vaddr, %opts) = @_;
    my $pt = $mmu->{page_tables}{$pid}
        or croak "Unknown PID $pid\n";

    my $frame_number = $opts{frame_number};
    unless (defined $frame_number) {
        $frame_number = alloc_frame($mmu->{frame_allocator});
        croak "Out of physical memory\n" unless defined $frame_number;
    }

    tpt_map($pt, $vaddr, $frame_number, %opts);
    $mmu->{refcounts}{$frame_number} = ($mmu->{refcounts}{$frame_number} // 0) + 1;
    policy_add_frame($mmu->{policy}, $frame_number);
    return $frame_number;
}

# Translate virtual address → physical address
# Returns: ($paddr, $result_type)
#   $result_type: 'hit' | 'miss' | 'fault' | 'error'
sub mmu_translate {
    my ($mmu, $pid, $vaddr, %opts) = @_;
    my $write = $opts{write} // 0;
    my ($vpn1, $vpn0, $offset) = split_address($vaddr);
    my $vpn = vpn_of($vaddr);

    # Step 1: TLB lookup
    my $tlb_entry = tlb_lookup($mmu->{tlb}, $pid, $vpn);
    if ($tlb_entry) {
        if ($write) {
            if ($tlb_entry->{cow}) {
                my ($new_paddr, $type) = _mmu_handle_cow($mmu, $pid, $vaddr);
                return ($new_paddr, 'cow');
            }
            croak "Write to read-only page at vaddr $vaddr\n"
                unless $tlb_entry->{writable};
            $tlb_entry->{dirty}    = 1;
            $tlb_entry->{accessed} = 1;
            # Update in page table too
            tpt_update_pte($mmu->{page_tables}{$pid}, $vaddr,
                dirty => 1, accessed => 1);
        } else {
            $tlb_entry->{accessed} = 1;
        }
        policy_record_access($mmu->{policy}, $tlb_entry->{frame_number});
        return (make_physical_address($tlb_entry->{frame_number}, $offset), 'hit');
    }

    # Step 2: Page table walk
    my $pt = $mmu->{page_tables}{$pid};
    unless ($pt) {
        return (undef, 'error');
    }

    my $pte = tpt_lookup_pte($pt, $vaddr);
    unless ($pte && $pte->{present}) {
        return (undef, 'fault');
    }

    if ($write && $pte->{cow}) {
        my ($new_paddr, $type) = _mmu_handle_cow($mmu, $pid, $vaddr);
        return ($new_paddr, 'cow');
    }

    if ($write) {
        croak "Write to read-only page at vaddr $vaddr\n"
            unless $pte->{writable};
        tpt_update_pte($pt, $vaddr, dirty => 1, accessed => 1);
    } else {
        tpt_update_pte($pt, $vaddr, accessed => 1);
    }

    # Install in TLB
    tlb_insert($mmu->{tlb}, $pid, $vpn, $pte);
    policy_record_access($mmu->{policy}, $pte->{frame_number});

    return (make_physical_address($pte->{frame_number}, $offset), 'miss');
}

# Handle a page fault — allocate frame and map the page
sub mmu_handle_page_fault {
    my ($mmu, $pid, $vaddr, %opts) = @_;
    my $frame = alloc_frame($mmu->{frame_allocator});
    croak "Out of physical memory during page fault\n" unless defined $frame;

    my $pt = $mmu->{page_tables}{$pid}
        or croak "Unknown PID $pid in page fault handler\n";

    tpt_map($pt, $vaddr, $frame, present => 1, %opts);
    $mmu->{refcounts}{$frame} = ($mmu->{refcounts}{$frame} // 0) + 1;
    policy_add_frame($mmu->{policy}, $frame);
    return $frame;
}

# Clone an address space (fork). All writable pages become COW.
sub mmu_clone_address_space {
    my ($mmu, $parent_pid, $child_pid) = @_;
    my $parent_pt = $mmu->{page_tables}{$parent_pid}
        or croak "Unknown parent PID $parent_pid\n";

    my $child_pt = new_two_level_pt();
    $mmu->{page_tables}{$child_pid} = $child_pt;

    my $mappings = tpt_all_mappings($parent_pt);
    for my $vaddr (keys %$mappings) {
        my $pte = $mappings->{$vaddr};
        next unless $pte->{present};
        my $frame = $pte->{frame_number};

        # Mark writable pages as COW in parent
        if ($pte->{writable}) {
            tpt_update_pte($parent_pt, $vaddr,
                writable => 0, cow => 1);
            my $vpn = vpn_of($vaddr);
            tlb_invalidate($mmu->{tlb}, $parent_pid, $vpn);
        }

        # Give child a COW (read-only) mapping to the same frame
        my $child_pte = new_pte($frame,
            present         => $pte->{present},
            dirty           => 0,
            accessed        => 0,
            writable        => 0,
            executable      => $pte->{executable},
            user_accessible => $pte->{user_accessible},
            cow             => ($pte->{writable} || $pte->{cow}) ? 1 : 0,
        );
        tpt_map($child_pt, $vaddr, $frame, %$child_pte);
        $mmu->{refcounts}{$frame} = ($mmu->{refcounts}{$frame} // 0) + 1;
    }

    return $mmu;
}

# ============================================================================
# Private MMU helpers
# ============================================================================

sub _mmu_decref {
    my ($mmu, $frame_number) = @_;
    my $rc = ($mmu->{refcounts}{$frame_number} // 1) - 1;
    if ($rc <= 0) {
        delete $mmu->{refcounts}{$frame_number};
        free_frame($mmu->{frame_allocator}, $frame_number);
        policy_remove_frame($mmu->{policy}, $frame_number);
    } else {
        $mmu->{refcounts}{$frame_number} = $rc;
    }
}

sub _mmu_handle_cow {
    my ($mmu, $pid, $vaddr) = @_;
    my $pt  = $mmu->{page_tables}{$pid};
    my $pte = tpt_lookup_pte($pt, $vaddr)
        or croak "COW fault: no PTE at vaddr $vaddr\n";

    my $old_frame = $pte->{frame_number};
    my $offset    = $vaddr & PAGE_OFFSET_MASK;

    # Allocate a new frame for this process's private copy
    my $new_frame = alloc_frame($mmu->{frame_allocator});
    croak "Out of physical memory during COW\n" unless defined $new_frame;

    # Update the page table entry: writable, not COW
    tpt_update_pte($pt, $vaddr,
        frame_number => $new_frame,
        writable     => 1,
        cow          => 0,
        dirty        => 1,
        accessed     => 1,
    );
    $mmu->{refcounts}{$new_frame} = 1;

    # Invalidate TLB for old mapping
    my $vpn = vpn_of($vaddr);
    tlb_invalidate($mmu->{tlb}, $pid, $vpn);

    # Decrement refcount on old frame
    _mmu_decref($mmu, $old_frame);

    # Install new mapping in TLB
    my $new_pte = tpt_lookup_pte($pt, $vaddr);
    tlb_insert($mmu->{tlb}, $pid, $vpn, $new_pte);

    return (make_physical_address($new_frame, $offset), 'cow');
}

1;

__END__

=head1 NAME

CodingAdventures::VirtualMemory - Virtual memory subsystem (PTEs, page tables, TLB, MMU) in Pure Perl

=head1 SYNOPSIS

    use CodingAdventures::VirtualMemory;

    # Create MMU with 64 physical frames, LRU replacement, 64-entry TLB
    my $mmu = CodingAdventures::VirtualMemory::new_mmu(
        total_frames => 64,
        policy_type  => 'lru',
    );

    # Create address space for process 1
    CodingAdventures::VirtualMemory::mmu_create_address_space($mmu, 1);

    # Map virtual page 0x1000 to a physical frame
    my $frame = CodingAdventures::VirtualMemory::mmu_map_page($mmu, 1, 0x1000);

    # Translate virtual address → physical address
    my ($paddr, $type) = CodingAdventures::VirtualMemory::mmu_translate($mmu, 1, 0x1042);
    # type is 'miss' on first access (TLB miss, page table hit)
    # paddr is frame*4096 + 0x042

    # Second access hits TLB
    my ($paddr2, $type2) = CodingAdventures::VirtualMemory::mmu_translate($mmu, 1, 0x1000);
    # type2 is 'hit'

=head1 VERSION

0.01

=head1 LICENSE

MIT

=cut
