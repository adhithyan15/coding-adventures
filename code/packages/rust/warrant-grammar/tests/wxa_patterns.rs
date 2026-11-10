//! VER04j section 4.11, "Exhaustiveness differential": the pattern-matrix
//! checker (`patterns.rs`) against brute-force enumeration of every value of
//! small finite types, for every generated pattern set up to four arms.
//!
//! The oracle never looks at the algorithm.  A pattern is turned into the set
//! of values it matches (a bitmask over the enumerated values of the type);
//! then
//!
//! ```text
//! the match is exhaustive   <=>  the union of the arms' sets is every value
//! arm i is redundant        <=>  set(i) is contained in the union of sets 0..i
//! ```

use coding_adventures_warrant_grammar::patterns::{
    analyze, matches, useful, values, Exhausted, MatchReport, PTy, Pat,
};

/// Every well-formed pattern of `ty` (a wildcard, or a constructor over every
/// combination of sub-patterns of its fields).
fn pats(ty: &PTy) -> Vec<Pat> {
    let mut out = vec![Pat::Wild];
    if let Some(ctors) = ty.ctors() {
        for (i, fields) in ctors.iter().enumerate() {
            let mut combos: Vec<Vec<Pat>> = vec![Vec::new()];
            for f in fields {
                let options = pats(f);
                let mut next = Vec::new();
                for c in &combos {
                    for o in &options {
                        let mut n = c.clone();
                        n.push(o.clone());
                        next.push(n);
                    }
                }
                combos = next;
            }
            out.extend(combos.into_iter().map(|c| Pat::Ctor(i, c)));
        }
    }
    out
}

/// The brute-force verdict for a sequence of arm masks over `n` values.
fn oracle(masks: &[u64], n: usize) -> MatchReport {
    let all = if n == 64 { u64::MAX } else { (1u64 << n) - 1 };
    let mut seen = 0u64;
    let mut redundant = Vec::new();
    for (i, m) in masks.iter().enumerate() {
        if m & !seen == 0 {
            redundant.push(i);
        }
        seen |= m;
    }
    MatchReport {
        exhaustive: seen == all,
        redundant,
    }
}

/// Compare the checker with the oracle on every arm sequence of length
/// `1..=max_len` (exhaustively) and on `sample` sequences of length `max_len + 1`
/// drawn by a fixed LCG.  Returns how many sequences were compared.
fn differential(ty: &PTy, max_len: usize, sample: usize) -> usize {
    let patterns = pats(ty);
    let vals = values(ty).expect("a finite type");
    assert!(vals.len() <= 63, "{} values", vals.len());
    let masks: Vec<u64> = patterns
        .iter()
        .map(|p| {
            vals.iter()
                .enumerate()
                .fold(0u64, |m, (i, v)| if matches(p, v) { m | (1 << i) } else { m })
        })
        .collect();
    let mut compared = 0;
    let check = |idx: &[usize]| {
        let arms: Vec<Pat> = idx.iter().map(|i| patterns[*i].clone()).collect();
        let ms: Vec<u64> = idx.iter().map(|i| masks[*i]).collect();
        let mut budget = usize::MAX;
        let got = analyze(&arms, ty, &mut budget).expect("budget");
        assert_eq!(got, oracle(&ms, vals.len()), "arms {arms:?}");
    };
    // Every sequence up to max_len, by odometer.
    for len in 1..=max_len {
        let mut idx = vec![0usize; len];
        loop {
            check(&idx);
            compared += 1;
            let mut pos = len;
            loop {
                if pos == 0 {
                    break;
                }
                pos -= 1;
                idx[pos] += 1;
                if idx[pos] < patterns.len() {
                    break;
                }
                idx[pos] = 0;
                if pos == 0 {
                    pos = usize::MAX;
                    break;
                }
            }
            if pos == usize::MAX || (pos == 0 && idx[0] == 0) {
                break;
            }
        }
    }
    // A deterministic sample of one more arm.
    let mut state = 0x9e37_79b9_7f4a_7c15u64;
    for _ in 0..sample {
        let mut idx = Vec::with_capacity(max_len + 1);
        for _ in 0..=max_len {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            idx.push(((state >> 33) as usize) % patterns.len());
        }
        check(&idx);
        compared += 1;
    }
    compared
}

fn bool_ty() -> PTy {
    PTy::Bool
}
fn opt(t: PTy) -> PTy {
    PTy::Option(Box::new(t))
}

#[test]
fn bool_by_option_bool_every_sequence_up_to_four_arms() {
    // 16 patterns: 16 + 256 + 4096 + 65536 sequences, all of them.
    let ty = PTy::Tuple(vec![bool_ty(), opt(bool_ty())]);
    assert_eq!(pats(&ty).len(), 16);
    let n = differential(&ty, 3, 0);
    assert_eq!(n, 16 + 256 + 4096);
    // Four arms exhaustively: one more level via the sample's twin.
    let n4 = {
        let patterns = pats(&ty);
        let vals = values(&ty).unwrap();
        let masks: Vec<u64> = patterns
            .iter()
            .map(|p| {
                vals.iter()
                    .enumerate()
                    .fold(0u64, |m, (i, v)| if matches(p, v) { m | (1 << i) } else { m })
            })
            .collect();
        let mut count = 0;
        for a in 0..16 {
            for b in 0..16 {
                for c in 0..16 {
                    for d in 0..16 {
                        let idx = [a, b, c, d];
                        let arms: Vec<Pat> = idx.iter().map(|i| patterns[*i].clone()).collect();
                        let ms: Vec<u64> = idx.iter().map(|i| masks[*i]).collect();
                        let mut budget = usize::MAX;
                        let got = analyze(&arms, &ty, &mut budget).unwrap();
                        assert_eq!(got, oracle(&ms, vals.len()), "{arms:?}");
                        count += 1;
                    }
                }
            }
        }
        count
    };
    assert_eq!(n4, 65_536);
}

#[test]
fn bool_by_option_bool_by_three_constructor_variant() {
    // The type of section 4.11: Bool x Option<Bool> x a 3-constructor variant
    // (61 patterns, 18 values).  Every sequence up to three arms and a
    // fixed-seed sample of four-arm sequences.
    let ty = PTy::Tuple(vec![bool_ty(), opt(bool_ty()), PTy::Data(vec![vec![]; 3])]);
    assert_eq!(pats(&ty).len(), 61);
    assert_eq!(values(&ty).unwrap().len(), 18);
    let n = differential(&ty, 3, 120_000);
    assert_eq!(n, 61 + 61 * 61 + 61 * 61 * 61 + 120_000);
}

#[test]
fn results_error_cases_and_nested_options() {
    let ty = PTy::Tuple(vec![
        PTy::Result(Box::new(bool_ty()), Box::new(PTy::ErrCases(2))),
        opt(opt(bool_ty())),
    ]);
    assert_eq!(values(&ty).unwrap().len(), 16);
    differential(&ty, 3, 20_000);
}

#[test]
fn variants_with_fields_and_records() {
    // variant V = A(Bool, Bool) | B(Option<Bool>) | C   x   record (Bool)
    let v = PTy::Data(vec![vec![bool_ty(), bool_ty()], vec![opt(bool_ty())], vec![]]);
    let r = PTy::Data(vec![vec![bool_ty()]]);
    let ty = PTy::Tuple(vec![v, r]);
    assert_eq!(values(&ty).unwrap().len(), (4 + 3 + 1) * 2);
    differential(&ty, 3, 30_000);
}

#[test]
fn single_column_types_every_sequence() {
    for ty in [
        bool_ty(),
        opt(bool_ty()),
        opt(opt(bool_ty())),
        PTy::ErrCases(3),
        PTy::Data(vec![vec![]; 4]),
        PTy::Result(Box::new(PTy::ErrCases(2)), Box::new(PTy::ErrCases(2))),
    ] {
        let k = pats(&ty).len();
        let n = differential(&ty, 4, 0);
        assert_eq!(n, k + k * k + k * k * k + k * k * k * k, "{ty:?}");
    }
}

#[test]
fn opaque_columns_take_only_wildcards() {
    // An Int/Bytes/List scrutinee has no constructor patterns: a wildcard is
    // exhaustive, a second arm is redundant.
    let mut budget = usize::MAX;
    let r = analyze(&[Pat::Wild], &PTy::Opaque, &mut budget).unwrap();
    assert_eq!(
        r,
        MatchReport {
            exhaustive: true,
            redundant: vec![]
        }
    );
    let r = analyze(&[Pat::Wild, Pat::Wild], &PTy::Opaque, &mut budget).unwrap();
    assert_eq!(r.redundant, vec![1]);
    // No arms at all is not exhaustive.
    let r = analyze(&[], &PTy::Opaque, &mut budget).unwrap();
    assert!(!r.exhaustive);
    // Inside a tuple the opaque column is transparent.
    let ty = PTy::Tuple(vec![PTy::Opaque, bool_ty()]);
    let arms = [
        Pat::Ctor(0, vec![Pat::Wild, Pat::Ctor(1, vec![])]),
        Pat::Ctor(0, vec![Pat::Wild, Pat::Ctor(0, vec![])]),
    ];
    let r = analyze(&arms, &ty, &mut budget).unwrap();
    assert!(r.exhaustive && r.redundant.is_empty());
    assert!(values(&PTy::Opaque).is_none());
    assert_eq!(PTy::Opaque.ctor_count(), None);
    assert_eq!(bool_ty().ctor_count(), Some(2));
    // A constructor pattern against an opaque column is never useful (the
    // checker never builds one; the algorithm answers conservatively).
    assert_eq!(
        useful(&[], &[Pat::Ctor(0, vec![])], &[PTy::Opaque], &mut budget),
        Ok(false)
    );
}

#[test]
fn the_work_budget_is_charged_and_exhaustion_is_reported() {
    let ty = PTy::Tuple(vec![bool_ty(), opt(bool_ty()), PTy::Data(vec![vec![]; 3])]);
    let arms: Vec<Pat> = pats(&ty).into_iter().take(40).collect();
    let mut unlimited = usize::MAX;
    assert!(analyze(&arms, &ty, &mut unlimited).is_ok());
    let spent = usize::MAX - unlimited;
    assert!(spent > 100, "a real match costs real work: {spent}");
    // The same match with a budget below that cost is refused, never guessed.
    let mut small = spent / 2;
    assert_eq!(analyze(&arms, &ty, &mut small), Err(Exhausted));
    assert_eq!(small, 0);
    // A zero budget fails at once.
    let mut none = 0;
    assert_eq!(analyze(&[Pat::Wild], &bool_ty(), &mut none), Err(Exhausted));
}

#[test]
fn a_mutant_checker_would_be_caught_by_this_oracle() {
    // Self-test of the differential: a checker that treated redundancy as
    // "same as an earlier arm" only (missing joint coverage) disagrees with the
    // oracle on [(true,_), (_,true)..., (false,false), (true,true)].
    let ty = PTy::Tuple(vec![bool_ty(), bool_ty()]);
    let t = |c| Pat::Ctor(c, vec![]);
    let arms = vec![
        Pat::Ctor(0, vec![t(1), Pat::Wild]),
        Pat::Ctor(0, vec![Pat::Wild, t(1)]),
        Pat::Ctor(0, vec![t(0), t(0)]),
        Pat::Ctor(0, vec![t(1), t(1)]),
    ];
    let mut budget = usize::MAX;
    let report = analyze(&arms, &ty, &mut budget).unwrap();
    assert_eq!(report.redundant, vec![3], "jointly covered by arms 0 and 1");
    assert!(report.exhaustive);
    let naive: Vec<usize> = (0..arms.len())
        .filter(|i| arms[..*i].contains(&arms[*i]))
        .collect();
    assert_ne!(naive, report.redundant, "the naive rule misses arm 3");
}
