//! Integration tests for the clock module.

use clock::*;
use std::cell::RefCell;
use std::rc::Rc;

// ===========================================================================
// Clock basic behavior
// ===========================================================================

#[test]
fn test_clock_alternates_values() {
    let mut clk = Clock::new(1_000_000);
    let values: Vec<u8> = (0..6).map(|_| clk.tick().value).collect();
    assert_eq!(values, vec![1, 0, 1, 0, 1, 0]);
}

#[test]
fn test_clock_cycle_increments_on_rising() {
    let mut clk = Clock::new(1_000_000);
    let cycles: Vec<u64> = (0..6).map(|_| clk.tick().cycle).collect();
    // Rising edges at ticks 0, 2, 4 -> cycles 1, 1, 2, 2, 3, 3
    assert_eq!(cycles, vec![1, 1, 2, 2, 3, 3]);
}

#[test]
fn test_clock_run_10_cycles() {
    let mut clk = Clock::new(1_000_000);
    let edges = clk.run(10);
    assert_eq!(edges.len(), 20); // 2 edges per cycle
    assert_eq!(clk.cycle(), 10);
    assert_eq!(clk.total_ticks(), 20);

    // Verify alternating pattern
    for (i, edge) in edges.iter().enumerate() {
        if i % 2 == 0 {
            assert!(edge.is_rising, "Edge {i} should be rising");
        } else {
            assert!(edge.is_falling, "Edge {i} should be falling");
        }
    }
}

#[test]
fn test_clock_full_cycle_returns_both_edges() {
    let mut clk = Clock::new(1_000_000);
    let (r, f) = clk.full_cycle();
    assert!(r.is_rising);
    assert!(!r.is_falling);
    assert!(!f.is_rising);
    assert!(f.is_falling);
    assert_eq!(r.cycle, f.cycle); // Same cycle
}

// ===========================================================================
// Clock reset
// ===========================================================================

#[test]
fn test_clock_reset_clears_state() {
    let mut clk = Clock::new(1_000_000);
    clk.run(100);
    assert_eq!(clk.cycle(), 100);

    clk.reset();
    assert_eq!(clk.cycle(), 0);
    assert_eq!(clk.value(), 0);
    assert_eq!(clk.total_ticks(), 0);

    // After reset, first tick should be rising edge cycle 1
    let edge = clk.tick();
    assert!(edge.is_rising);
    assert_eq!(edge.cycle, 1);
}

// ===========================================================================
// Clock period
// ===========================================================================

#[test]
fn test_period_ns_various_frequencies() {
    // 1 GHz -> 1 ns
    let clk = Clock::new(1_000_000_000);
    assert!((clk.period_ns() - 1.0).abs() < 0.0001);

    // 100 MHz -> 10 ns
    let clk = Clock::new(100_000_000);
    assert!((clk.period_ns() - 10.0).abs() < 0.0001);

    // 1 MHz -> 1000 ns
    let clk = Clock::new(1_000_000);
    assert!((clk.period_ns() - 1000.0).abs() < 0.0001);

    // 1 kHz -> 1_000_000 ns
    let clk = Clock::new(1_000);
    assert!((clk.period_ns() - 1_000_000.0).abs() < 0.1);
}

// ===========================================================================
// Listener pattern
// ===========================================================================

#[test]
fn test_listener_counts_rising_edges() {
    let rising_count = Rc::new(RefCell::new(0u64));
    let count_clone = rising_count.clone();

    let mut clk = Clock::new(1_000_000);
    clk.register_listener(Box::new(move |edge: &ClockEdge| {
        if edge.is_rising {
            *count_clone.borrow_mut() += 1;
        }
    }));

    clk.run(50);
    assert_eq!(*rising_count.borrow(), 50);
}

#[test]
fn test_listener_counts_all_edges() {
    let total_count = Rc::new(RefCell::new(0u64));
    let count_clone = total_count.clone();

    let mut clk = Clock::new(1_000_000);
    clk.register_listener(Box::new(move |_edge: &ClockEdge| {
        *count_clone.borrow_mut() += 1;
    }));

    clk.run(10);
    assert_eq!(*total_count.borrow(), 20); // 10 cycles * 2 edges
}

#[test]
fn test_multiple_listeners() {
    let count_a = Rc::new(RefCell::new(0u64));
    let count_b = Rc::new(RefCell::new(0u64));
    let a_clone = count_a.clone();
    let b_clone = count_b.clone();

    let mut clk = Clock::new(1_000_000);
    clk.register_listener(Box::new(move |edge: &ClockEdge| {
        if edge.is_rising {
            *a_clone.borrow_mut() += 1;
        }
    }));
    clk.register_listener(Box::new(move |edge: &ClockEdge| {
        if edge.is_falling {
            *b_clone.borrow_mut() += 1;
        }
    }));

    clk.run(5);
    assert_eq!(*count_a.borrow(), 5);
    assert_eq!(*count_b.borrow(), 5);
}

#[test]
fn test_listener_receives_correct_edge_data() {
    let edges_seen = Rc::new(RefCell::new(Vec::new()));
    let edges_clone = edges_seen.clone();

    let mut clk = Clock::new(1_000_000);
    clk.register_listener(Box::new(move |edge: &ClockEdge| {
        edges_clone.borrow_mut().push(edge.clone());
    }));

    clk.run(2);
    let seen = edges_seen.borrow();
    assert_eq!(seen.len(), 4);

    assert!(seen[0].is_rising);
    assert_eq!(seen[0].cycle, 1);
    assert!(seen[1].is_falling);
    assert_eq!(seen[1].cycle, 1);
    assert!(seen[2].is_rising);
    assert_eq!(seen[2].cycle, 2);
    assert!(seen[3].is_falling);
    assert_eq!(seen[3].cycle, 2);
}

// ===========================================================================
// ClockDivider
// ===========================================================================

#[test]
fn test_clock_divider_divide_by_2() {
    let mut source = Clock::new(1_000_000_000);
    let mut divider = ClockDivider::new(1_000_000_000, 2);

    assert_eq!(divider.output().frequency_hz(), 500_000_000);

    for _ in 0..10 {
        let (r, f) = source.full_cycle();
        divider.on_edge(&r);
        divider.on_edge(&f);
    }

    assert_eq!(divider.output().cycle(), 5); // 10 / 2 = 5
}

#[test]
fn test_clock_divider_divide_by_4() {
    let mut source = Clock::new(1_000_000_000);
    let mut divider = ClockDivider::new(1_000_000_000, 4);

    for _ in 0..16 {
        let (r, f) = source.full_cycle();
        divider.on_edge(&r);
        divider.on_edge(&f);
    }

    assert_eq!(divider.output().cycle(), 4); // 16 / 4 = 4
}

#[test]
fn test_clock_divider_only_counts_rising() {
    let mut divider = ClockDivider::new(1_000_000, 2);

    // Only falling edges — should not trigger output
    for i in 0..10 {
        divider.on_edge(&ClockEdge {
            cycle: i,
            value: 0,
            is_rising: false,
            is_falling: true,
        });
    }
    assert_eq!(divider.output().cycle(), 0);
}

// ===========================================================================
// MultiPhaseClock
// ===========================================================================

#[test]
fn test_multi_phase_4_phases_rotation() {
    let mut clk = Clock::new(1_000_000);
    let mut mpc = MultiPhaseClock::new(4);

    // Drive 4 full cycles, check phase rotation
    for expected_phase in 0..4 {
        let (r, f) = clk.full_cycle();
        mpc.on_edge(&r);
        mpc.on_edge(&f);

        // Verify only the expected phase is active
        for p in 0..4 {
            if p == expected_phase {
                assert_eq!(mpc.get_phase(p), 1, "Phase {p} should be active");
            } else {
                assert_eq!(mpc.get_phase(p), 0, "Phase {p} should be inactive");
            }
        }
    }
}

#[test]
fn test_multi_phase_wraps_around() {
    let mut clk = Clock::new(1_000_000);
    let mut mpc = MultiPhaseClock::new(3);

    // Drive 6 cycles (wraps around twice)
    for i in 0..6 {
        let (r, f) = clk.full_cycle();
        mpc.on_edge(&r);
        mpc.on_edge(&f);
        let expected = i % 3;
        assert_eq!(mpc.get_phase(expected), 1, "Cycle {i}: phase {expected} should be active");
    }
}

#[test]
fn test_multi_phase_2_phases() {
    let mut clk = Clock::new(1_000_000);
    let mut mpc = MultiPhaseClock::new(2);

    let (r, f) = clk.full_cycle();
    mpc.on_edge(&r);
    mpc.on_edge(&f);
    assert_eq!(mpc.get_phase(0), 1);
    assert_eq!(mpc.get_phase(1), 0);

    let (r, f) = clk.full_cycle();
    mpc.on_edge(&r);
    mpc.on_edge(&f);
    assert_eq!(mpc.get_phase(0), 0);
    assert_eq!(mpc.get_phase(1), 1);
}
