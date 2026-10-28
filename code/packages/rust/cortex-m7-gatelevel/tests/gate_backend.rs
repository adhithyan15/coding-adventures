use cortex_m7_gatelevel::{
    encoding::{qadd, qsub, sadd8, uadd8, ApsrMask},
    gates::{self, GateBackend, RepositoryGates},
};

#[derive(Default)]
struct Hostile {
    calls: usize,
    invert_at: Option<usize>,
}

impl Hostile {
    fn output(&mut self, value: u8) -> u8 {
        let call = self.calls;
        self.calls += 1;
        if self.invert_at == Some(call) {
            value ^ 1
        } else {
            value
        }
    }
}

impl GateBackend for Hostile {
    fn and(&mut self, left: u8, right: u8) -> u8 {
        self.output(left & right)
    }
    fn or(&mut self, left: u8, right: u8) -> u8 {
        self.output(left | right)
    }
    fn xor(&mut self, left: u8, right: u8) -> u8 {
        self.output(left ^ right)
    }
    fn not(&mut self, value: u8) -> u8 {
        self.output(value ^ 1)
    }
    fn mux(&mut self, d0: u8, d1: u8, select: u8) -> u8 {
        self.output(if select == 0 { d0 } else { d1 })
    }
}

fn mutation_changes<T: Eq + core::fmt::Debug>(expected: T, run: impl Fn(&mut Hostile) -> T) {
    let mut counter = Hostile::default();
    assert_eq!(run(&mut counter), expected);
    assert!(counter.calls > 0);
    assert!((0..counter.calls).any(|call| {
        run(&mut Hostile {
            calls: 0,
            invert_at: Some(call),
        }) != expected
    }));
}

#[test]
fn every_named_compiled_helper_depends_on_the_gate_backend() {
    let expected = gates::dsp_prefix(&mut RepositoryGates, qadd(0, 1, 2)[0]);
    mutation_changes(expected, |backend| {
        gates::dsp_prefix(backend, qadd(0, 1, 2)[0])
    });
    for words in [qadd(0, 1, 2), qsub(0, 1, 2), sadd8(0, 1, 2), uadd8(0, 1, 2)] {
        let expected = gates::classify(&mut RepositoryGates, words[0], words[1]);
        mutation_changes(expected, |backend| {
            gates::classify(backend, words[0], words[1])
        });
    }

    let words = qadd(0, 1, 2);
    let class = gates::classify(&mut RepositoryGates, words[0], words[1]).unwrap();
    let expected = gates::fields(&mut RepositoryGates, words[0], words[1], class);
    mutation_changes(expected, |backend| {
        gates::fields(backend, words[0], words[1], class)
    });

    for subtract in [0, 1] {
        for old_q in [0, 1] {
            let expected = gates::saturating(&mut RepositoryGates, 0x7fff_ffff, 1, subtract, old_q);
            mutation_changes(expected, |backend| {
                gates::saturating(backend, 0x7fff_ffff, 1, subtract, old_q)
            });
        }
    }
    for signed in [0, 1] {
        let expected = gates::parallel_add(&mut RepositoryGates, 0x7f80_ff01, 0x0101_01ff, signed);
        mutation_changes(expected, |backend| {
            gates::parallel_add(backend, 0x7f80_ff01, 0x0101_01ff, signed)
        });
    }

    let expected = gates::condition_holds(&mut RepositoryGates, 1 << 29, 8);
    mutation_changes(expected, |backend| {
        gates::condition_holds(backend, 1 << 29, 8)
    });
    let expected = gates::it_active(&mut RepositoryGates, 0x18);
    mutation_changes(expected, |backend| gates::it_active(backend, 0x18));
    let expected = gates::advance_itstate(&mut RepositoryGates, 0x24);
    mutation_changes(expected, |backend| gates::advance_itstate(backend, 0x24));
    let expected = gates::write_enable(&mut RepositoryGates, 1, 0);
    mutation_changes(expected, |backend| gates::write_enable(backend, 1, 0));
    let expected = gates::transport_ge(&mut RepositoryGates, 3, 0x000b_0000, 1);
    mutation_changes(expected, |backend| {
        gates::transport_ge(backend, 3, 0x000b_0000, 1)
    });
    for words in [
        cortex_m7_gatelevel::encoding::msr_masked(0, 0, ApsrMask::Ge),
        cortex_m7_gatelevel::encoding::msr_masked(0, 0, ApsrMask::Nzcvq),
        cortex_m7_gatelevel::encoding::msr_masked(0, 0, ApsrMask::NzcvqGe),
    ] {
        let expected = gates::apsr_msr_mask(&mut RepositoryGates, words[1]);
        mutation_changes(expected, |backend| gates::apsr_msr_mask(backend, words[1]));
        let expected = gates::apsr_msr_owned(&mut RepositoryGates, words[0], words[1]);
        mutation_changes(expected, |backend| {
            gates::apsr_msr_owned(backend, words[0], words[1])
        });
    }
    let expected = gates::msr_prefix(&mut RepositoryGates, 0xf380);
    mutation_changes(expected, |backend| gates::msr_prefix(backend, 0xf380));
    let expected = gates::msr_source(&mut RepositoryGates, 0xf38d);
    mutation_changes(expected, |backend| gates::msr_source(backend, 0xf38d));
    for write in 0..=1 {
        let expected =
            gates::transport_nzcvq(&mut RepositoryGates, 0x6000_0000, 0x5007_0000, write);
        mutation_changes(expected, |backend| {
            gates::transport_nzcvq(backend, 0x6000_0000, 0x5007_0000, write)
        });
    }
    let words = cortex_m7_gatelevel::encoding::mrs(1, 0);
    let expected = gates::apsr_transport_lines(&mut RepositoryGates, words[0], words[1]);
    mutation_changes(expected, |backend| {
        gates::apsr_transport_lines(backend, words[0], words[1])
    });
}
