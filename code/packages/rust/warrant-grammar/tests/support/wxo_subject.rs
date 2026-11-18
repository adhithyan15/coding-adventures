// The subject of tests/wxo_observer.rs (VOS-D063).  `make` and `pick` build their answers on the
// stack; `grow` allocates a kilobyte INSIDE the export and keeps it alive across the answer, which
// a `heap <= 0` row must catch.

pub struct Cfg {
    pub v: u8,
}

pub enum Pick {
    Idle,
    Run(u8),
}

pub fn make() -> Cfg {
    Cfg { v: 7 }
}

pub fn pick(k: u8) -> Pick {
    if k == 255 { Pick::Idle } else { Pick::Run(k) }
}

pub fn grow(k: u8) -> Cfg {
    let scratch = vec![k; 1024];
    std::hint::black_box(&scratch);
    Cfg { v: scratch[0] }
}
