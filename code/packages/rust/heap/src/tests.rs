use crate::*;

mod legacy_tests {
    use super::*;

    fn is_valid_min_heap<T: Ord>(values: &[T]) -> bool {
        for index in 0..values.len() {
            let left = 2 * index + 1;
            let right = 2 * index + 2;
            if left < values.len() && values[index] > values[left] {
                return false;
            }
            if right < values.len() && values[index] > values[right] {
                return false;
            }
        }
        true
    }

    fn is_valid_max_heap<T: Ord>(values: &[T]) -> bool {
        for index in 0..values.len() {
            let left = 2 * index + 1;
            let right = 2 * index + 2;
            if left < values.len() && values[index] < values[left] {
                return false;
            }
            if right < values.len() && values[index] < values[right] {
                return false;
            }
        }
        true
    }

    #[test]
    fn min_heap_push_peek_pop_order() {
        let mut heap = MinHeap::new();
        for value in [5, 3, 8, 1, 4] {
            heap.push(value);
        }
        assert_eq!(heap.peek(), Some(&1));
        assert_eq!(heap.pop(), Some(1));
        assert_eq!(heap.pop(), Some(3));
        assert_eq!(heap.pop(), Some(4));
        assert_eq!(heap.pop(), Some(5));
        assert_eq!(heap.pop(), Some(8));
        assert_eq!(heap.pop(), None);
    }

    #[test]
    fn min_heap_property_holds_after_each_push() {
        let mut heap = MinHeap::new();
        for value in [5, 3, 8, 1, 4, 2, 7] {
            heap.push(value);
            assert!(is_valid_min_heap(&heap.to_vec()));
        }
    }

    #[test]
    fn min_heap_property_holds_after_each_pop() {
        let mut heap = MinHeap::from_iterable([5, 3, 8, 1, 4, 2, 7]);
        while heap.pop().is_some() {
            assert!(is_valid_min_heap(&heap.to_vec()));
        }
    }

    #[test]
    fn min_heap_supports_generic_ord_values() {
        let mut heap = MinHeap::from_iterable([
            String::from("delta"),
            String::from("alpha"),
            String::from("charlie"),
        ]);
        assert_eq!(heap.pop(), Some(String::from("alpha")));
    }

    #[test]
    fn max_heap_push_peek_pop_order() {
        let mut heap = MaxHeap::new();
        for value in [5, 3, 8, 1, 4] {
            heap.push(value);
        }
        assert_eq!(heap.peek(), Some(&8));
        assert_eq!(heap.pop(), Some(8));
        assert_eq!(heap.pop(), Some(5));
        assert_eq!(heap.pop(), Some(4));
        assert_eq!(heap.pop(), Some(3));
        assert_eq!(heap.pop(), Some(1));
        assert_eq!(heap.pop(), None);
    }

    #[test]
    fn max_heap_property_holds_after_each_pop() {
        let mut heap = MaxHeap::from_iterable([5, 3, 8, 1, 4]);
        while heap.pop().is_some() {
            assert!(is_valid_max_heap(&heap.to_vec()));
        }
    }

    #[test]
    fn heapify_preserves_elements_and_invariant() {
        let values = vec![3, 1, 4, 1, 5, 9, 2, 6];
        let heap = heapify(values.clone());
        let mut sorted_original = values;
        let mut sorted_heap = heap.clone();
        sorted_original.sort();
        sorted_heap.sort();
        assert_eq!(sorted_original, sorted_heap);
        assert!(is_valid_min_heap(&heap));
    }

    #[test]
    fn heap_sort_sorts_ascending() {
        assert_eq!(
            heap_sort([3, 1, 4, 1, 5, 9, 2, 6]),
            vec![1, 1, 2, 3, 4, 5, 6, 9]
        );
    }

    #[test]
    fn heap_sort_handles_empty_and_single_element_inputs() {
        assert_eq!(heap_sort(Vec::<i32>::new()), Vec::<i32>::new());
        assert_eq!(heap_sort([42]), vec![42]);
    }

    #[test]
    fn nlargest_returns_descending_values() {
        assert_eq!(nlargest([3, 1, 4, 1, 5, 9, 2, 6], 3), vec![9, 6, 5]);
    }

    #[test]
    fn nsmallest_returns_ascending_values() {
        assert_eq!(nsmallest([3, 1, 4, 1, 5, 9, 2, 6], 3), vec![1, 1, 2]);
    }

    #[test]
    fn top_k_handles_zero_and_large_n() {
        assert_eq!(nlargest([3, 1, 4], 0), Vec::<i32>::new());
        assert_eq!(nsmallest([3, 1, 4], 10), vec![1, 3, 4]);
    }

    #[test]
    fn display_mentions_heap_type_and_root() {
        let heap = MinHeap::from_iterable([7, 3, 9]);
        let rendered = heap.to_string();
        assert!(rendered.contains("MinHeap"));
        assert!(rendered.contains("3"));
    }

    #[test]
    fn defaults_empty_checks_and_both_display_branches_work() {
        let min = MinHeap::<i32>::default();
        assert!(min.is_empty());
        assert_eq!(min.to_string(), "MinHeap(size=0, root=empty)");
        let max = MaxHeap::<i32>::default();
        assert!(max.is_empty());
        assert_eq!(max.to_string(), "MaxHeap(size=0, root=empty)");
        assert_eq!(
            MaxHeap::from_iterable([4, 8]).to_string(),
            "MaxHeap(size=2, root=8)"
        );
    }
}

mod bounded_tests {
    use super::{
        left_child_index, right_child_index, BoundedMaxHeap, BoundedMinHeap, CheckedPriorityQueue,
        TryBuildError, TryPushError,
    };
    use std::cell::Cell;
    use std::cmp::Ordering;
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::rc::Rc;
    use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
    use std::sync::Arc;

    #[derive(Clone, Debug)]
    enum Operation {
        Push(i32),
        Pop,
        Peek,
        Clear,
    }

    fn heap_property(values: &[i32], min_order: bool) -> bool {
        for parent in 0..values.len() {
            let Some(left) = left_child_index(parent) else {
                continue;
            };
            for child in [Some(left), right_child_index(left)].into_iter().flatten() {
                if child >= values.len() {
                    continue;
                }
                if min_order && values[parent] > values[child] {
                    return false;
                }
                if !min_order && values[parent] < values[child] {
                    return false;
                }
            }
        }
        true
    }

    fn snapshot<Q: CheckedPriorityQueue<i32> + Clone>(queue: &Q) -> Vec<i32> {
        let mut copy = queue.clone();
        let mut ordered = Vec::with_capacity(copy.len());
        while let Some(value) = copy.pop() {
            ordered.push(value);
        }
        ordered
    }

    fn validate<Q: CheckedPriorityQueue<i32> + Clone>(
        queue: &Q,
        model: &[i32],
        capacity: usize,
        min_order: bool,
        raw_heap: &[i32],
    ) -> Result<(), String> {
        if queue.capacity() != capacity {
            return Err(format!("capacity {} != {capacity}", queue.capacity()));
        }
        if queue.len() != model.len() || queue.len() > capacity {
            return Err(format!(
                "length {} differs from model {} or capacity {capacity}",
                queue.len(),
                model.len()
            ));
        }
        if queue.is_empty() != model.is_empty() {
            return Err("is_empty disagrees with the model".into());
        }
        if !heap_property(raw_heap, min_order) {
            return Err(format!("heap order is invalid: {raw_heap:?}"));
        }

        let mut expected = model.to_vec();
        expected.sort_unstable();
        if !min_order {
            expected.reverse();
        }
        let observed = snapshot(queue);
        if observed != expected {
            return Err(format!(
                "drained values {observed:?} differ from model {expected:?}"
            ));
        }
        if queue.peek().copied() != expected.first().copied() {
            return Err(format!(
                "peek {:?} differs from model root {:?}",
                queue.peek(),
                expected.first()
            ));
        }
        Ok(())
    }

    fn run_history<Q, F, S>(
        capacity: usize,
        min_order: bool,
        operations: &[Operation],
        make_queue: F,
        raw_heap: S,
    ) -> Result<(), String>
    where
        Q: CheckedPriorityQueue<i32> + Clone,
        F: FnOnce(usize) -> Q,
        S: Fn(&Q) -> Vec<i32>,
    {
        let mut queue = make_queue(capacity);
        let mut model = Vec::new();
        validate(&queue, &model, capacity, min_order, &raw_heap(&queue))?;

        for (step, operation) in operations.iter().enumerate() {
            match *operation {
                Operation::Push(value) => {
                    if model.len() == capacity {
                        match queue.try_push(value) {
                            Err(TryPushError::Full { value: rejected }) if rejected == value => {}
                            Err(TryPushError::Allocation { .. }) => {
                                return Err("unexpected allocation error while model queue is below usize limits".into())
                            }
                            other => return Err(format!("full push result {other:?} at step {step}")),
                        }
                    } else {
                        queue.try_push(value).map_err(|error| {
                            format!("push unexpectedly failed at step {step}: {error}")
                        })?;
                        model.push(value);
                    }
                }
                Operation::Pop => {
                    model.sort_unstable();
                    let expected = if min_order {
                        if model.is_empty() {
                            None
                        } else {
                            Some(model.remove(0))
                        }
                    } else if model.is_empty() {
                        None
                    } else {
                        Some(model.pop().expect("nonempty model"))
                    };
                    if queue.pop() != expected {
                        return Err(format!("pop differs from model at step {step}"));
                    }
                }
                Operation::Peek => {
                    model.sort_unstable();
                    let expected = if min_order {
                        model.first()
                    } else {
                        model.last()
                    };
                    if queue.peek() != expected {
                        return Err(format!("peek differs from model at step {step}"));
                    }
                }
                Operation::Clear => {
                    queue.clear();
                    model.clear();
                }
            }
            validate(&queue, &model, capacity, min_order, &raw_heap(&queue))
                .map_err(|message| format!("step {step}: {message}"))?;
        }
        Ok(())
    }

    fn visit_histories(
        prefix: &mut Vec<Operation>,
        remaining: usize,
        alphabet: &[Operation],
        visit: &mut impl FnMut(&[Operation]),
    ) {
        visit(prefix);
        if remaining == 0 {
            return;
        }
        for operation in alphabet {
            prefix.push(operation.clone());
            visit_histories(prefix, remaining - 1, alphabet, visit);
            prefix.pop();
        }
    }

    fn run_min(capacity: usize, operations: &[Operation]) -> Result<(), String> {
        run_history(
            capacity,
            true,
            operations,
            BoundedMinHeap::new,
            |queue: &BoundedMinHeap<i32>| queue.to_vec(),
        )
    }

    fn run_max(capacity: usize, operations: &[Operation]) -> Result<(), String> {
        run_history(
            capacity,
            false,
            operations,
            BoundedMaxHeap::new,
            |queue: &BoundedMaxHeap<i32>| queue.to_vec(),
        )
    }

    #[test]
    fn exhaustive_histories_cover_every_short_transition() {
        let alphabet = [
            Operation::Push(-1),
            Operation::Push(0),
            Operation::Push(1),
            Operation::Pop,
            Operation::Peek,
            Operation::Clear,
        ];
        let mut total_histories = 0usize;
        let mut total_operations = 0usize;
        for capacity in [0, 1, 3] {
            for min_order in [true, false] {
                let mut count = 0usize;
                let mut visit = |history: &[Operation]| {
                    let result = if min_order {
                        run_min(capacity, history)
                    } else {
                        run_max(capacity, history)
                    };
                    assert!(
                        result.is_ok(),
                        "capacity={capacity}, min={min_order}, history={history:?}: {result:?}"
                    );
                    count += 1;
                    total_operations += history.len();
                };
                visit_histories(&mut Vec::new(), 5, &alphabet, &mut visit);
                assert_eq!(count, 9_331, "capacity={capacity}, min={min_order}");
                total_histories += count;
            }
        }
        assert_eq!(total_histories, 55_986);
        assert_eq!(total_operations, 268_740);
    }

    struct SplitMix64(u64);

    impl SplitMix64 {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut value = self.0;
            value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            value ^ (value >> 31)
        }
    }

    fn generated_operation(rng: &mut SplitMix64) -> Operation {
        match rng.next() % 6 {
            0 => Operation::Push(i32::MIN),
            1 => Operation::Push(i32::MAX),
            2 => Operation::Push((rng.next() as i32) % 101),
            3 => Operation::Pop,
            4 => Operation::Peek,
            _ => Operation::Clear,
        }
    }

    #[test]
    fn seeded_mixed_histories_are_replayable() {
        let mut histories = 0usize;
        let mut operations = 0usize;
        for (min_order, seed) in [(true, 0xD04C), (false, 0xD04D)] {
            let mut rng = SplitMix64(seed);
            for history_index in 0..200 {
                let capacity = (rng.next() % 9) as usize;
                let length = (rng.next() % 101) as usize;
                let history: Vec<_> = (0..length).map(|_| generated_operation(&mut rng)).collect();
                histories += 1;
                operations += history.len();
                let result = if min_order {
                    run_min(capacity, &history)
                } else {
                    run_max(capacity, &history)
                };
                assert!(
                    result.is_ok(),
                    "seed={seed:#x}, min={min_order}, history_index={history_index}, capacity={capacity}, operations={history:?}: {result:?}"
                );
            }
        }
        assert_eq!(histories, 400);
        assert_eq!(operations, 20_312);
    }

    fn operations_from_bytes(bytes: &[u8]) -> Vec<Operation> {
        bytes
            .iter()
            .map(|byte| match byte % 6 {
                0 => Operation::Push(i32::from(*byte) - 128),
                1 => Operation::Push(i32::MIN),
                2 => Operation::Push(i32::MAX),
                3 => Operation::Pop,
                4 => Operation::Peek,
                _ => Operation::Clear,
            })
            .collect()
    }

    fn bytes_hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    #[test]
    fn deterministic_byte_histories_are_replayable() {
        let seed = 0xD5_04C;
        let mut rng = SplitMix64(seed);
        let mut history_executions = 0usize;
        let mut operations = 0usize;
        for case_index in 0..256 {
            let bytes: Vec<u8> = (0..64).map(|_| rng.next() as u8).collect();
            let history = operations_from_bytes(&bytes);
            let capacity = usize::from(bytes[0] % 9);
            for min_order in [true, false] {
                history_executions += 1;
                operations += history.len();
                let result = if min_order {
                    run_min(capacity, &history)
                } else {
                    run_max(capacity, &history)
                };
                assert!(
                    result.is_ok(),
                    "seed={seed:#x}, case={case_index}, min={min_order}, bytes={}, capacity={capacity}: {result:?}",
                    bytes_hex(&bytes)
                );
            }
        }
        assert_eq!(history_executions, 512);
        assert_eq!(operations, 32_768);
    }

    #[test]
    fn model_negative_controls_detect_wrong_pop_and_over_capacity() {
        // These are deliberately faulty observations in the test adapter, not
        // mutations to the production heap.
        fn observations_match(expected: &[i32], observed: &[i32], cap: usize) -> bool {
            observed.len() <= cap && expected == observed
        }
        assert!(
            !observations_match(&[1, 3], &[2, 3], 2),
            "oracle must catch a wrong root removal"
        );
        assert!(
            !observations_match(&[1], &[1, 2], 1),
            "oracle must catch admission past capacity"
        );
        assert!(observations_match(&[1, 3], &[1, 3], 2));
    }

    #[test]
    fn errors_preserve_owned_values_and_constructor_prefix() {
        let mut queue = BoundedMinHeap::try_from_iterable([String::from("kept")], 1).unwrap();
        let error = queue.try_push(String::from("rejected")).unwrap_err();
        assert_eq!(error.into_value(), "rejected");
        assert_eq!(queue.peek().map(String::as_str), Some("kept"));

        let build = BoundedMaxHeap::try_from_iterable([10, 20, 30], 2).unwrap_err();
        match build {
            TryBuildError::CapacityExceeded { accepted, rejected } => {
                assert_eq!(accepted, vec![10, 20]);
                assert_eq!(rejected, 30);
            }
            other => panic!("unexpected build error: {other:?}"),
        }
        assert!(BoundedMinHeap::<i32>::try_from_iterable([], 0)
            .unwrap()
            .is_empty());
        assert!(BoundedMaxHeap::<i32>::try_from_iterable([1, 2, 3], 3).is_ok());

        let full = TryPushError::Full { value: 17 };
        assert!(full.to_string().contains("capacity"));
        assert!(std::error::Error::source(&full).is_none());
        let capacity_error = TryBuildError::CapacityExceeded {
            accepted: vec![1],
            rejected: 2,
        };
        assert!(capacity_error.to_string().contains("capacity"));
        assert!(std::error::Error::source(&capacity_error).is_none());

        let source = Vec::<u8>::new().try_reserve(usize::MAX).unwrap_err();
        let allocation_push = TryPushError::Allocation { value: 17, source };
        assert!(allocation_push.to_string().contains("allocation"));
        assert!(std::error::Error::source(&allocation_push).is_some());
        let source = Vec::<u8>::new().try_reserve(usize::MAX).unwrap_err();
        let allocation_build = TryBuildError::Allocation {
            accepted: vec![1],
            rejected: 2,
            source,
        };
        assert!(allocation_build.to_string().contains("allocation"));
        assert!(std::error::Error::source(&allocation_build).is_some());
    }

    #[test]
    fn deterministic_capacity_overflow_seam_preserves_error_ownership() {
        let mut queue = BoundedMinHeap::try_from_iterable([4, 9], 3).unwrap();
        let before = queue.to_vec();
        let result = queue
            .core
            .try_push_with_reservation(7, |items, _| items.try_reserve(usize::MAX));
        match result {
            Err(TryPushError::Allocation { value, source }) => {
                assert_eq!(value, 7);
                assert!(!source.to_string().is_empty());
            }
            other => panic!("capacity-overflow reserve seam returned {other:?}"),
        }
        assert_eq!(
            queue.to_vec(),
            before,
            "failed reserve leaves queued contents unchanged"
        );

        let mut reservations = 0;
        let build = super::BoundedCore::try_from_iterable_with_reservation(
            3,
            true,
            [11, 12],
            |items, _| {
                reservations += 1;
                if reservations == 1 {
                    items.try_reserve(1)
                } else {
                    items.try_reserve(usize::MAX)
                }
            },
        );
        match build {
            Err(TryBuildError::Allocation {
                accepted,
                rejected,
                source,
            }) => {
                assert_eq!(accepted, vec![11]);
                assert_eq!(rejected, 12);
                assert!(!source.to_string().is_empty());
            }
            other => panic!("capacity-overflow constructor seam returned {other:?}"),
        }
    }

    #[test]
    fn constructor_reads_once_and_only_one_value_past_capacity() {
        struct Counted {
            next: i32,
            calls: Rc<Cell<usize>>,
        }
        impl Iterator for Counted {
            type Item = i32;
            fn next(&mut self) -> Option<Self::Item> {
                self.calls.set(self.calls.get() + 1);
                if self.next >= 4 {
                    return None;
                }
                let value = self.next;
                self.next += 1;
                Some(value)
            }
        }

        let calls = Rc::new(Cell::new(0));
        let iterator = Counted {
            next: 0,
            calls: Rc::clone(&calls),
        };
        let result = BoundedMinHeap::try_from_iterable(iterator, 2);
        assert!(matches!(
            result,
            Err(TryBuildError::CapacityExceeded { .. })
        ));
        assert_eq!(
            calls.get(),
            3,
            "only two admitted values and the first excess are observed"
        );

        let exact_calls = Rc::new(Cell::new(0));
        let exact = Counted {
            next: 0,
            calls: Rc::clone(&exact_calls),
        };
        assert!(BoundedMaxHeap::try_from_iterable(exact, 4).is_ok());
        assert_eq!(
            exact_calls.get(),
            5,
            "one terminal next observes iterator exhaustion"
        );
    }

    #[test]
    fn logical_capacity_is_independent_of_vec_capacity_and_handles_zst() {
        let mut queue = BoundedMinHeap::<()>::new(2);
        assert_eq!(queue.capacity(), 2);
        queue.try_push(()).unwrap();
        queue.try_push(()).unwrap();
        assert!(matches!(
            queue.try_push(()),
            Err(TryPushError::Full { value: () })
        ));
        queue.clear();
        assert_eq!(queue.capacity(), 2);
        assert!(queue.is_empty());

        let enormous_logical_limit = BoundedMaxHeap::<u8>::new(usize::MAX);
        assert_eq!(enormous_logical_limit.capacity(), usize::MAX);
        assert!(enormous_logical_limit.to_vec().is_empty());
        assert_eq!(left_child_index(usize::MAX), None);
        assert_eq!(right_child_index(usize::MAX), None);
    }

    #[test]
    fn trait_interface_dispatches_the_complete_queue_contract() {
        fn exercise<Q: CheckedPriorityQueue<i32>>(queue: &mut Q) {
            assert!(queue.is_empty());
            assert_eq!(queue.peek(), None);
            queue.try_push(4).unwrap();
            queue.try_push(2).unwrap();
            assert_eq!(queue.len(), 2);
            assert!(!queue.is_empty());
            assert_eq!(queue.peek(), Some(&2));
            assert_eq!(queue.pop(), Some(2));
            queue.clear();
            assert!(queue.is_empty());
        }
        exercise(&mut BoundedMinHeap::new(2));

        let mut max = BoundedMaxHeap::new(1);
        let queue: &mut dyn CheckedPriorityQueue<i32> = &mut max;
        queue.try_push(8).unwrap();
        assert_eq!(queue.peek(), Some(&8));
        assert_eq!(queue.pop(), Some(8));
    }

    #[test]
    fn existing_unbounded_heaps_and_helpers_match_sorted_oracles() {
        let mut rng = SplitMix64(0xD04E);
        for _case in 0..128 {
            let length = (rng.next() % 40) as usize;
            let mut values: Vec<i32> = (0..length)
                .map(|_| match rng.next() % 8 {
                    0 => i32::MIN,
                    1 => i32::MAX,
                    _ => rng.next() as i32,
                })
                .collect();
            let mut ascending = values.clone();
            ascending.sort_unstable();

            let mut min = super::MinHeap::from_iterable(values.clone());
            let mut min_output = Vec::new();
            while let Some(value) = min.pop() {
                min_output.push(value);
            }
            assert_eq!(min_output, ascending);

            let mut max = super::MaxHeap::from_iterable(values.clone());
            let mut max_output = Vec::new();
            while let Some(value) = max.pop() {
                max_output.push(value);
            }
            ascending.reverse();
            assert_eq!(max_output, ascending);

            assert_eq!(super::heap_sort(values.clone()), {
                let mut expected = values.clone();
                expected.sort_unstable();
                expected
            });
            let heapified = super::heapify(values.clone());
            let mut heapified_values = heapified.clone();
            heapified_values.sort_unstable();
            let mut expected = values.clone();
            expected.sort_unstable();
            assert_eq!(heapified_values, expected);
            assert!(heap_property(&heapified, true));
            for n in [0, 1, length / 2, length, length.saturating_add(1)] {
                let mut expected = values.clone();
                expected.sort_unstable();
                let smallest: Vec<_> = expected.iter().take(n).copied().collect();
                expected.sort_unstable_by(|left, right| right.cmp(left));
                let largest: Vec<_> = expected.iter().take(n).copied().collect();
                assert_eq!(super::nsmallest(values.clone(), n), smallest);
                assert_eq!(super::nlargest(values.clone(), n), largest);
            }
            values.clear();
        }
    }

    #[derive(Clone, Debug)]
    struct PanicOrd {
        value: i32,
        panic_next: Arc<AtomicBool>,
    }

    impl PartialEq for PanicOrd {
        fn eq(&self, other: &Self) -> bool {
            self.value == other.value
        }
    }
    impl Eq for PanicOrd {}
    impl PartialOrd for PanicOrd {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }
    impl Ord for PanicOrd {
        fn cmp(&self, other: &Self) -> Ordering {
            if self.panic_next.swap(false, AtomicOrdering::SeqCst) {
                panic!("controlled Ord panic");
            }
            self.value.cmp(&other.value)
        }
    }

    fn panic_value(value: i32, panic_next: &Arc<AtomicBool>) -> PanicOrd {
        PanicOrd {
            value,
            panic_next: Arc::clone(panic_next),
        }
    }

    #[test]
    fn comparison_panics_do_not_promise_rollback() {
        let panic_next = Arc::new(AtomicBool::new(false));
        let mut push_queue = BoundedMinHeap::try_from_iterable(
            [panic_value(10, &panic_next), panic_value(20, &panic_next)],
            4,
        )
        .unwrap();
        panic_next.store(true, AtomicOrdering::SeqCst);
        assert!(catch_unwind(AssertUnwindSafe(|| {
            let _ = push_queue.try_push(panic_value(1, &panic_next));
        }))
        .is_err());
        assert_eq!(
            push_queue.len(),
            3,
            "the push occurred before the comparison panic"
        );
        drop(push_queue);

        panic_next.store(false, AtomicOrdering::SeqCst);
        let mut pop_queue = BoundedMinHeap::try_from_iterable(
            [
                panic_value(1, &panic_next),
                panic_value(2, &panic_next),
                panic_value(3, &panic_next),
            ],
            4,
        )
        .unwrap();
        panic_next.store(true, AtomicOrdering::SeqCst);
        assert!(catch_unwind(AssertUnwindSafe(|| pop_queue.pop())).is_err());
        assert_eq!(
            pop_queue.len(),
            2,
            "the root was removed before sift-down comparison"
        );
        drop(pop_queue);

        panic_next.store(false, AtomicOrdering::SeqCst);
        panic_next.store(true, AtomicOrdering::SeqCst);
        let construction = catch_unwind(AssertUnwindSafe(|| {
            BoundedMaxHeap::try_from_iterable(
                [
                    panic_value(3, &panic_next),
                    panic_value(1, &panic_next),
                    panic_value(2, &panic_next),
                ],
                3,
            )
        }));
        assert!(construction.is_err());
    }
}
