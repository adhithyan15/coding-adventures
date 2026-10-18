//! Exact reviewed former25-unit refusal occurrences; complete sources/oracles.
//! Current cap24 must refuse every row before the dedicated repair.
use super::*;
use std::sync::{Arc, Mutex};

#[test]
fn native25_exact_original_occurrence_sources_and_typed_results() {
    let mut refused = Vec::new();
    for (id, source, expected_stdout, expected_return) in [
        ("old-26", "begin boolean procedure choose; choose := true; real procedure pick; pick := 2.25; real procedure relay(x); real x; relay := x; real left, right, result; left := pick(); right := pick(); result := if choose() then relay(entier(entier(sign(left)) + 25)) else relay(entier(entier(sign(right)) + 25)); output(result) end", "26", Value::Float(26.0)),
        ("old-27", "begin boolean procedure choose; choose := true; real procedure pick; pick := 2.25; real procedure relay(x); real x; relay := x; real left, right, result; left := pick(); right := pick(); result := if choose() then relay(entier(entier(sign(left)) + 24 + 1)) else relay(entier(entier(sign(right)) + 1 + 24)); output(result) end", "26", Value::Float(26.0)),
        ("old-29", "begin boolean procedure choose; choose := true; real procedure pick; pick := 2.25; real procedure relay(x); real x; relay := x; real left, right, result; left := pick(); right := pick(); result := if choose() then relay(entier(entier(sign(left)) + 25)) else relay(entier(entier(sign(right)) + 25)); output(result) end", "26", Value::Float(26.0)),
        ("old-30", "begin boolean procedure choose; choose := true; real procedure pick; pick := 2.25; real procedure relay(x); real x; relay := x; real left, right, result; left := pick(); right := pick(); result := if choose() then relay(entier(entier(sign(left)) + 24 + 1)) else relay(entier(entier(sign(right)) + 1 + 24)); output(result) end", "26", Value::Float(26.0)),
        ("old-32", "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 25); output(saved) end", "26", Value::Int(0)),
        ("old-33", "begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) + 17); output(saved) end", "26", Value::Int(0)),
        ("old-34", "begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (24 + sign(pick()))); output(saved) end", "26", Value::Int(0)),
        ("old-35", "begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 24); output(saved) end", "26", Value::Int(0)),
        ("old-36", "begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 23); output(saved) end", "26", Value::Int(0)),
        ("old-47", "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 25); output(saved) end", "26", Value::Int(0)),
        ("old-48", "begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 11) + 14); output(saved) end", "26", Value::Int(0)),
        ("old-49", "begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (24 + sign(pick()))); output(saved) end", "26", Value::Int(0)),
        ("old-50", "begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 24); output(saved) end", "26", Value::Int(0)),
        ("old-51", "begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 23); output(saved) end", "26", Value::Int(0)),
        ("old-76", "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 24 - 1); output(saved) end", "24", Value::Int(0)),
        ("old-77", "begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(entier(sign(pick()) + 24) + 1)); output(saved) end", "26", Value::Int(0)),
        ("old-78", "begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(abs(sign(pick()) + 24) + 1)); output(saved) end", "26", Value::Int(0)),
        ("old-82", "begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 25) else (entier(sign(right)) + 1)); output(saved) end", "24", Value::Int(0)),
        ("old-83", "begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 1) else (entier(sign(right)) + 25)); output(saved) end", "0", Value::Int(0)),
        ("old-84", "begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 25) else (entier(sign(right)) + 1)); output(saved) end", "2", Value::Int(0)),
        ("old-85", "begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 1) else (entier(sign(right)) + 25)); output(saved) end", "26", Value::Int(0)),
        ("old-86", "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 25); output(saved) end", "26", Value::Int(0)),
        ("old-87", "begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) + 17); output(saved) end", "26", Value::Int(0)),
        ("old-88", "begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (24 + sign(pick()))); output(saved) end", "26", Value::Int(0)),
        ("old-89", "begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 24); output(saved) end", "26", Value::Int(0)),
        ("old-90", "begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 23); output(saved) end", "26", Value::Int(0)),
        ("old-102", "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 25); output(saved) end", "26", Value::Int(0)),
        ("old-103", "begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) + 17); output(saved) end", "26", Value::Int(0)),
        ("old-104", "begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (24 + sign(pick()))); output(saved) end", "26", Value::Int(0)),
        ("old-105", "begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 24); output(saved) end", "26", Value::Int(0)),
        ("old-106", "begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 23); output(saved) end", "26", Value::Int(0)),
        ("old-117", "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 24 - 1); output(saved) end", "24", Value::Int(0)),
        ("old-118", "begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(entier(sign(pick()) + 24) + 1)); output(saved) end", "26", Value::Int(0)),
        ("old-119", "begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(abs(sign(pick()) + 24) + 1)); output(saved) end", "26", Value::Int(0)),
        ("old-123", "begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 25) else (entier(sign(right)) + 1)); output(saved) end", "24", Value::Int(0)),
        ("old-124", "begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 1) else (entier(sign(right)) + 25)); output(saved) end", "0", Value::Int(0)),
        ("old-125", "begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 25) else (entier(sign(right)) + 1)); output(saved) end", "2", Value::Int(0)),
        ("old-126", "begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 1) else (entier(sign(right)) + 25)); output(saved) end", "26", Value::Int(0)),
        ("old-127", "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 25); output(saved) end", "26", Value::Int(0)),
        ("old-128", "begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) + 17); output(saved) end", "26", Value::Int(0)),
        ("old-129", "begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (24 + sign(pick()))); output(saved) end", "26", Value::Int(0)),
        ("old-130", "begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 24); output(saved) end", "26", Value::Int(0)),
        ("old-131", "begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 23); output(saved) end", "26", Value::Int(0)),
        ("old-142", "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 25); output(saved) end", "26", Value::Int(0)),
        ("old-143", "begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) + 17); output(saved) end", "26", Value::Int(0)),
        ("old-144", "begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (24 + sign(pick()))); output(saved) end", "26", Value::Int(0)),
        ("old-145", "begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 24); output(saved) end", "26", Value::Int(0)),
        ("old-146", "begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 23); output(saved) end", "26", Value::Int(0)),
        ("old-157", "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 25); output(saved) end", "26", Value::Int(0)),
        ("old-158", "begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) + 17); output(saved) end", "26", Value::Int(0)),
        ("old-159", "begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (24 + sign(pick()))); output(saved) end", "26", Value::Int(0)),
        ("old-160", "begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 24); output(saved) end", "26", Value::Int(0)),
        ("old-161", "begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 23); output(saved) end", "26", Value::Int(0)),
        ("old-172", "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 24 - 1); output(saved) end", "24", Value::Int(0)),
        ("old-173", "begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(entier(sign(pick()) + 24) + 1)); output(saved) end", "26", Value::Int(0)),
        ("old-174", "begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(abs(sign(pick()) + 24) + 1)); output(saved) end", "26", Value::Int(0)),
        ("old-178", "begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 25) else (entier(sign(right)) + 1)); output(saved) end", "24", Value::Int(0)),
        ("old-179", "begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 1) else (entier(sign(right)) + 25)); output(saved) end", "0", Value::Int(0)),
        ("old-180", "begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 25) else (entier(sign(right)) + 1)); output(saved) end", "2", Value::Int(0)),
        ("old-181", "begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 1) else (entier(sign(right)) + 25)); output(saved) end", "26", Value::Int(0)),
        ("old-182", "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 25); output(saved) end", "26", Value::Int(0)),
        ("old-183", "begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) + 17); output(saved) end", "26", Value::Int(0)),
        ("old-184", "begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (24 + sign(pick()))); output(saved) end", "26", Value::Int(0)),
        ("old-185", "begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 24); output(saved) end", "26", Value::Int(0)),
        ("old-186", "begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 23); output(saved) end", "26", Value::Int(0)),
        ("old-197", "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 25); output(saved) end", "26", Value::Int(0)),
        ("old-198", "begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) + 17); output(saved) end", "26", Value::Int(0)),
        ("old-199", "begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (24 + sign(pick()))); output(saved) end", "26", Value::Int(0)),
        ("old-200", "begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 24); output(saved) end", "26", Value::Int(0)),
        ("old-201", "begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 23); output(saved) end", "26", Value::Int(0)),
        ("old-212", "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 25); output(saved) end", "26", Value::Int(0)),
        ("old-213", "begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) + 17); output(saved) end", "26", Value::Int(0)),
        ("old-214", "begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (24 + sign(pick()))); output(saved) end", "26", Value::Int(0)),
        ("old-215", "begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 24); output(saved) end", "26", Value::Int(0)),
        ("old-216", "begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 23); output(saved) end", "26", Value::Int(0)),
        ("old-227", "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 25); output(saved) end", "26", Value::Int(0)),
        ("old-228", "begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) + 17); output(saved) end", "26", Value::Int(0)),
        ("old-229", "begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (24 + sign(pick()))); output(saved) end", "26", Value::Int(0)),
        ("old-230", "begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 24); output(saved) end", "26", Value::Int(0)),
        ("old-231", "begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 23); output(saved) end", "26", Value::Int(0)),
        ("old-242", "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 24 - 1); output(saved) end", "24", Value::Int(0)),
        ("old-243", "begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(entier(sign(pick()) + 24) + 1)); output(saved) end", "26", Value::Int(0)),
        ("old-244", "begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(abs(sign(pick()) + 24) + 1)); output(saved) end", "26", Value::Int(0)),
        ("old-248", "begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 25) else (entier(sign(right)) + 1)); output(saved) end", "24", Value::Int(0)),
        ("old-249", "begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 1) else (entier(sign(right)) + 25)); output(saved) end", "0", Value::Int(0)),
        ("old-250", "begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 25) else (entier(sign(right)) + 1)); output(saved) end", "2", Value::Int(0)),
        ("old-251", "begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 1) else (entier(sign(right)) + 25)); output(saved) end", "26", Value::Int(0)),
        ("old-252", "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 25); output(saved) end", "26", Value::Int(0)),
        ("old-253", "begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) + 17); output(saved) end", "26", Value::Int(0)),
        ("old-254", "begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (24 + sign(pick()))); output(saved) end", "26", Value::Int(0)),
        ("old-255", "begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 24); output(saved) end", "26", Value::Int(0)),
        ("old-256", "begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 23); output(saved) end", "26", Value::Int(0)),
        ("old-267", "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 25); output(saved) end", "26", Value::Int(0)),
        ("old-268", "begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) + 17); output(saved) end", "26", Value::Int(0)),
        ("old-269", "begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (24 + sign(pick()))); output(saved) end", "26", Value::Int(0)),
        ("old-270", "begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 24); output(saved) end", "26", Value::Int(0)),
        ("old-271", "begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 23); output(saved) end", "26", Value::Int(0)),
        ("old-282", "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 24 - 1); output(saved) end", "24", Value::Int(0)),
        ("old-283", "begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(entier(sign(pick()) + 24) + 1)); output(saved) end", "26", Value::Int(0)),
        ("old-284", "begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(abs(sign(pick()) + 24) + 1)); output(saved) end", "26", Value::Int(0)),
        ("old-288", "begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 25) else (entier(sign(right)) + 1)); output(saved) end", "24", Value::Int(0)),
        ("old-289", "begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 1) else (entier(sign(right)) + 25)); output(saved) end", "0", Value::Int(0)),
        ("old-290", "begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 25) else (entier(sign(right)) + 1)); output(saved) end", "2", Value::Int(0)),
        ("old-291", "begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 1) else (entier(sign(right)) + 25)); output(saved) end", "26", Value::Int(0)),
        ("old-292", "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 25); output(saved) end", "26", Value::Int(0)),
        ("old-293", "begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) + 17); output(saved) end", "26", Value::Int(0)),
        ("old-294", "begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (24 + sign(pick()))); output(saved) end", "26", Value::Int(0)),
        ("old-295", "begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 24); output(saved) end", "26", Value::Int(0)),
        ("old-296", "begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 23); output(saved) end", "26", Value::Int(0)),
        ("old-307", "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 25); output(saved) end", "26", Value::Int(0)),
        ("old-308", "begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) + 17); output(saved) end", "26", Value::Int(0)),
        ("old-309", "begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (24 + sign(pick()))); output(saved) end", "26", Value::Int(0)),
        ("old-310", "begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 24); output(saved) end", "26", Value::Int(0)),
        ("old-311", "begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 23); output(saved) end", "26", Value::Int(0)),
        ("old-323", "begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) + 17); output(saved) end", "26", Value::Int(0)),
        ("old-324", "begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (24 + sign(pick()))); output(saved) end", "26", Value::Int(0)),
        ("old-325", "begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 24); output(saved) end", "26", Value::Int(0)),
        ("old-326", "begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 23); output(saved) end", "26", Value::Int(0)),
        ("old-336", "begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, unit, saved; left := leftvalue(); right := rightvalue(); unit := 1.0; saved := entier(if choose() then (entier(sign(left)) + 25) else (entier(sign(right)) + 1)); output(saved) end", "24", Value::Int(0)),
        ("old-337", "begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, unit, saved; left := leftvalue(); right := rightvalue(); unit := 1.0; saved := entier(if choose() then (entier(sign(left)) + 1) else (entier(sign(right)) + 25)); output(saved) end", "0", Value::Int(0)),
        ("old-339", "begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, unit, saved; left := leftvalue(); right := rightvalue(); unit := 1.0; saved := entier(if choose() then (entier(sign(left)) + 25) else (entier(sign(right)) + 1)); output(saved) end", "2", Value::Int(0)),
        ("old-340", "begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, unit, saved; left := leftvalue(); right := rightvalue(); unit := 1.0; saved := entier(if choose() then (entier(sign(left)) + 1) else (entier(sign(right)) + 25)); output(saved) end", "26", Value::Int(0)),
        ("old-341", "begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) + 17); output(saved) end", "26", Value::Int(0)),
        ("old-342", "begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (24 + sign(pick()))); output(saved) end", "26", Value::Int(0)),
        ("old-343", "begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 24); output(saved) end", "26", Value::Int(0)),
        ("old-344", "begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 23); output(saved) end", "26", Value::Int(0)),
        ("old-346", "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 8 + 17); output(saved) end", "26", Value::Int(0)),
        ("old-347", "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 25); output(saved) end", "26", Value::Int(0)),
        ("main-additive-0", "begin real procedure pick; pick := -2.25; real result; result := entier(sign(pick()) + 25); output(result) end", "24", Value::Float(24.0)),
        ("main-additive-1", "begin real procedure pick; pick := -2.25; real result; result := entier(sign(pick()) + 24 + 1); output(result) end", "24", Value::Float(24.0)),
        ("main-additive-2", "begin real procedure pick; pick := -2.25; real result; result := entier((sign(pick()) + 24) + 1); output(result) end", "24", Value::Float(24.0)),
        ("main-additive-3", "begin real procedure pick; pick := -2.25; real result; result := entier(abs((sign(pick()) + 12) + 13)); output(result) end", "24", Value::Float(24.0)),
        ("canonical_twenty_four_units_stay_outside_the_dedicated_budget-0", "begin real procedure pick; pick := -2.25; real saved; saved := entier(sign(pick()) + 25); output(saved) end", "24", Value::Int(0)),
        ("canonical_twenty_four_units_stay_outside_the_dedicated_budget-1", "begin real procedure pick; pick := -2.25; real saved; saved := entier((sign(pick()) + 12) + 13); output(saved) end", "24", Value::Int(0)),
        ("canonical_twenty_four_units_stay_outside_the_dedicated_budget-2", "begin real procedure pick; pick := -2.25; real saved; saved := entier(entier(sign(pick()) + 1) + 24); output(saved) end", "24", Value::Int(0)),
        ("canonical_twenty_four_units_stay_outside_the_dedicated_budget-3", "begin real procedure pick; pick := -2.25; real saved; saved := entier(abs(sign(pick()) + 12) + 13); output(saved) end", "24", Value::Int(0)),
        ("runtime-compile25", "begin real procedure pick; begin output('P'); pick := -2.25 end; real saved; saved := entier(sign(pick()) + 25); output(saved) end", "P24", Value::Int(0)),
        ("integration24", "begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real procedure relay(x); real x; relay := x; real left, right, result; left := leftvalue(); right := rightvalue(); result := if choose() then relay(entier(entier(sign(left)) + 25)) else relay(entier(entier(sign(right)) + 25)); output(result) end", "24", Value::Float(24.0)),
    ] {
        let mut module = match compile_source(source, "native25_exact") {
            Ok(module) => module,
            Err(error) => { eprintln!("native25-exact REFUSED occurrence={id}"); refused.push(format!("{id}: {error:?}")); continue; }
        };
        let instructions = &module.get_function("main").expect("source main").instructions;
        assert_eq!(instructions.iter().filter(|i| i.op == "call" && i.srcs.first().and_then(Operand::as_var) == Some("__basic_print_real")).count(), 1, "{id}: {source}");
        assert_eq!(instructions.iter().filter(|i| i.op == "call_builtin" && i.srcs.first().and_then(Operand::as_var) == Some("print_i64")).count(), 0, "{id}: {source}");
        let output = Arc::new(Mutex::new(Vec::<u8>::new()));
        let mut vm = VMCore::new();
        let captured = Arc::clone(&output);
        vm.builtins_mut().register("putchar", move |args: &[Value]| {
            assert_eq!(args.len(), 1);
            let value = args[0].as_i64().expect("formatter byte is an integer");
            let byte = u8::try_from(value).expect("formatter byte range");
            let mut buffer = captured.lock().expect("formatter buffer");
            assert!(buffer.len() < 32, "bounded declared formatter output");
            buffer.push(byte);
            Ok(Value::Null)
        });
        let captured = Arc::clone(&output);
        vm.builtins_mut().register("print_i64", move |args: &[Value]| {
            assert_eq!(args.len(), 1);
            let text = args[0].as_i64().expect("formatter integer").to_string();
            let mut buffer = captured.lock().expect("formatter buffer");
            assert!(buffer.len() + text.len() <= 32, "bounded declared formatter output");
            buffer.extend_from_slice(text.as_bytes());
            Ok(Value::Null)
        });
        // Literal effects and numeric formatter calls share one ordered byte stream.
        // In particular, the exact runtime-compile25 source must retain its leading P.
        let captured = Arc::clone(&output);
        vm.builtins_mut().register("print_str", move |args: &[Value]| {
            assert_eq!(args.len(), 1);
            let text = args[0].as_str().expect("literal effect is a string");
            let mut buffer = captured.lock().expect("formatter buffer");
            assert!(buffer.len().checked_add(text.len()).is_some_and(|len| len <= 32), "bounded declared formatter output");
            buffer.extend_from_slice(text.as_bytes());
            Ok(Value::Null)
        });
        let actual = vm.execute(&mut module, "main", &[]).unwrap_or_else(|e| panic!("{id}: {source}: {e:?}"));
        match (actual, expected_return) {
            (Some(Value::Float(actual)), Value::Float(expected)) => assert_eq!(actual.to_bits(), expected.to_bits(), "{id}: exact f64 main"),
            (Some(Value::Int(actual)), Value::Int(expected)) => assert_eq!(actual, expected, "{id}: exact i64 main"),
            (actual, expected) => panic!("{id}: typed main {actual:?} expected {expected:?}"),
        }
        assert_eq!(*output.lock().expect("formatter buffer"), expected_stdout.as_bytes(), "{id}: complete formatter bytes: {source}");
        eprintln!("native25-exact COMPLETE occurrence={id}");
    }
    assert!(refused.is_empty(), "exact25 programs remain refused: {refused:?}");
}

#[test]
fn native25_intermediate_rounding_and_shared_exp_remain_refused() {
    for expression in [
        "entier(sign(pick()) + ((9007199254740992 + 1) - 9007199254740992 + 25))",
        "entier(sign(pick()) + ((9007199254740993 - 9007199254740992) + 25))",
        "entier(ln(exp(exp(sign(pick()) + 25))))",
        "entier((if true then (sign(pick()) + 25) else (sign(pick()) + 25)) + 1)",
    ] {
        let source = format!("begin real procedure pick; pick := 3.5; real saved; saved := {expression}; output(saved) end");
        assert!(matches!(compile_source(&source, "native25_guard"), Err(CompileError::Type(message)) if message.contains("cannot print a real value")), "{source}");
        eprintln!("native25-guard REFUSED expression={expression}");
    }
}

#[test]
fn native25_all_seven_raw_new_refusal_contexts_remain_complete() {
    for (id, source) in [
        (1, "begin boolean procedure choose; choose := true; real procedure pick; pick := 2.25; real procedure relay(x); real x; relay := x; real left, right, result; left := pick(); right := pick(); result := if choose() then relay(entier(entier(sign(left)) + 33)) else relay(entier(entier(sign(right)) + 33)); output(result) end"),
        (2, "begin boolean procedure choose; choose := true; real procedure pick; pick := 2.25; real procedure relay(x); real x; relay := x; real left, right, result; left := pick(); right := pick(); result := if choose() then relay(entier(entier(sign(left)) + 32 + 1)) else relay(entier(entier(sign(right)) + 1 + 32)); output(result) end"),
        (3, "begin boolean procedure choose; choose := true; real procedure pick; pick := 2.25; real procedure relay(x); real x; relay := x; real left, right, result; left := pick(); right := pick(); result := relay(entier((if choose() then (entier(sign(left)) + 25) else (entier(sign(right)) + 25)) + 1)); output(result) end"),
        (4, "begin real procedure pick; pick := -2.25; real result; result := entier(sign(pick()) + 33); output(result) end"),
        (5, "begin real procedure pick; pick := -2.25; real result; result := entier(sign(pick()) + 32 + 1); output(result) end"),
        (6, "begin real procedure pick; pick := -2.25; real result; result := entier((sign(pick()) + 32) + 1); output(result) end"),
        (7, "begin real procedure pick; pick := -2.25; real result; result := entier(abs((sign(pick()) + 13) + 20)); output(result) end"),
    ] {
        assert!(matches!(compile_source(source, "native25_raw_refusal"), Err(CompileError::Type(message)) if message.contains("cannot print a real value")), "raw{id}: {source}");
        eprintln!("native25-raw REFUSED context={id}");
    }
}
