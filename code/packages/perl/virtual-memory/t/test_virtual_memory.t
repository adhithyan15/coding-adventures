use strict;
use warnings;
use Test2::V0;

use lib 'lib';
use CodingAdventures::VirtualMemory;

# Convenience aliases
{
    no strict 'refs';
    for my $fn (qw(
        PAGE_SIZE PAGE_OFFSET_BITS PAGE_OFFSET_MASK DEFAULT_TLB_CAP
        new_pte
        new_page_table pt_map pt_lookup pt_unmap pt_mapped_count pt_update_pte pt_all_mappings
        split_address vpn_of make_physical_address
        new_two_level_pt tpt_map tpt_translate tpt_lookup_pte tpt_unmap tpt_update_pte
        tpt_all_mappings tpt_mapped_count
        new_tlb tlb_lookup tlb_insert tlb_invalidate tlb_flush tlb_hit_rate tlb_size
        new_frame_allocator alloc_frame free_frame frame_is_allocated
        free_frame_count allocated_frame_count
        new_fifo_policy new_lru_policy new_clock_policy
        policy_add_frame policy_record_access policy_select_victim policy_remove_frame
        new_mmu mmu_create_address_space mmu_destroy_address_space
        mmu_map_page mmu_translate mmu_handle_page_fault mmu_clone_address_space
    )) {
        *{"main::$fn"} = \&{"CodingAdventures::VirtualMemory::$fn"};
    }
}

# ============================================================================
# Constants
# ============================================================================

subtest 'constants are correct' => sub {
    is(PAGE_SIZE(),        4096, 'PAGE_SIZE = 4096');
    is(PAGE_OFFSET_BITS(), 12,   'PAGE_OFFSET_BITS = 12');
    is(PAGE_OFFSET_MASK(), 0xFFF, 'PAGE_OFFSET_MASK = 0xFFF');
    is(DEFAULT_TLB_CAP(),  64,   'DEFAULT_TLB_CAP = 64');
};

# ============================================================================
# PageTableEntry
# ============================================================================

subtest 'new_pte creates PTE with defaults' => sub {
    my $pte = new_pte(42);
    is($pte->{frame_number},    42, 'frame_number');
    is($pte->{present},          1, 'present default = 1');
    is($pte->{dirty},            0, 'dirty default = 0');
    is($pte->{accessed},         0, 'accessed default = 0');
    is($pte->{writable},         1, 'writable default = 1');
    is($pte->{executable},       0, 'executable default = 0');
    is($pte->{user_accessible},  1, 'user_accessible default = 1');
};

subtest 'new_pte accepts explicit flags' => sub {
    my $pte = new_pte(7,
        present => 0, dirty => 1, writable => 0, executable => 1);
    is($pte->{present},    0, 'present = 0');
    is($pte->{dirty},      1, 'dirty = 1');
    is($pte->{writable},   0, 'writable = 0');
    is($pte->{executable}, 1, 'executable = 1');
};

# ============================================================================
# Single-Level Page Table
# ============================================================================

subtest 'page table map and lookup' => sub {
    my $pt = new_page_table();
    pt_map($pt, 0, 5);
    pt_map($pt, 1, 7);
    is(pt_lookup($pt, 0)->{frame_number}, 5, 'vpn 0 → frame 5');
    is(pt_lookup($pt, 1)->{frame_number}, 7, 'vpn 1 → frame 7');
    is(pt_lookup($pt, 2), undef, 'unmapped vpn returns undef');
};

subtest 'page table unmap' => sub {
    my $pt = new_page_table();
    pt_map($pt, 0, 5);
    pt_unmap($pt, 0);
    is(pt_lookup($pt, 0), undef, 'unmap removes entry');
};

subtest 'pt_mapped_count' => sub {
    my $pt = new_page_table();
    is(pt_mapped_count($pt), 0, 'empty table has 0 mappings');
    pt_map($pt, 0, 1); pt_map($pt, 1, 2); pt_map($pt, 2, 3);
    is(pt_mapped_count($pt), 3, '3 mappings');
    pt_unmap($pt, 1);
    is(pt_mapped_count($pt), 2, '2 after unmap');
};

subtest 'pt_update_pte updates fields' => sub {
    my $pt = new_page_table();
    pt_map($pt, 0, 5);
    pt_update_pte($pt, 0, dirty => 1, accessed => 1);
    my $pte = pt_lookup($pt, 0);
    is($pte->{dirty},    1, 'dirty updated');
    is($pte->{accessed}, 1, 'accessed updated');
};

subtest 'pt_all_mappings returns all PTEs' => sub {
    my $pt = new_page_table();
    pt_map($pt, 0, 10); pt_map($pt, 1, 20);
    my $all = pt_all_mappings($pt);
    is(scalar keys %$all, 2, '2 mappings returned');
};

# ============================================================================
# Address Decomposition
# ============================================================================

subtest 'split_address decomposes Sv32 address' => sub {
    # 0x00000000: vpn1=0, vpn0=0, offset=0
    my ($vpn1, $vpn0, $offset) = split_address(0x00000000);
    is($vpn1, 0, 'vpn1=0');
    is($vpn0, 0, 'vpn0=0');
    is($offset, 0, 'offset=0');
};

subtest 'split_address with page offset' => sub {
    # 0x00001042: vpn1=0, vpn0=1, offset=0x042
    my ($vpn1, $vpn0, $offset) = split_address(0x00001042);
    is($vpn1,   0,     'vpn1=0');
    is($vpn0,   1,     'vpn0=1');
    is($offset, 0x042, 'offset=0x042');
};

subtest 'split_address with vpn1 set' => sub {
    # 0x00400000 = bit 22 set: vpn1=1, vpn0=0, offset=0
    my ($vpn1, $vpn0, $offset) = split_address(0x00400000);
    is($vpn1,   1, 'vpn1=1');
    is($vpn0,   0, 'vpn0=0');
    is($offset, 0, 'offset=0');
};

subtest 'split_address max address' => sub {
    # 0xFFFFFFFF: vpn1=0x3FF, vpn0=0x3FF, offset=0xFFF
    my ($vpn1, $vpn0, $offset) = split_address(0xFFFFFFFF);
    is($vpn1,   0x3FF, 'vpn1=0x3FF');
    is($vpn0,   0x3FF, 'vpn0=0x3FF');
    is($offset, 0xFFF, 'offset=0xFFF');
};

subtest 'vpn_of extracts VPN' => sub {
    is(vpn_of(0x1000), 1,  'vpn of page 1');
    is(vpn_of(0x2FFF), 2,  'vpn of address in page 2');
    is(vpn_of(0x0000), 0,  'vpn of page 0');
};

subtest 'make_physical_address reconstructs paddr' => sub {
    is(make_physical_address(0, 0),      0,      'frame 0 offset 0');
    is(make_physical_address(1, 0),      0x1000, 'frame 1 offset 0');
    is(make_physical_address(1, 0x042),  0x1042, 'frame 1 offset 0x042');
    is(make_physical_address(5, 0xFFF),  0x5FFF, 'frame 5 max offset');
};

# ============================================================================
# Two-Level Page Table
# ============================================================================

subtest 'tpt_map and tpt_translate' => sub {
    my $tpt = new_two_level_pt();
    tpt_map($tpt, 0x1000, 5);
    my $paddr = tpt_translate($tpt, 0x1042);
    is($paddr, make_physical_address(5, 0x042), 'translated correctly');
};

subtest 'tpt_translate returns undef for unmapped address' => sub {
    my $tpt = new_two_level_pt();
    is(tpt_translate($tpt, 0x1000), undef, 'unmapped returns undef');
};

subtest 'tpt_translate returns undef for not-present PTE' => sub {
    my $tpt = new_two_level_pt();
    tpt_map($tpt, 0x1000, 5, present => 0);
    is(tpt_translate($tpt, 0x1000), undef, 'not-present PTE returns undef');
};

subtest 'tpt_unmap removes mapping' => sub {
    my $tpt = new_two_level_pt();
    tpt_map($tpt, 0x1000, 5);
    tpt_unmap($tpt, 0x1000);
    is(tpt_translate($tpt, 0x1000), undef, 'unmapped after tpt_unmap');
};

subtest 'tpt_lookup_pte returns PTE' => sub {
    my $tpt = new_two_level_pt();
    tpt_map($tpt, 0x2000, 7, dirty => 1);
    my $pte = tpt_lookup_pte($tpt, 0x2000);
    is($pte->{frame_number}, 7, 'correct frame');
    is($pte->{dirty},        1, 'dirty flag preserved');
};

subtest 'tpt_update_pte updates fields' => sub {
    my $tpt = new_two_level_pt();
    tpt_map($tpt, 0x3000, 9);
    tpt_update_pte($tpt, 0x3000, dirty => 1, accessed => 1);
    my $pte = tpt_lookup_pte($tpt, 0x3000);
    is($pte->{dirty},    1, 'dirty updated');
    is($pte->{accessed}, 1, 'accessed updated');
};

subtest 'tpt_mapped_count' => sub {
    my $tpt = new_two_level_pt();
    is(tpt_mapped_count($tpt), 0, 'empty = 0');
    tpt_map($tpt, 0x1000, 1);
    tpt_map($tpt, 0x2000, 2);
    tpt_map($tpt, 0x400000, 3);  # different vpn1
    is(tpt_mapped_count($tpt), 3, '3 mappings');
};

subtest 'tpt_all_mappings' => sub {
    my $tpt = new_two_level_pt();
    tpt_map($tpt, 0x1000, 1);
    tpt_map($tpt, 0x2000, 2);
    my $all = tpt_all_mappings($tpt);
    is(scalar keys %$all, 2, '2 mappings');
};

# ============================================================================
# TLB
# ============================================================================

subtest 'tlb_insert and tlb_lookup hit' => sub {
    my $tlb = new_tlb(8);
    my $pte = new_pte(5);
    tlb_insert($tlb, 1, 1, $pte);
    my $entry = tlb_lookup($tlb, 1, 1);
    ok(defined $entry, 'TLB hit');
    is($entry->{frame_number}, 5, 'correct frame');
};

subtest 'tlb_lookup miss on unknown entry' => sub {
    my $tlb = new_tlb(8);
    is(tlb_lookup($tlb, 1, 99), undef, 'miss for unknown vpn');
};

subtest 'tlb_hit_rate tracking' => sub {
    my $tlb = new_tlb(8);
    my $pte = new_pte(1);
    tlb_insert($tlb, 1, 0, $pte);
    tlb_lookup($tlb, 1, 0);  # hit
    tlb_lookup($tlb, 1, 1);  # miss
    my $rate = tlb_hit_rate($tlb);
    ok($rate > 0 && $rate < 1, 'hit rate between 0 and 1');
    is(sprintf("%.2f", $rate), '0.50', '50% hit rate');
};

subtest 'tlb evicts LRU when at capacity' => sub {
    my $tlb = new_tlb(2);  # capacity = 2
    tlb_insert($tlb, 1, 0, new_pte(0));
    tlb_insert($tlb, 1, 1, new_pte(1));
    # Access vpn 0 to make it recently used
    tlb_lookup($tlb, 1, 0);
    # Insert vpn 2 — should evict vpn 1 (LRU)
    tlb_insert($tlb, 1, 2, new_pte(2));
    is(tlb_size($tlb), 2, 'still at capacity');
    ok(defined tlb_lookup($tlb, 1, 0), 'vpn 0 still present');
    # vpn 1 may or may not be present depending on LRU order
    ok(defined tlb_lookup($tlb, 1, 2), 'vpn 2 inserted');
};

subtest 'tlb_invalidate removes single entry' => sub {
    my $tlb = new_tlb(8);
    tlb_insert($tlb, 1, 0, new_pte(0));
    tlb_insert($tlb, 1, 1, new_pte(1));
    tlb_invalidate($tlb, 1, 0);
    is(tlb_lookup($tlb, 1, 0), undef, 'invalidated entry gone');
    ok(defined tlb_lookup($tlb, 1, 1), 'other entry still present');
};

subtest 'tlb_flush with pid clears only that process' => sub {
    my $tlb = new_tlb(16);
    tlb_insert($tlb, 1, 0, new_pte(0));
    tlb_insert($tlb, 2, 0, new_pte(1));
    tlb_flush($tlb, 1);
    is(tlb_lookup($tlb, 1, 0), undef, 'pid 1 entry flushed');
    ok(defined tlb_lookup($tlb, 2, 0), 'pid 2 entry remains');
};

subtest 'tlb_flush without pid clears all' => sub {
    my $tlb = new_tlb(16);
    tlb_insert($tlb, 1, 0, new_pte(0));
    tlb_insert($tlb, 2, 0, new_pte(1));
    tlb_flush($tlb);
    is(tlb_size($tlb), 0, 'all entries cleared');
};

subtest 'tlb_size returns entry count' => sub {
    my $tlb = new_tlb(16);
    is(tlb_size($tlb), 0, 'empty TLB');
    tlb_insert($tlb, 1, 0, new_pte(0));
    tlb_insert($tlb, 1, 1, new_pte(1));
    is(tlb_size($tlb), 2, '2 entries');
};

# ============================================================================
# Frame Allocator
# ============================================================================

subtest 'alloc_frame allocates frames sequentially' => sub {
    my $fa = new_frame_allocator(4);
    is(alloc_frame($fa), 0, 'first frame = 0');
    is(alloc_frame($fa), 1, 'second frame = 1');
    is(alloc_frame($fa), 2, 'third frame = 2');
    is(alloc_frame($fa), 3, 'fourth frame = 3');
};

subtest 'alloc_frame returns undef when exhausted' => sub {
    my $fa = new_frame_allocator(1);
    alloc_frame($fa);
    is(alloc_frame($fa), undef, 'no more frames');
};

subtest 'free_frame makes frame available again' => sub {
    my $fa = new_frame_allocator(2);
    alloc_frame($fa); alloc_frame($fa);
    free_frame($fa, 0);
    is(alloc_frame($fa), 0, 'freed frame re-allocated');
};

subtest 'frame_is_allocated reports correctly' => sub {
    my $fa = new_frame_allocator(4);
    is(frame_is_allocated($fa, 0), 0, 'not allocated yet');
    alloc_frame($fa);
    is(frame_is_allocated($fa, 0), 1, 'now allocated');
    free_frame($fa, 0);
    is(frame_is_allocated($fa, 0), 0, 'freed = not allocated');
};

subtest 'double-free dies' => sub {
    my $fa = new_frame_allocator(4);
    alloc_frame($fa);
    free_frame($fa, 0);
    ok(dies { free_frame($fa, 0) }, 'double-free dies');
};

subtest 'free_frame_count and allocated_frame_count' => sub {
    my $fa = new_frame_allocator(4);
    is(free_frame_count($fa),      4, 'all frames free initially');
    is(allocated_frame_count($fa), 0, 'none allocated initially');
    alloc_frame($fa); alloc_frame($fa);
    is(free_frame_count($fa),      2, '2 frames free');
    is(allocated_frame_count($fa), 2, '2 frames allocated');
};

# ============================================================================
# Replacement Policies — FIFO
# ============================================================================

subtest 'FIFO selects oldest frame' => sub {
    my $p = new_fifo_policy();
    policy_add_frame($p, 10);
    policy_add_frame($p, 20);
    policy_add_frame($p, 30);
    is(policy_select_victim($p), 10, 'oldest frame is victim');
    # FIFO ignores access order
    policy_record_access($p, 10);
    is(policy_select_victim($p), 10, 'FIFO still picks oldest');
};

subtest 'FIFO remove_frame removes from queue' => sub {
    my $p = new_fifo_policy();
    policy_add_frame($p, 1);
    policy_add_frame($p, 2);
    policy_remove_frame($p, 1);
    is(policy_select_victim($p), 2, 'after removing 1, victim is 2');
};

subtest 'FIFO select_victim on empty returns undef' => sub {
    my $p = new_fifo_policy();
    is(policy_select_victim($p), undef, 'empty policy returns undef');
};

# ============================================================================
# Replacement Policies — LRU
# ============================================================================

subtest 'LRU selects least recently used' => sub {
    my $p = new_lru_policy();
    policy_add_frame($p, 1);
    policy_add_frame($p, 2);
    policy_add_frame($p, 3);
    policy_record_access($p, 1);  # 1 is now most recently used
    policy_record_access($p, 3);  # 3 is now MRU
    # Order: 2 (LRU), 1, 3 (MRU)
    is(policy_select_victim($p), 2, 'frame 2 is LRU');
};

subtest 'LRU remove_frame' => sub {
    my $p = new_lru_policy();
    policy_add_frame($p, 1);
    policy_add_frame($p, 2);
    policy_remove_frame($p, 1);
    is(policy_select_victim($p), 2, 'only frame 2 remains');
};

# ============================================================================
# Replacement Policies — Clock
# ============================================================================

subtest 'Clock clears use bits and evicts' => sub {
    my $p = new_clock_policy();
    policy_add_frame($p, 1);
    policy_add_frame($p, 2);
    policy_add_frame($p, 3);
    # All start with use_bit=1 (freshly added)
    # First sweep: clear use bits; second time evict first with bit=0
    my $v = policy_select_victim($p);
    ok(defined $v, 'clock selects a victim');
};

subtest 'Clock evicts frame with use bit = 0' => sub {
    my $p = new_clock_policy();
    policy_add_frame($p, 1);
    policy_add_frame($p, 2);
    # Set frame 1 use bit to 0 (not accessed recently)
    $p->{use_bits}{1} = 0;
    $p->{hand} = 0;  # hand points at frame 1
    my $v = policy_select_victim($p);
    is($v, 1, 'evicts frame 1 with use bit = 0');
};

subtest 'Clock remove_frame' => sub {
    my $p = new_clock_policy();
    policy_add_frame($p, 1);
    policy_add_frame($p, 2);
    policy_remove_frame($p, 1);
    ok(!grep { $_ == 1 } @{$p->{frames}}, 'frame 1 removed');
};

# ============================================================================
# MMU — Full System
# ============================================================================

subtest 'mmu_create_address_space creates PID entry' => sub {
    my $mmu = new_mmu(total_frames => 16, policy_type => 'lru');
    mmu_create_address_space($mmu, 1);
    ok(exists $mmu->{page_tables}{1}, 'page table for PID 1 created');
};

subtest 'mmu_map_page allocates frame and maps it' => sub {
    my $mmu = new_mmu(total_frames => 16, policy_type => 'lru');
    mmu_create_address_space($mmu, 1);
    my $frame = mmu_map_page($mmu, 1, 0x1000);
    ok(defined $frame, 'frame allocated');
    is(frame_is_allocated($mmu->{frame_allocator}, $frame), 1, 'frame marked allocated');
};

subtest 'mmu_translate TLB miss then TLB hit' => sub {
    my $mmu = new_mmu(total_frames => 16, policy_type => 'lru');
    mmu_create_address_space($mmu, 1);
    my $frame = mmu_map_page($mmu, 1, 0x1000);

    # First access: TLB miss, page table hit
    my ($paddr1, $type1) = mmu_translate($mmu, 1, 0x1042);
    is($type1, 'miss', 'first access is TLB miss');
    is($paddr1, make_physical_address($frame, 0x042), 'correct physical address');

    # Second access: TLB hit
    my ($paddr2, $type2) = mmu_translate($mmu, 1, 0x1100);
    is($type2, 'hit', 'second access is TLB hit');
    is($paddr2, make_physical_address($frame, 0x100), 'correct physical address');
};

subtest 'mmu_translate page fault for unmapped address' => sub {
    my $mmu = new_mmu(total_frames => 16, policy_type => 'lru');
    mmu_create_address_space($mmu, 1);
    my ($paddr, $type) = mmu_translate($mmu, 1, 0x5000);
    is($type, 'fault', 'unmapped address returns fault');
    is($paddr, undef, 'no physical address on fault');
};

subtest 'mmu_handle_page_fault allocates and maps frame' => sub {
    my $mmu = new_mmu(total_frames => 16, policy_type => 'lru');
    mmu_create_address_space($mmu, 1);
    my ($paddr, $type) = mmu_translate($mmu, 1, 0x5000);
    is($type, 'fault', 'fault first');
    my $frame = mmu_handle_page_fault($mmu, 1, 0x5000);
    ok(defined $frame, 'frame allocated for fault');
    my ($paddr2, $type2) = mmu_translate($mmu, 1, 0x5042);
    is($type2, 'miss', 'next access is miss (page now present)');
    ok(defined $paddr2, 'physical address now available');
};

subtest 'mmu_translate write sets dirty flag' => sub {
    my $mmu = new_mmu(total_frames => 16, policy_type => 'lru');
    mmu_create_address_space($mmu, 1);
    mmu_map_page($mmu, 1, 0x1000);
    mmu_translate($mmu, 1, 0x1000, write => 1);
    my $pte = tpt_lookup_pte($mmu->{page_tables}{1}, 0x1000);
    is($pte->{dirty},    1, 'dirty bit set on write');
    is($pte->{accessed}, 1, 'accessed bit set on write');
};

subtest 'mmu_destroy_address_space frees frames' => sub {
    my $mmu = new_mmu(total_frames => 16, policy_type => 'lru');
    mmu_create_address_space($mmu, 1);
    mmu_map_page($mmu, 1, 0x1000);
    mmu_map_page($mmu, 1, 0x2000);
    is(allocated_frame_count($mmu->{frame_allocator}), 2, '2 frames before destroy');
    mmu_destroy_address_space($mmu, 1);
    is(allocated_frame_count($mmu->{frame_allocator}), 0, '0 frames after destroy');
};

subtest 'mmu_destroy_address_space flushes TLB' => sub {
    my $mmu = new_mmu(total_frames => 16, policy_type => 'lru');
    mmu_create_address_space($mmu, 1);
    mmu_map_page($mmu, 1, 0x1000);
    mmu_translate($mmu, 1, 0x1000);  # populate TLB
    mmu_destroy_address_space($mmu, 1);
    is(tlb_size($mmu->{tlb}), 0, 'TLB empty after destroy');
};

# ============================================================================
# MMU — Copy-on-Write Fork
# ============================================================================

subtest 'mmu_clone_address_space shares frames between parent and child' => sub {
    my $mmu = new_mmu(total_frames => 32, policy_type => 'lru');
    mmu_create_address_space($mmu, 1);
    my $frame = mmu_map_page($mmu, 1, 0x1000);

    mmu_clone_address_space($mmu, 1, 2);

    # Both should translate to same physical address
    my ($paddr_child, $type_child) = mmu_translate($mmu, 2, 0x1000);
    ok(defined $paddr_child, 'child can access mapped page');
    is($paddr_child, make_physical_address($frame, 0), 'child maps to same frame');
};

subtest 'mmu_clone sets COW flag on writable pages' => sub {
    my $mmu = new_mmu(total_frames => 32, policy_type => 'lru');
    mmu_create_address_space($mmu, 1);
    mmu_map_page($mmu, 1, 0x1000, writable => 1);
    mmu_clone_address_space($mmu, 1, 2);

    # Parent page should now be COW (read-only)
    my $parent_pte = tpt_lookup_pte($mmu->{page_tables}{1}, 0x1000);
    is($parent_pte->{cow}, 1, 'parent page marked COW after fork');

    # Child page should also be COW
    my $child_pte = tpt_lookup_pte($mmu->{page_tables}{2}, 0x1000);
    is($child_pte->{cow}, 1, 'child page marked COW after fork');
};

subtest 'COW write triggers copy and gives private frame' => sub {
    my $mmu = new_mmu(total_frames => 32, policy_type => 'lru');
    mmu_create_address_space($mmu, 1);
    my $orig_frame = mmu_map_page($mmu, 1, 0x1000, writable => 1);
    mmu_clone_address_space($mmu, 1, 2);

    is(allocated_frame_count($mmu->{frame_allocator}), 1, '1 frame before COW');

    # Child writes to the page — triggers COW
    my ($paddr, $type) = mmu_translate($mmu, 2, 0x1000, write => 1);
    is($type, 'cow', 'COW triggered');

    # Now child has its own frame
    is(allocated_frame_count($mmu->{frame_allocator}), 2, '2 frames after COW');
    my $child_pte = tpt_lookup_pte($mmu->{page_tables}{2}, 0x1000);
    ok($child_pte->{frame_number} != $orig_frame, 'child has different frame after COW');
    is($child_pte->{cow},      0, 'child COW flag cleared');
    is($child_pte->{writable}, 1, 'child page now writable');
};

# ============================================================================
# MMU — Multiple processes
# ============================================================================

subtest 'multiple processes can have independent address spaces' => sub {
    my $mmu = new_mmu(total_frames => 32, policy_type => 'fifo');
    mmu_create_address_space($mmu, 1);
    mmu_create_address_space($mmu, 2);
    my $f1 = mmu_map_page($mmu, 1, 0x1000);
    my $f2 = mmu_map_page($mmu, 2, 0x1000);
    ok($f1 != $f2, 'different processes get different frames for same vaddr');

    my ($p1) = mmu_translate($mmu, 1, 0x1000);
    my ($p2) = mmu_translate($mmu, 2, 0x1000);
    is($p1, make_physical_address($f1, 0), 'PID 1 translates to its frame');
    is($p2, make_physical_address($f2, 0), 'PID 2 translates to its frame');
};

# ============================================================================
# MMU — Clock and FIFO policies
# ============================================================================

subtest 'MMU works with clock policy' => sub {
    my $mmu = new_mmu(total_frames => 8, policy_type => 'clock');
    mmu_create_address_space($mmu, 1);
    my $frame = mmu_map_page($mmu, 1, 0x1000);
    my ($paddr, $type) = mmu_translate($mmu, 1, 0x1000);
    is($type, 'miss', 'first access is miss');
};

subtest 'MMU works with fifo policy' => sub {
    my $mmu = new_mmu(total_frames => 8, policy_type => 'fifo');
    mmu_create_address_space($mmu, 1);
    my $frame = mmu_map_page($mmu, 1, 0x2000);
    my ($paddr, $type) = mmu_translate($mmu, 1, 0x2FFF);
    ok(defined $paddr, 'FIFO policy translates correctly');
};

done_testing;
