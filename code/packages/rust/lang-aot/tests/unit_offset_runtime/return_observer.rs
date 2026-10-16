//! Test-only typed return evidence for exact unchanged real-result sources.
//! Ordinary matrix runs remain inert. A guard binds one case/source/backend,
//! accepts one actual execute-Ok Float, and removes its state even on unwind.
use super::{Backend,Vm,Jit};
use std::cell::RefCell;
use vm_core::value::Value;

struct Armed {
    source: &'static str,
    backend: Backend,
    case: usize,
    visits: usize,
    bits: Option<u64>,
}
thread_local! { static ACTIVE: RefCell<Option<Armed>> = const { RefCell::new(None) }; }

// A guard belongs to the thread whose slot it armed. Encoding that ownership
// prevents future callers from moving its Drop onto a different thread.
pub(super) struct Observation { _thread: std::marker::PhantomData<std::rc::Rc<()>> }
impl Observation {
    pub(super) fn arm(source: &'static str, backend: Backend, case: usize) -> Self {
        assert!(matches!(backend,Vm|Jit),"typed engine observation is only for VM/JIT");
        ACTIVE.with(|slot| {
            let mut state=slot.borrow_mut();
            assert!(state.is_none(),"a prior typed engine observation is still armed");
            *state=Some(Armed{source,backend,case,visits:0,bits:None});
        });
        Self { _thread: std::marker::PhantomData }
    }

    pub(super) fn assert_float(&self,expected: f64) -> u64 {
        ACTIVE.with(|slot| {
            let state=slot.borrow();
            let state=state.as_ref().expect("typed observation guard must remain armed");
            assert_eq!(state.visits,1,"case={} {:?}: need one fresh execute-Ok return",state.case,state.backend);
            assert_eq!(state.bits,Some(expected.to_bits()),"case={} {:?}: exact Float return",state.case,state.backend);
            state.bits.expect("exact float bits checked")
        })
    }
}
impl Drop for Observation {
    fn drop(&mut self) { ACTIVE.with(|slot| {slot.borrow_mut().take();}); }
}

pub(super) fn observe(backend: Backend,source: &'static str,value: Option<&Value>) {
    ACTIVE.with(|slot| {
        let mut state=slot.borrow_mut();
        let Some(state)=state.as_mut() else {return;};
        assert_eq!(backend,state.backend,"case={}: wrong engine",state.case);
        assert_eq!(source,state.source,"case={}: wrong/stale source",state.case);
        state.visits=state.visits.checked_add(1).expect("bounded observation count");
        state.bits=match value {Some(Value::Float(number))=>Some(number.to_bits()),_=>None};
    });
}
