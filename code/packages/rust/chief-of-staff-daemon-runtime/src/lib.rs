//! Fail-closed scheduling and serving for the D18 Chief daemon.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use chief_of_staff_daemon_api::{
    bind_daemon, BindAddress, ChiefControlPlane, ControlPlaneError, DaemonApi, DaemonSession,
    SessionAuthorizer,
};
use core::fmt::{self, Display, Formatter};
use std::net::SocketAddr;
use std::panic::{self, AssertUnwindSafe};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use transport_platform::TransportPlatform;
use websocket_runtime::{
    StopHandle, WebSocketRuntime, WebSocketRuntimeError, WebSocketServerOptions,
};

/// Validated cadence for bounded background reconciliation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReconcileSchedule {
    interval: Duration,
}

impl ReconcileSchedule {
    /// Construct a schedule with a non-zero interval.
    pub fn new(interval: Duration) -> Result<Self, DaemonRuntimeError> {
        if interval.is_zero() {
            return Err(DaemonRuntimeError::InvalidSchedule);
        }
        Ok(Self { interval })
    }

    /// Return the interval between completed scheduling waits.
    pub fn interval(self) -> Duration {
        self.interval
    }
}

/// Stable failure from daemon binding, scheduling, reconciliation, or serving.
#[derive(Debug)]
pub enum DaemonRuntimeError {
    /// The reconciliation interval was zero.
    InvalidSchedule,
    /// Startup or background reconciliation failed.
    Reconciliation(ControlPlaneError),
    /// The operating system refused to create the scheduler thread.
    SchedulerUnavailable,
    /// The scheduler thread panicked.
    SchedulerPanicked,
    /// The WebSocket transport could not bind or serve.
    Transport(WebSocketRuntimeError),
}

impl Display for DaemonRuntimeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidSchedule => "chief daemon runtime: invalid reconciliation schedule",
            Self::Reconciliation(_) => "chief daemon runtime: reconciliation failed",
            Self::SchedulerUnavailable => "chief daemon runtime: scheduler unavailable",
            Self::SchedulerPanicked => "chief daemon runtime: scheduler panicked",
            Self::Transport(_) => "chief daemon runtime: transport failed",
        })
    }
}

impl std::error::Error for DaemonRuntimeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Transport(error) => Some(error),
            Self::InvalidSchedule
            | Self::Reconciliation(_)
            | Self::SchedulerUnavailable
            | Self::SchedulerPanicked => None,
        }
    }
}

impl From<WebSocketRuntimeError> for DaemonRuntimeError {
    fn from(error: WebSocketRuntimeError) -> Self {
        Self::Transport(error)
    }
}

#[derive(Default)]
struct SchedulerState {
    stopped: bool,
    failure: Option<ControlPlaneError>,
}

/// Authenticated WebSocket listener plus mandatory reconciliation scheduler.
pub struct ChiefDaemonRuntime<P, C, A>
where
    A: SessionAuthorizer,
{
    websocket: WebSocketRuntime<P, DaemonSession<A::Session>>,
    api: Arc<DaemonApi<C, A>>,
    schedule: ReconcileSchedule,
}

impl<P, C, A> ChiefDaemonRuntime<P, C, A>
where
    P: TransportPlatform,
    C: ChiefControlPlane + Send + 'static,
    A: SessionAuthorizer + Send + Sync + 'static,
    A::Session: Send + 'static,
{
    /// Bind the caller-supplied API and schedule to an explicit listener.
    pub fn bind(
        platform: P,
        address: BindAddress,
        options: WebSocketServerOptions,
        api: Arc<DaemonApi<C, A>>,
        schedule: ReconcileSchedule,
    ) -> Result<Self, DaemonRuntimeError> {
        let websocket = bind_daemon(platform, address, options, Arc::clone(&api))?;
        Ok(Self {
            websocket,
            api,
            schedule,
        })
    }

    /// Return the concrete local address selected by the listener.
    pub fn local_addr(&self) -> SocketAddr {
        self.websocket.local_addr()
    }

    /// Return a cooperative handle that stops serving and wakes the scheduler.
    pub fn stop_handle(&self) -> StopHandle {
        self.websocket.stop_handle()
    }

    /// Reconcile once before serving, then run periodic fail-closed convergence.
    pub fn serve(&mut self) -> Result<(), DaemonRuntimeError> {
        self.api
            .reconcile_once()
            .map_err(DaemonRuntimeError::Reconciliation)?;

        let state = Arc::new((Mutex::new(SchedulerState::default()), Condvar::new()));
        let scheduler = spawn_scheduler(
            Arc::clone(&self.api),
            self.schedule,
            Arc::clone(&state),
            self.websocket.stop_handle(),
        )?;
        let transport_result = self.websocket.serve();
        request_scheduler_stop(&state);
        if scheduler.join().is_err() {
            return Err(DaemonRuntimeError::SchedulerPanicked);
        }
        let failure = lock_state(&state).failure;
        if let Some(error) = failure {
            return Err(DaemonRuntimeError::Reconciliation(error));
        }
        transport_result.map_err(DaemonRuntimeError::Transport)
    }
}

fn spawn_scheduler<C, A>(
    api: Arc<DaemonApi<C, A>>,
    schedule: ReconcileSchedule,
    state: Arc<(Mutex<SchedulerState>, Condvar)>,
    server_stop: StopHandle,
) -> Result<JoinHandle<()>, DaemonRuntimeError>
where
    C: ChiefControlPlane + Send + 'static,
    A: SessionAuthorizer + Send + Sync + 'static,
{
    thread::Builder::new()
        .name("chief-reconcile".to_string())
        .spawn(move || {
            let panic_state = Arc::clone(&state);
            let panic_stop = server_stop.clone();
            let result = panic::catch_unwind(AssertUnwindSafe(|| {
                scheduler_loop(api, schedule, state, server_stop);
            }));
            if let Err(payload) = result {
                request_scheduler_stop(&panic_state);
                panic_stop.stop();
                panic::resume_unwind(payload);
            }
        })
        .map_err(|_| DaemonRuntimeError::SchedulerUnavailable)
}

fn scheduler_loop<C, A>(
    api: Arc<DaemonApi<C, A>>,
    schedule: ReconcileSchedule,
    state: Arc<(Mutex<SchedulerState>, Condvar)>,
    server_stop: StopHandle,
) where
    C: ChiefControlPlane,
    A: SessionAuthorizer,
{
    let (lock, wake) = &*state;
    loop {
        let guard = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let (guard, _) = wake
            .wait_timeout_while(guard, schedule.interval(), |state| !state.stopped)
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if guard.stopped {
            return;
        }
        drop(guard);

        if let Err(error) = api.reconcile_once() {
            let mut guard = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            guard.failure = Some(error);
            guard.stopped = true;
            wake.notify_all();
            server_stop.stop();
            return;
        }
    }
}

fn request_scheduler_stop(state: &Arc<(Mutex<SchedulerState>, Condvar)>) {
    let (lock, wake) = &**state;
    lock.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .stopped = true;
    wake.notify_all();
}

fn lock_state(
    state: &Arc<(Mutex<SchedulerState>, Condvar)>,
) -> std::sync::MutexGuard<'_, SchedulerState> {
    state
        .0
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chief_of_staff_daemon_api::Operation;
    use chief_of_staff_orchestrator_core::HostHealth;
    use chief_of_staff_service_reconciler::ReconcileReport;
    use chief_of_staff_service_registry::{DesiredState, HostName, HostRegistration, LoadedHost};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Instant;

    struct TestAuthorizer;

    impl SessionAuthorizer for TestAuthorizer {
        type Session = ();
        type Error = ();

        fn authenticate(&self, credential: &str) -> Result<Self::Session, Self::Error> {
            if credential == "secret" {
                Ok(())
            } else {
                Err(())
            }
        }

        fn authorize(
            &self,
            _session: &Self::Session,
            _operation: Operation,
        ) -> Result<bool, Self::Error> {
            Ok(true)
        }
    }

    struct CountingControlPlane {
        reconciliations: Arc<AtomicUsize>,
        fail_at: usize,
        panic_at: usize,
    }

    impl CountingControlPlane {
        fn new(reconciliations: Arc<AtomicUsize>, fail_at: usize) -> Self {
            Self {
                reconciliations,
                fail_at,
                panic_at: usize::MAX,
            }
        }

        fn panicking_at(reconciliations: Arc<AtomicUsize>, panic_at: usize) -> Self {
            Self {
                reconciliations,
                fail_at: usize::MAX,
                panic_at,
            }
        }
    }

    impl ChiefControlPlane for CountingControlPlane {
        fn register_host(
            &mut self,
            _registration: HostRegistration,
            _desired_state: DesiredState,
        ) -> Result<LoadedHost, ControlPlaneError> {
            Err(ControlPlaneError::Internal)
        }

        fn list_hosts(&mut self) -> Result<Vec<LoadedHost>, ControlPlaneError> {
            Ok(Vec::new())
        }

        fn set_desired_state(
            &mut self,
            _host_name: &HostName,
            _desired_state: DesiredState,
        ) -> Result<LoadedHost, ControlPlaneError> {
            Err(ControlPlaneError::NotFound)
        }

        fn reconcile_once(&mut self) -> Result<ReconcileReport, ControlPlaneError> {
            let count = self.reconciliations.fetch_add(1, Ordering::SeqCst) + 1;
            assert_ne!(count, self.panic_at, "deliberate scheduler panic");
            if count == self.fail_at {
                Err(ControlPlaneError::Internal)
            } else {
                Ok(ReconcileReport::empty())
            }
        }

        fn health_check(&mut self, _host_name: &HostName) -> Result<HostHealth, ControlPlaneError> {
            Err(ControlPlaneError::NotFound)
        }

        fn deregister_host(&mut self, _host_name: &HostName) -> Result<(), ControlPlaneError> {
            Err(ControlPlaneError::NotFound)
        }
    }

    #[cfg(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    ))]
    type HostPlatform = transport_platform::bsd::KqueueTransportPlatform;
    #[cfg(target_os = "linux")]
    type HostPlatform = transport_platform::linux::EpollTransportPlatform;
    #[cfg(target_os = "windows")]
    type HostPlatform = transport_platform::windows::WindowsTransportPlatform;

    fn host_platform() -> HostPlatform {
        HostPlatform::new().expect("host platform")
    }

    fn runtime(
        reconciliations: Arc<AtomicUsize>,
        fail_at: usize,
        interval: Duration,
    ) -> ChiefDaemonRuntime<HostPlatform, CountingControlPlane, TestAuthorizer> {
        runtime_for(
            CountingControlPlane::new(reconciliations, fail_at),
            interval,
        )
    }

    fn runtime_for(
        control_plane: CountingControlPlane,
        interval: Duration,
    ) -> ChiefDaemonRuntime<HostPlatform, CountingControlPlane, TestAuthorizer> {
        let api = Arc::new(DaemonApi::new(control_plane, TestAuthorizer));
        ChiefDaemonRuntime::bind(
            host_platform(),
            BindAddress::Ip("127.0.0.1:0".parse().unwrap()),
            WebSocketServerOptions::default(),
            api,
            ReconcileSchedule::new(interval).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn zero_schedule_is_rejected() {
        assert!(matches!(
            ReconcileSchedule::new(Duration::ZERO),
            Err(DaemonRuntimeError::InvalidSchedule)
        ));
    }

    #[test]
    fn public_errors_have_stable_payload_blind_diagnostics() {
        let cases = [
            (
                DaemonRuntimeError::InvalidSchedule,
                "chief daemon runtime: invalid reconciliation schedule",
            ),
            (
                DaemonRuntimeError::Reconciliation(ControlPlaneError::Internal),
                "chief daemon runtime: reconciliation failed",
            ),
            (
                DaemonRuntimeError::SchedulerUnavailable,
                "chief daemon runtime: scheduler unavailable",
            ),
            (
                DaemonRuntimeError::SchedulerPanicked,
                "chief daemon runtime: scheduler panicked",
            ),
        ];
        for (error, expected) in cases {
            assert_eq!(error.to_string(), expected);
            assert!(std::error::Error::source(&error).is_none());
        }

        let error = DaemonRuntimeError::from(WebSocketRuntimeError::InvalidOptions);
        assert_eq!(error.to_string(), "chief daemon runtime: transport failed");
        assert!(std::error::Error::source(&error).is_some());
    }

    #[test]
    fn startup_reconciliation_failure_prevents_serving() {
        let reconciliations = Arc::new(AtomicUsize::new(0));
        let mut runtime = runtime(Arc::clone(&reconciliations), 1, Duration::from_millis(5));
        assert!(runtime.local_addr().ip().is_loopback());

        assert!(matches!(
            runtime.serve(),
            Err(DaemonRuntimeError::Reconciliation(
                ControlPlaneError::Internal
            ))
        ));
        assert_eq!(reconciliations.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn background_failure_stops_server_and_surfaces_error() {
        let reconciliations = Arc::new(AtomicUsize::new(0));
        let mut runtime = runtime(Arc::clone(&reconciliations), 2, Duration::from_millis(5));

        assert!(matches!(
            runtime.serve(),
            Err(DaemonRuntimeError::Reconciliation(
                ControlPlaneError::Internal
            ))
        ));
        assert_eq!(reconciliations.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn periodic_ticks_and_external_stop_join_scheduler_promptly() {
        let reconciliations = Arc::new(AtomicUsize::new(0));
        let mut runtime = runtime(
            Arc::clone(&reconciliations),
            usize::MAX,
            Duration::from_millis(5),
        );
        let stop = runtime.stop_handle();
        let server = thread::spawn(move || runtime.serve());
        let deadline = Instant::now() + Duration::from_secs(2);
        while reconciliations.load(Ordering::SeqCst) < 3 && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(1));
        }
        assert!(reconciliations.load(Ordering::SeqCst) >= 3);

        let stopped_at = Instant::now();
        stop.stop();
        assert!(server.join().unwrap().is_ok());
        assert!(stopped_at.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn scheduler_panic_stops_server_and_is_reported() {
        let reconciliations = Arc::new(AtomicUsize::new(0));
        let mut runtime = runtime_for(
            CountingControlPlane::panicking_at(Arc::clone(&reconciliations), 2),
            Duration::from_millis(5),
        );

        assert!(matches!(
            runtime.serve(),
            Err(DaemonRuntimeError::SchedulerPanicked)
        ));
        assert_eq!(reconciliations.load(Ordering::SeqCst), 2);
    }
}
