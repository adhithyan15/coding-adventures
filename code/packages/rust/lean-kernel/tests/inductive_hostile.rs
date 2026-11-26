//! Hostile inductive declarations: enormous or very deep blocks must end in
//! `ResourceLimit` (or another clean error), never a panic, an overflow or a hang.
//!
//! Every case runs on a thread with a *large* stack, so passing means the kernel returned
//! an error by its own accounting, not that the native stack happened to survive.

mod common;
use common::*;
use lean_kernel::{Environment, Expr, ExprStore, KernelError, MAX_CONSTRUCTORS};

fn big_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(f)
        .unwrap()
        .join()
        .expect("the kernel panicked")
}

fn is_limit(e: &KernelError) -> bool {
    matches!(e, KernelError::ResourceLimit(_))
}

/// `Nat → Nat → ... → base` with `k` binders, built by a loop (no recursion).
fn telescope(st: &mut ExprStore, k: usize, base: Expr) -> Expr {
    let nat = c(st, "Nat");
    let mut r = base;
    for _ in 0..k {
        r = st.pi(binder("x"), nat, r);
    }
    r
}

#[test]
fn too_many_constructors_is_a_resource_limit() {
    big_stack(|| {
        let mut env = std_env();
        let st = &mut env.store;
        let (t1, a) = (sort(st, 1), c(st, "A"));
        let ctors: Vec<_> = (0..MAX_CONSTRUCTORS + 1)
            .map(|i| ctor(&format!("A.c{i}"), a))
            .collect();
        let before = env.len();
        let e = env
            .add_inductive(single(&[], 0, "A", t1, ctors))
            .unwrap_err();
        assert!(is_limit(&e), "{e:?}");
        assert_eq!(env.len(), before);
    });
}

#[test]
fn a_recursor_too_deep_to_check_is_a_resource_limit_not_an_overflow() {
    // MAX_CONSTRUCTORS constructors are allowed, but then the recursor type is a Pi
    // telescope one binder per minor premise: its universe level (a nested `imax` chain,
    // limited to depth 128) and its term depth (limit 1000) are refused by the checker.
    big_stack(|| {
        let mut env = std_env();
        let st = &mut env.store;
        let (t1, a) = (sort(st, 1), c(st, "A"));
        let ctors: Vec<_> = (0..MAX_CONSTRUCTORS)
            .map(|i| ctor(&format!("A.c{i}"), a))
            .collect();
        let before = env.len();
        let e = env
            .add_inductive(single(&[], 0, "A", t1, ctors))
            .unwrap_err();
        assert!(is_limit(&e), "{e:?}");
        assert_eq!(env.len(), before, "rolled back");
        for i in 0..3 {
            assert!(!env.contains(&n(&format!("A.c{i}"))));
        }
    });
}

#[test]
fn a_moderate_number_of_constructors_is_fine() {
    big_stack(|| {
        let mut env = std_env();
        let st = &mut env.store;
        let (t1, a) = (sort(st, 1), c(st, "A"));
        let ctors: Vec<_> = (0..100).map(|i| ctor(&format!("A.c{i}"), a)).collect();
        env.add_inductive(single(&[], 0, "A", t1, ctors)).unwrap();
        assert!(env.contains(&n("A.rec")));
    });
}

#[test]
fn a_constructor_with_thousands_of_fields_is_a_resource_limit() {
    big_stack(|| {
        let mut env = std_env();
        let st = &mut env.store;
        let (t1, a) = (sort(st, 1), c(st, "A"));
        let mk = telescope(st, 5_000, a);
        let e = env
            .add_inductive(single(&[], 0, "A", t1, vec![ctor("A.mk", mk)]))
            .unwrap_err();
        assert!(is_limit(&e), "{e:?}");
    });
}

#[test]
fn a_hundred_thousand_deep_constructor_telescope_is_a_resource_limit() {
    big_stack(|| {
        let mut env = std_env();
        let st = &mut env.store;
        let (t1, a) = (sort(st, 1), c(st, "A"));
        let mk = telescope(st, 100_000, a);
        let e = env
            .add_inductive(single(&[], 0, "A", t1, vec![ctor("A.mk", mk)]))
            .unwrap_err();
        assert!(is_limit(&e), "{e:?}");
        assert!(!env.contains(&n("A")), "rolled back");
    });
}

#[test]
fn a_hundred_thousand_deep_inductive_type_is_a_resource_limit() {
    big_stack(|| {
        let mut env = std_env();
        let t1 = sort(&mut env.store, 1);
        let ty = telescope(&mut env.store, 100_000, t1);
        let e = env
            .add_inductive(single(&[], 0, "A", ty, vec![]))
            .unwrap_err();
        assert!(is_limit(&e), "{e:?}");
    });
}

#[test]
fn hundreds_of_fields_succeed_or_fail_cleanly() {
    big_stack(|| {
        for k in [100usize, 400, 900] {
            let mut env = std_env();
            let st = &mut env.store;
            let (t1, a) = (sort(st, 1), c(st, "A"));
            let mk = telescope(st, k, a);
            match env.add_inductive(single(&[], 0, "A", t1, vec![ctor("A.mk", mk)])) {
                Ok(()) => assert!(env.contains(&n("A.rec"))),
                Err(e) => assert!(is_limit(&e), "{k}: {e:?}"),
            }
        }
    });
}

#[test]
fn a_long_recursive_function_field_succeeds_or_fails_cleanly() {
    // mk : (Nat → Nat → ... → A) → A   with a long telescope inside the field
    big_stack(|| {
        for k in [50usize, 500, 900] {
            let mut env = std_env();
            let st = &mut env.store;
            let (t1, a) = (sort(st, 1), c(st, "A"));
            let field = telescope(st, k, a);
            let mk = arrow(st, field, a);
            match env.add_inductive(single(&[], 0, "A", t1, vec![ctor("A.mk", mk)])) {
                Ok(()) => assert!(env.contains(&n("A.rec"))),
                Err(e) => assert!(is_limit(&e), "{k}: {e:?}"),
            }
        }
    });
}

#[test]
fn many_parameters_succeed_or_fail_cleanly() {
    big_stack(|| {
        for k in [10usize, 300, 700] {
            let mut env = std_env();
            let st = &mut env.store;
            let (t1, nat) = (sort(st, 1), c(st, "Nat"));
            // A (x1 ... xk : Nat) : Type | mk : A x1 ... xk, with the parameters named
            // by one loop of binders
            let mut ty = t1;
            for _ in 0..k {
                ty = st.pi(binder("p"), nat, ty);
            }
            let a = c(st, "A");
            let mut body = a;
            for i in 0..k {
                let v = st.bvar(u32::try_from(k - 1 - i).unwrap());
                body = st.app(body, v);
            }
            let mut mk = body;
            for _ in 0..k {
                mk = st.pi(binder("p"), nat, mk);
            }
            match env.add_inductive(single(&[], k, "A", ty, vec![ctor("A.mk", mk)])) {
                Ok(()) => assert!(env.contains(&n("A.rec"))),
                Err(e) => assert!(is_limit(&e), "{k}: {e:?}"),
            }
        }
    });
}

#[test]
fn an_exponentially_shared_field_type_is_handled_in_linear_time() {
    // t_{k+1} = t_k → t_k : a DAG with 2^80 paths but 80 nodes. No occurrence of A, so
    // it is a plain non-recursive field; the occurrence scan must not walk the tree.
    big_stack(|| {
        let mut env = std_env();
        let st = &mut env.store;
        let (t1, a, nat) = (sort(st, 1), c(st, "A"), c(st, "Nat"));
        let mut t = nat;
        for _ in 0..80 {
            t = st.pi(binder("_"), t, t);
        }
        let mk = arrow(st, t, a);
        let r = env.add_inductive(single(&[], 0, "A", t1, vec![ctor("A.mk", mk)]));
        assert!(
            matches!(&r, Ok(()) | Err(KernelError::ResourceLimit(_))),
            "{r:?}"
        );
        // and with an occurrence of A at the far end it is rejected, not walked forever
        let mut env2: Environment = std_env();
        let st = &mut env2.store;
        let (t1, a) = (sort(st, 1), c(st, "A"));
        let mut t = a;
        for _ in 0..80 {
            t = st.pi(binder("_"), t, t);
        }
        let mk = arrow(st, t, a);
        let r = env2.add_inductive(single(&[], 0, "A", t1, vec![ctor("A.mk", mk)]));
        assert!(r.is_err(), "{r:?}");
    });
}

#[test]
fn about_nine_hundred_nullary_constructors_are_refused_quickly() {
    // Each constructor adds one binder to the recursor type, whose universe level cannot
    // be typed past 128 binders. The kernel must say so up front, not after hundreds of
    // millions of unmetered abstractions.
    big_stack(|| {
        for count in [129usize, 800, 950] {
            let mut env = std_env();
            let st = &mut env.store;
            let (t1, a) = (sort(st, 1), c(st, "A"));
            let ctors: Vec<_> = (0..count).map(|i| ctor(&format!("A.c{i}"), a)).collect();
            let before = env.len();
            let start = std::time::Instant::now();
            let e = env
                .add_inductive(single(&[], 0, "A", t1, ctors))
                .unwrap_err();
            let took = start.elapsed();
            assert!(is_limit(&e), "{count}: {e:?}");
            assert_eq!(env.len(), before);
            assert!(took.as_millis() < 500, "{count}: took {took:?}");
        }
    });
}
