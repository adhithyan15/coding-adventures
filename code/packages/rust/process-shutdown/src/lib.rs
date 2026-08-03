//! Cross-platform cooperative process-shutdown notification.
//!
//! Native signal and console callbacks are severely restricted execution
//! contexts. This crate keeps them minimal: the handler only stores a small
//! integer in a lock-free atomic. A dedicated Rust thread observes that value
//! and invokes the caller's callback at most once in ordinary thread context.

#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

use std::fmt::{self, Display, Formatter};
use std::io;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

const EVENT_NONE: u8 = 0;
const EVENT_INTERRUPT: u8 = 1;
const EVENT_TERMINATE: u8 = 2;
const POLL_INTERVAL: Duration = Duration::from_millis(10);

static ACTIVE: AtomicBool = AtomicBool::new(false);
static PENDING_EVENT: AtomicU8 = AtomicU8::new(EVENT_NONE);

/// A portable reason why the operating system requested process shutdown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShutdownEvent {
    /// Interactive interruption, such as Unix `SIGINT`, Windows Ctrl+C, or
    /// Windows Ctrl+Break.
    Interrupt,
    /// Service or session termination, such as Unix `SIGTERM`, Windows console
    /// close, user logoff, or system shutdown.
    Terminate,
}

impl ShutdownEvent {
    fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            EVENT_INTERRUPT => Some(Self::Interrupt),
            EVENT_TERMINATE => Some(Self::Terminate),
            _ => None,
        }
    }
}

impl Display for ShutdownEvent {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Interrupt => "interrupt",
            Self::Terminate => "terminate",
        })
    }
}

/// Stable installation, restoration, and worker failures.
#[derive(Debug)]
pub enum ShutdownError {
    /// This process already owns a live shutdown listener.
    AlreadyInstalled,
    /// The target does not expose a supported native shutdown API.
    UnsupportedPlatform,
    /// The operating system refused to install the native handler.
    HandlerInstall(io::Error),
    /// The operating system refused to restore the previous handler.
    HandlerRestore(io::Error),
    /// The operating system refused to create the callback worker thread.
    ThreadUnavailable(io::Error),
    /// The caller's shutdown callback panicked.
    CallbackPanicked,
}

impl Display for ShutdownError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::AlreadyInstalled => "process shutdown listener is already installed",
            Self::UnsupportedPlatform => "process shutdown signals are unsupported on this target",
            Self::HandlerInstall(_) => "process shutdown handler installation failed",
            Self::HandlerRestore(_) => "process shutdown handler restoration failed",
            Self::ThreadUnavailable(_) => "process shutdown callback thread is unavailable",
            Self::CallbackPanicked => "process shutdown callback panicked",
        })
    }
}

impl std::error::Error for ShutdownError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::HandlerInstall(error)
            | Self::HandlerRestore(error)
            | Self::ThreadUnavailable(error) => Some(error),
            Self::AlreadyInstalled | Self::UnsupportedPlatform | Self::CallbackPanicked => None,
        }
    }
}

/// Exclusive process-global native handler and callback worker.
///
/// Only one listener may be installed at a time. Dropping the listener stops
/// its worker and restores the native handlers that were present before
/// installation. Call [`Self::uninstall`] when restoration errors must be
/// observed explicitly.
pub struct ShutdownListener {
    cancellation: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    registration: Option<platform::Registration>,
}

impl ShutdownListener {
    /// Install the process-global handler and invoke `callback` at most once.
    ///
    /// The callback runs on a dedicated thread named `process-shutdown`, never
    /// inside the native signal or console callback. Delivery latency is
    /// bounded by the worker's 10 millisecond polling interval.
    pub fn install<F>(callback: F) -> Result<Self, ShutdownError>
    where
        F: FnOnce(ShutdownEvent) + Send + 'static,
    {
        ACTIVE
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| ShutdownError::AlreadyInstalled)?;
        PENDING_EVENT.store(EVENT_NONE, Ordering::Release);

        let registration = match platform::install() {
            Ok(registration) => registration,
            Err(error) => {
                ACTIVE.store(false, Ordering::Release);
                return Err(map_install_error(error));
            }
        };
        let cancellation = Arc::new(AtomicBool::new(false));
        let worker_cancellation = Arc::clone(&cancellation);
        let worker = match thread::Builder::new()
            .name("process-shutdown".to_string())
            .spawn(move || worker_loop(worker_cancellation, callback))
        {
            Ok(worker) => worker,
            Err(error) => {
                let restore_result = platform::uninstall(&registration);
                if restore_result.is_ok() {
                    ACTIVE.store(false, Ordering::Release);
                }
                return Err(ShutdownError::ThreadUnavailable(error));
            }
        };

        Ok(Self {
            cancellation,
            worker: Some(worker),
            registration: Some(registration),
        })
    }

    /// Stop listening and report callback panic or native restoration failure.
    pub fn uninstall(mut self) -> Result<(), ShutdownError> {
        self.finish()
    }

    fn finish(&mut self) -> Result<(), ShutdownError> {
        if let Some(registration) = self.registration.as_ref() {
            if let Err(error) = platform::uninstall(registration) {
                return Err(ShutdownError::HandlerRestore(error));
            }
            self.registration.take();
            ACTIVE.store(false, Ordering::Release);
        }

        self.cancellation.store(true, Ordering::Release);
        let callback_panicked = self
            .worker
            .take()
            .is_some_and(|worker| worker.join().is_err());
        PENDING_EVENT.store(EVENT_NONE, Ordering::Release);
        if callback_panicked {
            return Err(ShutdownError::CallbackPanicked);
        }
        Ok(())
    }
}

impl Drop for ShutdownListener {
    fn drop(&mut self) {
        let _ = self.finish();
    }
}

fn worker_loop<F>(cancellation: Arc<AtomicBool>, callback: F)
where
    F: FnOnce(ShutdownEvent),
{
    while !cancellation.load(Ordering::Acquire) {
        let raw = PENDING_EVENT.swap(EVENT_NONE, Ordering::AcqRel);
        if let Some(event) = ShutdownEvent::from_raw(raw) {
            callback(event);
            return;
        }
        thread::sleep(POLL_INTERVAL);
    }
}

fn record_event(raw: u8) {
    let _ = PENDING_EVENT.compare_exchange(EVENT_NONE, raw, Ordering::AcqRel, Ordering::Acquire);
}

fn map_install_error(error: io::Error) -> ShutdownError {
    if error.kind() == io::ErrorKind::Unsupported {
        ShutdownError::UnsupportedPlatform
    } else {
        ShutdownError::HandlerInstall(error)
    }
}

#[cfg(unix)]
mod platform {
    use super::{io, record_event, EVENT_INTERRUPT, EVENT_TERMINATE};
    use std::mem::MaybeUninit;

    pub struct Registration {
        previous_interrupt: libc::sigaction,
        previous_terminate: libc::sigaction,
    }

    unsafe extern "C" fn signal_handler(signal: libc::c_int) {
        match signal {
            libc::SIGINT => record_event(EVENT_INTERRUPT),
            libc::SIGTERM => record_event(EVENT_TERMINATE),
            _ => {}
        }
    }

    pub fn install() -> io::Result<Registration> {
        let action = new_action()?;
        let previous_interrupt = install_one(libc::SIGINT, &action)?;
        match install_one(libc::SIGTERM, &action) {
            Ok(previous_terminate) => Ok(Registration {
                previous_interrupt,
                previous_terminate,
            }),
            Err(error) => {
                let _ = restore_one(libc::SIGINT, &previous_interrupt);
                Err(error)
            }
        }
    }

    pub fn uninstall(registration: &Registration) -> io::Result<()> {
        let interrupt_result = restore_one(libc::SIGINT, &registration.previous_interrupt);
        let terminate_result = restore_one(libc::SIGTERM, &registration.previous_terminate);
        interrupt_result.and(terminate_result)
    }

    fn new_action() -> io::Result<libc::sigaction> {
        let mut action = MaybeUninit::<libc::sigaction>::zeroed();
        // SAFETY: `action` points to writable, correctly aligned sigaction
        // storage. sigemptyset initializes only the embedded mask.
        let result = unsafe { libc::sigemptyset(&raw mut (*action.as_mut_ptr()).sa_mask) };
        if result != 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: zero is a valid baseline for sigaction on supported Unix
        // targets, and sigemptyset initialized the mask before this read.
        let mut action = unsafe { action.assume_init() };
        action.sa_sigaction = signal_handler as *const () as usize;
        action.sa_flags = libc::SA_RESTART;
        Ok(action)
    }

    fn install_one(signal: libc::c_int, action: &libc::sigaction) -> io::Result<libc::sigaction> {
        let mut previous = MaybeUninit::<libc::sigaction>::uninit();
        // SAFETY: both pointers reference valid sigaction storage for the
        // duration of the call, and `signal` is SIGINT or SIGTERM.
        let result = unsafe { libc::sigaction(signal, action, previous.as_mut_ptr()) };
        if result != 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: successful sigaction initialized the previous-action output.
        Ok(unsafe { previous.assume_init() })
    }

    fn restore_one(signal: libc::c_int, action: &libc::sigaction) -> io::Result<()> {
        // SAFETY: `action` was returned by a successful sigaction call and the
        // null output pointer tells libc that the replaced action is unwanted.
        let result = unsafe { libc::sigaction(signal, action, std::ptr::null_mut()) };
        if result == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }
}

#[cfg(windows)]
mod platform {
    use super::{io, record_event, EVENT_INTERRUPT, EVENT_TERMINATE};
    use windows_sys::Win32::Foundation::{FALSE, TRUE};
    use windows_sys::Win32::System::Console::{
        SetConsoleCtrlHandler, CTRL_BREAK_EVENT, CTRL_CLOSE_EVENT, CTRL_C_EVENT, CTRL_LOGOFF_EVENT,
        CTRL_SHUTDOWN_EVENT,
    };

    pub struct Registration;

    unsafe extern "system" fn console_handler(event: u32) -> i32 {
        match event {
            CTRL_C_EVENT | CTRL_BREAK_EVENT => record_event(EVENT_INTERRUPT),
            CTRL_CLOSE_EVENT | CTRL_LOGOFF_EVENT | CTRL_SHUTDOWN_EVENT => {
                record_event(EVENT_TERMINATE);
            }
            _ => return FALSE,
        }
        TRUE
    }

    pub fn install() -> io::Result<Registration> {
        // SAFETY: the callback has the required ABI and remains valid for the
        // process lifetime. The listener's singleton enforces balanced removal.
        let result = unsafe { SetConsoleCtrlHandler(Some(console_handler), TRUE) };
        if result == FALSE {
            Err(io::Error::last_os_error())
        } else {
            Ok(Registration)
        }
    }

    pub fn uninstall(_registration: &Registration) -> io::Result<()> {
        // SAFETY: removes the exact callback registered by `install`.
        let result = unsafe { SetConsoleCtrlHandler(Some(console_handler), FALSE) };
        if result == FALSE {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::{EVENT_NONE, PENDING_EVENT};
        use std::sync::atomic::Ordering;

        #[test]
        fn console_events_map_to_portable_events() {
            PENDING_EVENT.store(EVENT_NONE, Ordering::Release);
            // SAFETY: direct invocation uses the ABI and event values Windows
            // supplies; the callback only writes a lock-free atomic.
            assert_eq!(unsafe { console_handler(CTRL_C_EVENT) }, TRUE);
            assert_eq!(
                PENDING_EVENT.swap(EVENT_NONE, Ordering::AcqRel),
                EVENT_INTERRUPT
            );
            // SAFETY: same reasoning as above for a service-stop-shaped event.
            assert_eq!(unsafe { console_handler(CTRL_SHUTDOWN_EVENT) }, TRUE);
            assert_eq!(
                PENDING_EVENT.swap(EVENT_NONE, Ordering::AcqRel),
                EVENT_TERMINATE
            );
            // SAFETY: an unknown value is accepted as input and must be passed
            // on to the next registered Windows handler.
            assert_eq!(unsafe { console_handler(u32::MAX) }, FALSE);
        }
    }
}

#[cfg(not(any(unix, windows)))]
mod platform {
    use super::io;

    pub struct Registration;

    pub fn install() -> io::Result<Registration> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "native process shutdown notifications are unsupported",
        ))
    }

    pub fn uninstall(_registration: &Registration) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::sync::mpsc;

    #[test]
    fn event_display_and_raw_mapping_are_stable() {
        assert_eq!(ShutdownEvent::Interrupt.to_string(), "interrupt");
        assert_eq!(ShutdownEvent::Terminate.to_string(), "terminate");
        assert_eq!(
            ShutdownEvent::from_raw(EVENT_INTERRUPT),
            Some(ShutdownEvent::Interrupt)
        );
        assert_eq!(
            ShutdownEvent::from_raw(EVENT_TERMINATE),
            Some(ShutdownEvent::Terminate)
        );
        assert_eq!(ShutdownEvent::from_raw(99), None);
    }

    #[test]
    fn error_display_is_payload_blind() {
        let error = ShutdownError::HandlerInstall(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "sensitive operating-system detail",
        ));
        assert_eq!(
            error.to_string(),
            "process shutdown handler installation failed"
        );
        assert!(std::error::Error::source(&error).is_some());
        assert_eq!(
            ShutdownError::AlreadyInstalled.to_string(),
            "process shutdown listener is already installed"
        );
        assert_eq!(
            ShutdownError::UnsupportedPlatform.to_string(),
            "process shutdown signals are unsupported on this target"
        );
        assert_eq!(
            ShutdownError::HandlerRestore(io::Error::other("private")).to_string(),
            "process shutdown handler restoration failed"
        );
        assert_eq!(
            ShutdownError::ThreadUnavailable(io::Error::other("private")).to_string(),
            "process shutdown callback thread is unavailable"
        );
        assert_eq!(
            ShutdownError::CallbackPanicked.to_string(),
            "process shutdown callback panicked"
        );
        assert!(matches!(
            map_install_error(io::Error::new(io::ErrorKind::Unsupported, "private")),
            ShutdownError::UnsupportedPlatform
        ));
    }

    #[cfg(unix)]
    #[test]
    fn unix_signals_are_delivered_outside_the_handler() {
        assert_unix_signal(libc::SIGINT, ShutdownEvent::Interrupt, true);
        assert_unix_signal(libc::SIGTERM, ShutdownEvent::Terminate, false);

        let (sender, receiver) = mpsc::channel();
        let listener = ShutdownListener::install(move |_| {
            sender.send(()).expect("receiver remains live");
            panic!("intentional callback panic");
        })
        .expect("listener installs");
        // SAFETY: SIGINT is handled by the listener installed immediately
        // above, so it records an atomic event rather than terminating tests.
        assert_eq!(unsafe { libc::raise(libc::SIGINT) }, 0);
        receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("panicking callback started");
        assert!(matches!(
            listener.uninstall(),
            Err(ShutdownError::CallbackPanicked)
        ));
        assert!(!ACTIVE.load(Ordering::Acquire));
    }

    #[cfg(unix)]
    fn assert_unix_signal(signal: libc::c_int, expected: ShutdownEvent, test_exclusivity: bool) {
        let (sender, receiver) = mpsc::channel();
        let listener = ShutdownListener::install(move |event| {
            sender
                .send((event, thread::current().name().map(str::to_owned)))
                .expect("receiver remains live");
        })
        .expect("listener installs");

        if test_exclusivity {
            assert!(matches!(
                ShutdownListener::install(|_| {}),
                Err(ShutdownError::AlreadyInstalled)
            ));
        }
        // SAFETY: `signal` is SIGINT or SIGTERM and is handled by the listener
        // installed immediately above, so the process is not terminated.
        assert_eq!(unsafe { libc::raise(signal) }, 0);
        let (event, thread_name) = receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("worker receives signal");
        assert_eq!(event, expected);
        assert_eq!(thread_name.as_deref(), Some("process-shutdown"));
        listener.uninstall().expect("handler restores");
        assert!(!ACTIVE.load(Ordering::Acquire));
    }

    #[test]
    fn worker_honors_cancellation_without_invoking_callback() {
        let cancellation = Arc::new(AtomicBool::new(true));
        let invoked = Arc::new(AtomicBool::new(false));
        let worker_invoked = Arc::clone(&invoked);
        worker_loop(cancellation, move |_| {
            worker_invoked.store(true, Ordering::Release);
        });
        assert!(!invoked.load(Ordering::Acquire));
    }
}
