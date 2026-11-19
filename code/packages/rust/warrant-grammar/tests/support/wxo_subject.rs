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

/// A result whose `Drop` allocates: the export allocates nothing, the drop of what it
/// returns does.  Clone is derived, so the harness marshals a copy and drops the original
/// inside a window.
#[derive(Clone)]
pub struct Hold {
    pub v: u8,
}

impl Drop for Hold {
    fn drop(&mut self) {
        let scratch = vec![self.v; 512];
        std::hint::black_box(&scratch);
    }
}

pub fn held(k: u8) -> Hold {
    Hold { v: k }
}

/// Drop glue and not Clone: the harness cannot split a copy off, so it must refuse the
/// instance instead of passing it unobserved.
pub struct Raw {
    pub v: u8,
}

impl Drop for Raw {
    fn drop(&mut self) {
        std::hint::black_box(&self.v);
    }
}

pub fn raw(k: u8) -> Raw {
    Raw { v: k }
}
