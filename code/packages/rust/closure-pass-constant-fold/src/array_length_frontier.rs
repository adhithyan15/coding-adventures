//! Meter the temporary operand frontier before it reaches checked CV mutation.
//!
//! A graph's edge limit cannot protect allocations in its caller. The first
//! scan charges every slot and ID byte; storage arithmetic and initialization
//! charges precede fallible allocation. The second scan meters hash/probe/ID
//! comparisons, including collisions. Only the first-seen vector is emitted:
//! random table placement affects performance, never parent order.
use coding_adventures_correlation_vector::GraphLimits;
use std::collections::hash_map::RandomState;
use std::hash::BuildHasher;

struct Budget {
    remaining_work: usize,
    max_buffer_bytes: usize,
}

impl Budget {
    fn take(&mut self, amount: usize) -> Result<(), String> {
        self.remaining_work = self.remaining_work.checked_sub(amount)
            .ok_or("CV array frontier work limit exceeded")?;
        Ok(())
    }
}

type Buffers<'a> = (Vec<&'a str>, Vec<Option<&'a str>>);

fn buffer_layout(count: usize, cap: usize) -> Result<(usize, usize), String> {
    if count == 0 { return Ok((0, 0)); }
    let overflow = "CV array frontier buffer accounting overflow";
    let buckets = count.checked_mul(2).and_then(usize::checked_next_power_of_two).ok_or(overflow)?;
    let bytes = count.checked_mul(std::mem::size_of::<&str>())
        .and_then(|n| buckets.checked_mul(std::mem::size_of::<Option<&str>>()).and_then(|m| n.checked_add(m)))
        .ok_or(overflow)?;
    if bytes > cap { return Err("CV array frontier buffer limit exceeded".into()); }
    Ok((buckets, bytes))
}

fn reserve_buffers<'a>(count: usize, buckets: usize) -> Result<Buffers<'a>, String> {
    let mut unique = Vec::new();
    let mut table = Vec::new();
    unique.try_reserve_exact(count).map_err(|_| "CV array frontier allocation refused")?;
    table.try_reserve_exact(buckets).map_err(|_| "CV array frontier allocation refused")?;
    table.resize(buckets, None);
    Ok((unique, table))
}

fn collect_with<'a, I, H, A>(parents: I, budget: &mut Budget, mut hash: H, allocate: A) -> Result<Vec<&'a str>, String>
where
    I: Iterator<Item = Option<&'a str>> + Clone,
    H: FnMut(&str) -> u64,
    A: FnOnce(usize, usize) -> Result<Buffers<'a>, String>,
{
    let mut count = 0usize;
    let mut first = parents.clone();
    loop {
        // Charge before next() inspects a source cell, including both scans'
        // final iterator advance. Missing/opaque/duplicate/hole cells count.
        budget.take(2)?;
        let Some(parent) = first.next() else { break; };
        if let Some(id) = parent {
            budget.take(id.len())?;
            count = count.checked_add(1).ok_or("CV array frontier count overflow")?;
        }
    }
    if count == 0 { return Ok(Vec::new()); }
    let (buckets, _) = buffer_layout(count, budget.max_buffer_bytes)?;
    budget.take(buckets)?; // Precharge table resize before either allocation.
    let (mut unique, mut table) = allocate(count, buckets)?;
    let mask = buckets - 1;
    for parent in parents {
        // Slot inspection was charged by the first, identical immutable scan.
        let Some(id) = parent else { continue; };
        // Hash setup, str terminator and finish have constant cost; hash input
        // bytes are charged separately. No client-provided hash is used by the
        // public pass; injection below is private to deterministic unit tests.
        budget.take(4)?;
        budget.take(id.len())?;
        let mut slot = (hash(id) as usize) & mask;
        let mut found = false;
        for _ in 0..buckets {
            budget.take(1)?;
            if let Some(previous) = table[slot] {
                budget.take(id.len())?;
                budget.take(previous.len())?;
                if previous == id { found = true; break; }
            } else {
                table[slot] = Some(id);
                unique.push(id);
                found = true;
                break;
            }
            // The table is a power of two with load <= 1/2. Explicit wrapping
            // and a finite probe loop also make corrupted/full-table refusal
            // bounded; neither overflow nor a missing empty slot can loop.
            slot = slot.wrapping_add(1) & mask;
        }
        if !found { return Err("CV array frontier probe capacity exhausted".into()); }
    }
    Ok(unique)
}

pub(super) fn collect<'a>(parents: impl Iterator<Item = Option<&'a str>> + Clone) -> Result<Vec<&'a str>, String> {
    // These are per-collector work/requested-storage caps. They do not claim
    // to bound the entire recursive pass or its already allocated input AST.
    let limits = GraphLimits::default();
    let mut budget = Budget { remaining_work: limits.max_work, max_buffer_bytes: limits.max_input_bytes };
    let state = RandomState::new();
    collect_with(parents, &mut budget, |id| state.hash_one(id), reserve_buffers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forced_collisions_preserve_first_occurrence_and_charge_every_step() {
        for parents in [
            vec![None, None],
            vec![Some("a"), Some("a"), None, Some("a")],
            vec![Some("z"), None, Some("a"), Some("z"), Some("b")],
        ] {
            let mut budget = Budget { remaining_work: usize::MAX, max_buffer_bytes: usize::MAX };
            let expected = collect_with(parents.iter().copied(), &mut budget, |_| 0, reserve_buffers).unwrap();
            assert_eq!(expected, if parents[0].is_none() { vec![] }
                else if parents[0] == Some("a") { vec!["a"] }
                else { vec!["z", "a", "b"] });
            let used = usize::MAX - budget.remaining_work;
            // Every smaller budget refuses, covering preflight visits/bytes,
            // initialization, hashing, collided probes and equality charges.
            for limit in 0..=used {
                let mut budget = Budget { remaining_work: limit, max_buffer_bytes: usize::MAX };
                let result = collect_with(parents.iter().copied(), &mut budget, |_| 0, reserve_buffers);
                if limit == used { assert_eq!(result.unwrap(), expected); }
                else { assert!(result.unwrap_err().contains("work limit")); }
            }
        }
    }

    #[test]
    fn storage_is_checked_before_allocation_and_both_buffers_count() {
        let (_, bytes) = buffer_layout(3, usize::MAX).unwrap();
        assert_eq!(bytes, 3 * std::mem::size_of::<&str>() + 8 * std::mem::size_of::<Option<&str>>());
        for cap in [bytes - 1, bytes] {
            let mut budget = Budget { remaining_work: 1000, max_buffer_bytes: cap };
            let result = collect_with([Some("a"), Some("a"), Some("b")].into_iter(), &mut budget, |_| 0, reserve_buffers);
            if cap == bytes { assert_eq!(result.unwrap(), ["a", "b"]); }
            else { assert!(result.unwrap_err().contains("buffer limit")); }
        }
        assert!(buffer_layout(usize::MAX, usize::MAX).unwrap_err().contains("overflow"));
        // Multiplication can fit while rounding or byte accounting overflows.
        assert!(buffer_layout(usize::MAX / 2, usize::MAX).is_err());
        assert!(buffer_layout(usize::MAX / 16, usize::MAX).is_err());
    }

    #[test]
    fn missing_identities_do_not_allocate_or_hash_and_allocation_failure_is_an_error() {
        let mut budget = Budget { remaining_work: 6, max_buffer_bytes: 0 };
        let out = collect_with([None, None].into_iter(), &mut budget,
            |_| panic!("no available IDs"), |_, _| panic!("no buffer needed")).unwrap();
        assert!(out.is_empty());
        let mut budget = Budget { remaining_work: 100, max_buffer_bytes: 1000 };
        let error = collect_with([Some("a")].into_iter(), &mut budget,
            |_| panic!("allocation precedes hashing"), |_, _| Err("CV array frontier allocation refused (injected)".into())).unwrap_err();
        assert!(error.contains("allocation refused"));
        let mut budget = Budget { remaining_work: 0, max_buffer_bytes: 1000 };
        assert!(collect_with([Some("a")].into_iter(), &mut budget,
            |_| panic!("preflight precedes hashing"), |_, _| panic!("preflight precedes allocation")).is_err());
    }

    #[test]
    fn long_ids_are_metered_and_random_placement_never_changes_order() {
        let long = "x".repeat(4096);
        let parents = [Some(long.as_str()), Some("b"), Some(long.as_str())];
        let mut budget = Budget { remaining_work: usize::MAX, max_buffer_bytes: usize::MAX };
        let expected = collect_with(parents.into_iter(), &mut budget, |_| 0, reserve_buffers).unwrap();
        let used = usize::MAX - budget.remaining_work;
        let mut short = Budget { remaining_work: used - 1, max_buffer_bytes: usize::MAX };
        assert!(collect_with(parents.into_iter(), &mut short, |_| 0, reserve_buffers).is_err());
        for _ in 0..16 { assert_eq!(collect(parents.into_iter()).unwrap(), expected); }
    }
}
