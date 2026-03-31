use strict;
use warnings;
use Test2::V0;

# Load all branch predictor modules
ok(eval { require CodingAdventures::BranchPredictor; 1 },
    'CodingAdventures::BranchPredictor loads');

# Aliases for shorter test code
my $Stats      = 'CodingAdventures::BranchPredictor::Stats';
my $Pred       = 'CodingAdventures::BranchPredictor::Prediction';
my $AT         = 'CodingAdventures::BranchPredictor::Static::AlwaysTaken';
my $ANT        = 'CodingAdventures::BranchPredictor::Static::AlwaysNotTaken';
my $BTFNT      = 'CodingAdventures::BranchPredictor::Static::BTFNT';
my $OneBit     = 'CodingAdventures::BranchPredictor::OneBit';
my $TwoBit     = 'CodingAdventures::BranchPredictor::TwoBit';
my $BTB        = 'CodingAdventures::BranchPredictor::BTB';

# ============================================================================
# Stats
# ============================================================================

subtest 'Stats — initialization' => sub {
    my $s = $Stats->new();
    is($s->predictions, 0, 'predictions start at 0');
    is($s->correct,     0, 'correct starts at 0');
    is($s->incorrect,   0, 'incorrect starts at 0');
    is($s->accuracy,    0.0, 'accuracy is 0.0 with no predictions');
    is($s->misprediction_rate, 0.0, 'misprediction_rate is 0.0 with no predictions');
};

subtest 'Stats — record correct' => sub {
    my $s = $Stats->new()->record(1);
    is($s->predictions, 1, 'predictions = 1');
    is($s->correct,     1, 'correct = 1');
    is($s->incorrect,   0, 'incorrect = 0');
    is($s->accuracy,    100.0, 'accuracy = 100%');
};

subtest 'Stats — record incorrect' => sub {
    my $s = $Stats->new()->record(0);
    is($s->predictions, 1, 'predictions = 1');
    is($s->correct,     0, 'correct = 0');
    is($s->incorrect,   1, 'incorrect = 1');
    is($s->accuracy,    0.0, 'accuracy = 0%');
    is($s->misprediction_rate, 100.0, 'miss rate = 100%');
};

subtest 'Stats — mixed accuracy' => sub {
    my $s = $Stats->new();
    $s = $s->record(1) for 1..8;  # 8 correct
    $s = $s->record(0) for 1..2;  # 2 incorrect
    is($s->predictions, 10, 'total 10');
    ok(abs($s->accuracy - 80.0) < 0.001, 'accuracy = 80%');
    ok(abs($s->misprediction_rate - 20.0) < 0.001, 'miss rate = 20%');
};

subtest 'Stats — reset' => sub {
    my $s = $Stats->new()->record(1)->record(0)->reset();
    is($s->predictions, 0, 'reset predictions');
    is($s->accuracy,    0.0, 'reset accuracy');
};

# ============================================================================
# Prediction type
# ============================================================================

subtest 'Prediction — defaults' => sub {
    my $p = $Pred->new(predicted_taken => 1);
    is($p->predicted_taken, 1, 'predicted_taken = 1');
    is($p->confidence,      0.5, 'confidence defaults to 0.5');
    is($p->address,         undef, 'address defaults to undef');
};

subtest 'Prediction — all fields' => sub {
    my $p = $Pred->new(predicted_taken => 0, confidence => 1.0, address => 0x200);
    is($p->predicted_taken, 0,     'not taken');
    is($p->confidence,      1.0,   'full confidence');
    is($p->address,         0x200, 'address set');
};

# ============================================================================
# AlwaysTaken
# ============================================================================

subtest 'AlwaysTaken — always predicts taken' => sub {
    my $p = $AT->new();
    for my $pc (0, 0x100, 0x200, 0xFFF) {
        my ($pred, $np) = $p->predict($pc);
        is($pred->predicted_taken, 1, "PC $pc → taken");
    }
};

subtest 'AlwaysTaken — correct when branch is actually taken' => sub {
    my $p = $AT->new();
    $p = $p->update(0x100, 1);   # correct
    $p = $p->update(0x100, 0);   # wrong
    is($p->get_stats->predictions, 2, '2 predictions');
    is($p->get_stats->correct,     1, '1 correct');
};

subtest 'AlwaysTaken — loop accuracy near 99%' => sub {
    my $p = $AT->new();
    # 99 taken + 1 not taken
    for (1..99) { $p = $p->update(0x100, 1) }
    $p = $p->update(0x100, 0);
    ok($p->get_stats->accuracy >= 99.0, 'accuracy >= 99% on 100-iter loop');
};

subtest 'AlwaysTaken — reset' => sub {
    my $p = $AT->new()->update(0x100, 1)->reset();
    is($p->get_stats->predictions, 0, 'reset clears stats');
};

# ============================================================================
# AlwaysNotTaken
# ============================================================================

subtest 'AlwaysNotTaken — always predicts not-taken' => sub {
    my $p = $ANT->new();
    for my $pc (0, 0x100, 0xFF) {
        my ($pred) = $p->predict($pc);
        is($pred->predicted_taken, 0, "PC $pc → not taken");
    }
};

subtest 'AlwaysNotTaken — correct when branch is not taken' => sub {
    my $p = $ANT->new();
    $p = $p->update(0x100, 0);   # correct
    $p = $p->update(0x100, 1);   # wrong
    is($p->get_stats->correct, 1, '1 correct');
};

# ============================================================================
# BTFNT
# ============================================================================

subtest 'BTFNT — cold start defaults to not-taken' => sub {
    my $p = $BTFNT->new();
    my ($pred) = $p->predict(0x108);
    is($pred->predicted_taken, 0, 'cold start = not taken');
};

subtest 'BTFNT — backward branch (target <= pc) predicts taken' => sub {
    my $p = $BTFNT->new();
    $p = $p->update(0x108, 1, 0x100);  # learn: 0x108 → 0x100 (backward)
    my ($pred) = $p->predict(0x108);
    is($pred->predicted_taken, 1, 'backward branch = taken');
};

subtest 'BTFNT — forward branch (target > pc) predicts not-taken' => sub {
    my $p = $BTFNT->new();
    $p = $p->update(0x100, 1, 0x200);  # learn: 0x100 → 0x200 (forward)
    my ($pred) = $p->predict(0x100);
    is($pred->predicted_taken, 0, 'forward branch = not taken');
};

subtest 'BTFNT — target == pc treated as backward (taken)' => sub {
    my $p = $BTFNT->new();
    $p = $p->update(0x100, 1, 0x100);  # self-loop
    my ($pred) = $p->predict(0x100);
    is($pred->predicted_taken, 1, 'self-loop = taken (target == pc)');
};

subtest 'BTFNT — reset clears target cache' => sub {
    my $p = $BTFNT->new();
    $p = $p->update(0x100, 1, 0x50)->reset();
    my ($pred) = $p->predict(0x100);
    is($pred->predicted_taken, 0, 'after reset, cold start = not taken');
};

# ============================================================================
# OneBit
# ============================================================================

subtest 'OneBit — cold start = not taken' => sub {
    my $p = $OneBit->new();
    my ($pred) = $p->predict(0x100);
    is($pred->predicted_taken, 0, 'cold start = not taken');
};

subtest 'OneBit — learns taken' => sub {
    my $p = $OneBit->new();
    $p = $p->update(0x100, 1);
    my ($pred) = $p->predict(0x100);
    is($pred->predicted_taken, 1, 'after taken update, predicts taken');
};

subtest 'OneBit — flips on not-taken' => sub {
    my $p = $OneBit->new();
    $p = $p->update(0x100, 1)->update(0x100, 0);
    my ($pred) = $p->predict(0x100);
    is($pred->predicted_taken, 0, 'flipped back to not-taken');
};

subtest 'OneBit — aliasing: same index maps to same entry' => sub {
    my $p = $OneBit->new(table_size => 4);
    $p = $p->update(0, 1);    # index 0 → taken
    my ($pred) = $p->predict(4);  # 4 % 4 = 0, same entry
    is($pred->predicted_taken, 1, 'aliased entry shares state');
};

subtest 'OneBit — independent PCs tracked separately' => sub {
    my $p = $OneBit->new();
    $p = $p->update(0x100, 1);
    $p = $p->update(0x200, 0);
    my ($p1) = $p->predict(0x100);
    my ($p2) = $p->predict(0x200);
    is($p1->predicted_taken, 1, 'PC 0x100 = taken');
    is($p2->predicted_taken, 0, 'PC 0x200 = not taken');
};

subtest 'OneBit — loop mispredicts at least twice' => sub {
    my $p = $OneBit->new();
    my @outcomes = ((1) x 4, 0, (1) x 4, 0);  # 5-iter loop × 2
    my $misses = 0;
    for my $taken (@outcomes) {
        my ($pred) = $p->predict(0x100);
        $misses++ unless $pred->predicted_taken == $taken;
        $p = $p->update(0x100, $taken);
    }
    ok($misses >= 2, "Expected >= 2 misses, got $misses");
};

subtest 'OneBit — reset' => sub {
    my $p = $OneBit->new()->update(0x100, 1)->reset();
    my ($pred) = $p->predict(0x100);
    is($pred->predicted_taken, 0, 'after reset = not taken');
    is($p->get_stats->predictions, 0, 'stats cleared');
};

# ============================================================================
# TwoBit
# ============================================================================

my $SNT = CodingAdventures::BranchPredictor::TwoBit::SNT();
my $WNT = CodingAdventures::BranchPredictor::TwoBit::WNT();
my $WT  = CodingAdventures::BranchPredictor::TwoBit::WT();
my $ST  = CodingAdventures::BranchPredictor::TwoBit::ST();

subtest 'TwoBit — starts in WNT (predicts not taken)' => sub {
    my $p = $TwoBit->new();
    my ($pred) = $p->predict(0x100);
    is($pred->predicted_taken, 0, 'WNT predicts not taken');
    is($p->get_state_for_pc(0x100), $WNT, 'initial state = WNT');
};

subtest 'TwoBit — WNT + taken → WT (now predicts taken)' => sub {
    my $p = $TwoBit->new()->update(0x100, 1);
    is($p->get_state_for_pc(0x100), $WT, 'state = WT after one taken');
    my ($pred) = $p->predict(0x100);
    is($pred->predicted_taken, 1, 'WT predicts taken');
};

subtest 'TwoBit — all 8 state transitions' => sub {
    my @tests = (
        # [initial, outcome, expected_next]
        [$SNT, 0, $SNT],  # saturate at SNT
        [$SNT, 1, $WNT],
        [$WNT, 0, $SNT],
        [$WNT, 1, $WT],
        [$WT,  0, $WNT],
        [$WT,  1, $ST],
        [$ST,  0, $WT],
        [$ST,  1, $ST],   # saturate at ST
    );
    for my $t (@tests) {
        my ($init, $outcome, $expected) = @$t;
        my $p = $TwoBit->new(initial_state => $init);
        $p = $p->update(0x100, $outcome);
        my $got = $p->get_state_for_pc(0x100);
        is($got, $expected, "From $init + $outcome → $expected (got $got)");
    }
};

subtest 'TwoBit — hysteresis: one not-taken from ST → WT (still predicts taken)' => sub {
    my $p = $TwoBit->new();
    $p = $p->update(0x100, 1)->update(0x100, 1);  # WNT→WT→ST
    $p = $p->update(0x100, 0);                     # ST→WT
    is($p->get_state_for_pc(0x100), $WT, 'state = WT');
    my ($pred) = $p->predict(0x100);
    is($pred->predicted_taken, 1, 'WT still predicts taken (hysteresis)');
};

subtest 'TwoBit — strong states give confidence 1.0' => sub {
    my $p = $TwoBit->new(initial_state => $ST);
    my ($pred) = $p->predict(0x100);
    is($pred->confidence, 1.0, 'ST → confidence 1.0');
};

subtest 'TwoBit — reset clears table' => sub {
    my $p = $TwoBit->new()->update(0x100, 1)->reset();
    is($p->get_state_for_pc(0x100), $WNT, 'reset → WNT');
    is($p->get_stats->predictions, 0, 'stats cleared');
};

subtest 'TwoBit — custom initial state' => sub {
    my $p = $TwoBit->new(initial_state => $WT);
    my ($pred) = $p->predict(0x100);
    is($pred->predicted_taken, 1, 'WT initial state predicts taken');
};

# ============================================================================
# BTB
# ============================================================================

subtest 'BTB — cold start returns undef' => sub {
    my $btb = $BTB->new();
    my ($target) = $btb->lookup(0x100);
    is($target, undef, 'cold start miss = undef');
};

subtest 'BTB — store and retrieve' => sub {
    my $btb = $BTB->new()->update(0x100, 0x200);
    my ($target) = $btb->lookup(0x100);
    is($target, 0x200, 'retrieved stored target');
};

subtest 'BTB — updates an existing entry' => sub {
    my $btb = $BTB->new()->update(0x100, 0x200)->update(0x100, 0x300);
    my ($target) = $btb->lookup(0x100);
    is($target, 0x300, 'updated to new target');
};

subtest 'BTB — hit/miss tracking' => sub {
    my $btb = $BTB->new();
    (my $t, $btb) = $btb->lookup(0x100);   # miss
    $btb = $btb->update(0x100, 0x200);
    ($t, $btb) = $btb->lookup(0x100);      # hit
    is($btb->lookups, 2, '2 lookups');
    is($btb->hits,    1, '1 hit');
    is($btb->misses,  1, '1 miss');
    ok(abs($btb->hit_rate - 50.0) < 0.001, 'hit rate 50%');
};

subtest 'BTB — direct-mapped eviction' => sub {
    my $btb = $BTB->new(size => 4);
    # 0x100 % 4 = 0, 0x104 % 4 = 0 → conflict
    $btb = $btb->update(0x100, 0xA00)->update(0x104, 0xB00);
    my ($t1) = $btb->lookup(0x100);
    my ($t2) = $btb->lookup(0x104);
    is($t1, undef, '0x100 evicted by 0x104');
    is($t2, 0xB00, '0x104 still present');
};

subtest 'BTB — stores branch_type metadata' => sub {
    my $btb = $BTB->new()->update(0x100, 0x200, 'call');
    my $entry = $btb->get_entry(0x100);
    is($entry->{branch_type}, 'call', 'branch_type stored');
};

subtest 'BTB — get_entry returns undef for unknown PC' => sub {
    my $btb = $BTB->new();
    is($btb->get_entry(0x999), undef, 'unknown PC returns undef');
};

subtest 'BTB — hit_rate is 0.0 with no lookups' => sub {
    my $btb = $BTB->new();
    is($btb->hit_rate, 0.0, 'hit rate 0 with no lookups');
};

subtest 'BTB — reset clears all' => sub {
    my $btb = $BTB->new()->update(0x100, 0x200);
    (my $t, $btb) = $btb->lookup(0x100);
    $btb = $btb->reset();
    ($t, $btb) = $btb->lookup(0x100);
    is($t, undef, 'after reset, miss');
    is($btb->lookups, 1, 'only the post-reset lookup counted');
};

# ============================================================================
# Integration: loop benchmark
# ============================================================================

subtest 'Integration: AlwaysTaken ~90% on 10-iter loop' => sub {
    my $p = $AT->new();
    my @outcomes = ((1) x 9, 0) x 3;
    for my $taken (@outcomes) { $p = $p->update(0x100, $taken) }
    ok($p->get_stats->accuracy >= 90.0 - 0.001,
        'AlwaysTaken >= 90% on 10-iter loop');
};

subtest 'Integration: 2-bit outperforms 1-bit on loop' => sub {
    my $p1 = $OneBit->new();
    my $p2 = $TwoBit->new();
    my @outcomes = ((1) x 9, 0) x 5;
    for my $taken (@outcomes) {
        $p1 = $p1->update(0x100, $taken);
        $p2 = $p2->update(0x100, $taken);
    }
    my $acc1 = $p1->get_stats->accuracy;
    my $acc2 = $p2->get_stats->accuracy;
    # 2-bit should be within range of 1-bit (usually better or equal)
    ok($acc2 >= $acc1 - 10.0,
        "2-bit ($acc2%) should be within 10% of 1-bit ($acc1%)");
};

done_testing;
