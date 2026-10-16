//! A public huge-finite call runs only in this test binary's bounded child.
//! The compatibility wrapper must reject unsupported geometry explicitly; its
//! caller must never be trapped in the former floating-point hatch loop.
use std::process::{Command,Stdio};
use std::time::{Duration,Instant};

// Drop owns only this freshly spawned test child. Error/panic unwinding also
// attempts kill+wait, so an OS query/kill error cannot silently abandon it.
struct OwnedChild(Option<std::process::Child>);
impl OwnedChild {
    fn process(&mut self)->&mut std::process::Child {self.0.as_mut().expect("owned probe child")}
    fn finish(mut self)->std::process::Output {self.0.take().expect("owned probe child").wait_with_output().expect("consume owned child")}
}
impl Drop for OwnedChild {
    fn drop(&mut self) {if let Some(child)=self.0.as_mut() {let _=child.kill();let _=child.wait();}}
}

const CHILD_FLAG:&str="CV04_ISHIKAWA_HUGE_CANVAS_CHILD";

#[test]
fn cv04_ishikawa_legacy_huge_canvas_child() {
    if std::env::var_os(CHILD_FLAG).as_deref()!=Some(std::ffi::OsStr::new("1")) {return;}
    let diagram=diagram_ir::IshikawaDiagram {
        effect:"E".into(),causes:Vec::new(),
        config:diagram_ir::IshikawaConfig {
            look:diagram_ir::IshikawaLook::HandDrawn,
            ..Default::default()
        },
    };
    let layout=diagram_layout_geometric::layout_ishikawa(&diagram,1e20);
    println!("CV04_UNCHECKED_LAYOUT_RETURNED bones={}",layout.bones.len());
}

#[test]
fn cv04_ishikawa_huge_finite_legacy_wrapper_refuses_within_bound() {
    let mut command=Command::new(std::env::current_exe().expect("current liveness test binary"));
    command.args(["--exact","cv04_ishikawa_legacy_huge_canvas_child","--nocapture"])
        .env(CHILD_FLAG,"1").stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(windows)] {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child=OwnedChild(Some(command.spawn().expect("spawn only owned liveness child")));
    let child_id=child.process().id();
    let start=Instant::now();
    let bound=Duration::from_secs(3);
    loop {
        if child.process().try_wait().expect("query owned child").is_some() {break;}
        if start.elapsed()>=bound {
            child.process().kill().expect("terminate only this owned liveness child");
            let output=child.finish();
            panic!("CV04_ISHIKAWA_TIMEOUT child={child_id} bound_ms={} actual_ms={} consumed_code={:?} stdout={:?} stderr={:?}",
                bound.as_millis(),start.elapsed().as_millis(),output.status.code(),
                String::from_utf8_lossy(&output.stdout),String::from_utf8_lossy(&output.stderr));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let output=child.finish();
    assert!(!output.status.success(),"unsupported huge geometry must refuse explicitly, not return a layout: {:?}",String::from_utf8_lossy(&output.stdout));
    let diagnostic=String::from_utf8_lossy(&output.stderr);
    assert!(diagnostic.contains("unsupported Ishikawa geometry"),"expected explicit compatibility-wrapper diagnostic, got {diagnostic:?}");
    assert!(!String::from_utf8_lossy(&output.stdout).contains("CV04_UNCHECKED_LAYOUT_RETURNED"));
    println!("CV04_ISHIKAWA_BOUNDED_REFUSAL child={child_id} elapsed_ms={} code={:?}",start.elapsed().as_millis(),output.status.code());
}


#[test]
fn cv04_public_checked_ishikawa_returns_errors_before_output() {
    use diagram_layout_geometric::{try_layout_ishikawa,IshikawaLayoutError};
    let mut d=diagram_ir::IshikawaDiagram {effect:"E".into(),causes:Vec::new(),config:Default::default()};
    let ordinary=try_layout_ishikawa(&d,640.0).expect("ordinary classic geometry");
    assert!(ordinary.effect_hatches.is_empty());
    assert_eq!(try_layout_ishikawa(&d,f64::NAN),Err(IshikawaLayoutError::DerivedGeometry));
    assert_eq!(try_layout_ishikawa(&d,f64::INFINITY),Err(IshikawaLayoutError::DerivedGeometry));
    assert!(try_layout_ishikawa(&d,1e12).is_ok(),"classic does not acquire hatch-only bounds");
    d.config.look=diagram_ir::IshikawaLook::HandDrawn;
    assert_eq!(try_layout_ishikawa(&d,1e20),Err(IshikawaLayoutError::DerivedGeometry));
    let ordinary=try_layout_ishikawa(&d,640.0).expect("ordinary hand-drawn hatching");
    assert!(!ordinary.effect_hatches.is_empty());
    assert_eq!(ordinary.config.look,diagram_ir::IshikawaLook::HandDrawn);
}
