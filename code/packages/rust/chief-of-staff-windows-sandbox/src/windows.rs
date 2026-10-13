//! The Windows half: the Win32 calls that make and check the confined
//! process. Every `unsafe` block names what it relies on. Every handle and
//! SID is owned by a value whose `Drop` releases it, so no path out of a
//! function leaks one, and a confined process is never left running
//! unowned: its job is held by the [`ConfinedChild`], or closed (killing
//! it) on every error.

use crate::{
    check_environment, check_plan, command_line, container_name, environment_block,
    strip_verbatim_prefix, ConfinementError, HANDLES_VARIABLE, MITIGATIONS,
};
use capability_os_sandbox::SandboxPlan;
use chief_of_staff_spawn_isolation::VerifiedExecutable;
use std::ffi::c_void;
use std::fs::File;
use std::io;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle, RawHandle};
use std::os::windows::process::ExitStatusExt;
use std::process::ExitStatus;
use windows_sys::Win32::Foundation::{
    LocalFree, SetHandleInformation, HANDLE, HANDLE_FLAG_INHERIT, INVALID_HANDLE_VALUE, PSID,
    WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows_sys::Win32::Security::Authorization::{
    GetSecurityInfo, SetEntriesInAclW, SetSecurityInfo, EXPLICIT_ACCESS_W, GRANT_ACCESS,
    NO_MULTIPLE_TRUSTEE, SE_FILE_OBJECT, TRUSTEE_IS_SID, TRUSTEE_IS_UNKNOWN, TRUSTEE_W,
};
use windows_sys::Win32::Security::Isolation::DeriveAppContainerSidFromAppContainerName;
use windows_sys::Win32::Security::{
    EqualSid, FreeSid, GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation,
    TokenAppContainerSid, TokenCapabilities, TokenIntegrityLevel, TokenIsAppContainer,
    TokenIsLessPrivilegedAppContainer, ACL, DACL_SECURITY_INFORMATION, NO_INHERITANCE,
    SECURITY_CAPABILITIES, TOKEN_APPCONTAINER_INFORMATION, TOKEN_GROUPS, TOKEN_INFORMATION_CLASS,
    TOKEN_MANDATORY_LABEL, TOKEN_QUERY,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, GetFinalPathNameByHandleW, ReOpenFile, FILE_GENERIC_EXECUTE, FILE_GENERIC_READ,
    FILE_GENERIC_WRITE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING, READ_CONTROL, WRITE_DAC,
};
use windows_sys::Win32::System::JobObjects::{
    CreateJobObjectW, IsProcessInJob, JobObjectBasicUIRestrictions,
    JobObjectExtendedLimitInformation, SetInformationJobObject, TerminateJobObject,
    JOBOBJECT_BASIC_UI_RESTRICTIONS, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_ACTIVE_PROCESS, JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOB_OBJECT_UILIMIT_DESKTOP,
    JOB_OBJECT_UILIMIT_DISPLAYSETTINGS, JOB_OBJECT_UILIMIT_EXITWINDOWS,
    JOB_OBJECT_UILIMIT_GLOBALATOMS, JOB_OBJECT_UILIMIT_HANDLES, JOB_OBJECT_UILIMIT_READCLIPBOARD,
    JOB_OBJECT_UILIMIT_SYSTEMPARAMETERS, JOB_OBJECT_UILIMIT_WRITECLIPBOARD,
};
use windows_sys::Win32::System::Pipes::CreatePipe;
use windows_sys::Win32::System::SystemInformation::{
    GetSystemDirectoryW, GetSystemWindowsDirectoryW,
};
use windows_sys::Win32::System::Threading::{
    CreateProcessW, DeleteProcThreadAttributeList, GetExitCodeProcess, GetProcessMitigationPolicy,
    InitializeProcThreadAttributeList, OpenProcessToken, ProcessChildProcessPolicy,
    ProcessDynamicCodePolicy, ProcessExtensionPointDisablePolicy, ProcessFontDisablePolicy,
    ProcessImageLoadPolicy, ProcessSignaturePolicy, ProcessStrictHandleCheckPolicy,
    ProcessSystemCallDisablePolicy, QueryFullProcessImageNameW, ResumeThread,
    UpdateProcThreadAttribute, WaitForSingleObject, CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT,
    DETACHED_PROCESS, EXTENDED_STARTUPINFO_PRESENT, INFINITE, LPPROC_THREAD_ATTRIBUTE_LIST,
    PROCESS_INFORMATION, PROCESS_MITIGATION_POLICY,
    PROC_THREAD_ATTRIBUTE_ALL_APPLICATION_PACKAGES_POLICY,
    PROC_THREAD_ATTRIBUTE_CHILD_PROCESS_POLICY, PROC_THREAD_ATTRIBUTE_HANDLE_LIST,
    PROC_THREAD_ATTRIBUTE_JOB_LIST, PROC_THREAD_ATTRIBUTE_MITIGATION_POLICY,
    PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES, STARTF_USESTDHANDLES, STARTUPINFOEXW,
    STARTUPINFOW,
};
use windows_sys::Win32::System::WindowsProgramming::{
    PROCESS_CREATION_ALL_APPLICATION_PACKAGES_OPT_OUT, PROCESS_CREATION_CHILD_PROCESS_RESTRICTED,
};

/// `SECURITY_MANDATORY_LOW_RID`: the integrity level of an AppContainer.
const LOW_INTEGRITY: u32 = 0x1000;

/// What to run the agent with. The handles in `inherited` are passed on
/// and nothing else is; their values reach it in [`HANDLES_VARIABLE`](crate::HANDLES_VARIABLE).
#[derive(Debug, Default)]
pub struct SpawnRequest {
    /// The agent's arguments, after its own path.
    pub arguments: Vec<String>,
    /// Its environment: each name in the grantable set, with a plain value.
    pub environment: Vec<(String, String)>,
    /// Handles it holds from its first instruction (P2.8b: a broker's
    /// keys).
    pub inherited: Vec<OwnedHandle>,
}

/// One agent, checked and ready to run in its own AppContainer. Prepare one
/// for each launch: preparing re-verifies the binary.
#[derive(Debug)]
pub struct WindowsConfinement {
    /// A duplicate of the verified handle. While it is open, the image can
    /// be neither changed, renamed nor deleted.
    held: File,
    /// The image's path, exactly its final path.
    image: String,
    sid: OwnedSid,
    system_directory: Vec<u16>,
    system_root: String,
}

impl WindowsConfinement {
    /// Check `plan` and `agent`, find or make `agent_id`'s AppContainer,
    /// and let that container read and run the agent's image.
    pub fn prepare_verified(
        plan: &SandboxPlan,
        agent: &VerifiedExecutable,
        agent_id: &str,
    ) -> Result<Self, ConfinementError> {
        check_plan(plan)?;
        agent
            .verify()
            .map_err(|error| ConfinementError::Executable(error.to_string()))?;
        let held = agent
            .descriptor()
            .map_err(|error| ConfinementError::Executable(error.to_string()))?;
        let image = final_path(&held)
            .map_err(|error| ConfinementError::Executable(format!("final path: {error}")))?;
        let given = agent
            .path()
            .to_str()
            .ok_or_else(|| ConfinementError::Executable("the path is not Unicode".to_string()))?;
        // Exactly, case included: the child's image name is compared with
        // it after creation, and that name is the one this path opens.
        if given != image {
            return Err(ConfinementError::Executable(format!(
                "{given}: not canonical (its final path is {image})"
            )));
        }
        let sid = container_sid(&container_name(agent_id))?;
        grant_image_access(&held, &sid)?;
        Ok(Self {
            held,
            image,
            sid,
            system_directory: system_path(GetSystemDirectoryW)?,
            system_root: String::from_utf16_lossy(&strip_nul(system_path(
                GetSystemWindowsDirectoryW,
            )?)),
        })
    }

    /// The image that will run: the agent's canonical path, for the audit
    /// record and tests.
    pub fn image(&self) -> &str {
        &self.image
    }

    /// Run the agent, confined, and confirm it before its first
    /// instruction. On any failure nothing of it has run, and it is ended.
    pub fn spawn(self, request: SpawnRequest) -> Result<ConfinedChild, ConfinementError> {
        check_environment(
            request
                .environment
                .iter()
                .map(|(name, value)| (name.as_str(), value.as_str())),
        )?;
        let mut environment = request.environment.clone();
        environment.push(("SystemRoot".to_string(), self.system_root.clone()));
        if !request.inherited.is_empty() {
            let values: Vec<String> = request
                .inherited
                .iter()
                .map(|handle| (handle.as_raw_handle() as usize).to_string())
                .collect();
            environment.push((HANDLES_VARIABLE.to_string(), values.join(",")));
        }
        let block = environment_block(&environment)?;
        let mut line = command_line(&self.image, &request.arguments)?;
        let application = wide(&self.image);

        let job = new_job()?;
        let (child_stdin, parent_stdin) = pipe()?;
        let (parent_stdout, child_stdout) = pipe()?;
        let stderr = null_device()?;
        let mut listed: Vec<HANDLE> = vec![raw(&child_stdin), raw(&child_stdout), raw(&stderr)];
        listed.extend(request.inherited.iter().map(raw));
        let mut capabilities = SECURITY_CAPABILITIES {
            AppContainerSid: self.sid.0,
            Capabilities: std::ptr::null_mut(),
            CapabilityCount: 0,
            Reserved: 0,
        };
        let mut all_packages: u32 = PROCESS_CREATION_ALL_APPLICATION_PACKAGES_OPT_OUT;
        let mut child_policy: u32 = PROCESS_CREATION_CHILD_PROCESS_RESTRICTED;
        let mut mitigations: u64 = MITIGATIONS;
        let mut jobs: [HANDLE; 1] = [raw(&job)];
        let mut attributes = AttributeList::new(6)?;
        attributes.set(
            PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES,
            &mut capabilities as *mut _ as *mut c_void,
            std::mem::size_of::<SECURITY_CAPABILITIES>(),
        )?;
        attributes.set(
            PROC_THREAD_ATTRIBUTE_ALL_APPLICATION_PACKAGES_POLICY,
            &mut all_packages as *mut _ as *mut c_void,
            std::mem::size_of::<u32>(),
        )?;
        attributes.set(
            PROC_THREAD_ATTRIBUTE_HANDLE_LIST,
            listed.as_mut_ptr() as *mut c_void,
            std::mem::size_of_val(listed.as_slice()),
        )?;
        attributes.set(
            PROC_THREAD_ATTRIBUTE_JOB_LIST,
            jobs.as_mut_ptr() as *mut c_void,
            std::mem::size_of_val(&jobs),
        )?;
        attributes.set(
            PROC_THREAD_ATTRIBUTE_CHILD_PROCESS_POLICY,
            &mut child_policy as *mut _ as *mut c_void,
            std::mem::size_of::<u32>(),
        )?;
        attributes.set(
            PROC_THREAD_ATTRIBUTE_MITIGATION_POLICY,
            &mut mitigations as *mut _ as *mut c_void,
            std::mem::size_of::<u64>(),
        )?;

        // SAFETY: STARTUPINFOEXW is plain data; all-zero is its documented
        // empty value, and the fields used are set below.
        let mut startup: STARTUPINFOEXW = unsafe { std::mem::zeroed() };
        startup.StartupInfo.cb = std::mem::size_of::<STARTUPINFOEXW>() as u32;
        startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
        startup.StartupInfo.hStdInput = raw(&child_stdin);
        startup.StartupInfo.hStdOutput = raw(&child_stdout);
        startup.StartupInfo.hStdError = raw(&stderr);
        startup.lpAttributeList = attributes.as_ptr();
        // Only now, immediately before the creation, are the listed handles
        // made inheritable, so the window in which a std spawn on another
        // thread (which inherits every inheritable handle) could take one
        // is as short as it can be (D18S step 8, residuals).
        for &handle in &listed {
            // SAFETY: each is a handle this process owns, open until it is
            // dropped right after the creation below.
            if unsafe { SetHandleInformation(handle, HANDLE_FLAG_INHERIT, HANDLE_FLAG_INHERIT) }
                == 0
            {
                return Err(create_error("SetHandleInformation"));
            }
        }
        // SAFETY: plain data, filled by CreateProcessW.
        let mut information: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
        // SAFETY: every pointer is to a NUL-terminated UTF-16 buffer or a
        // struct that lives until this call returns; the command line is
        // mutable, as CreateProcessW requires; the attribute list was
        // initialised and every value it points to is alive. Inheritance
        // is TRUE but limited to `listed` by the handle-list attribute.
        let created = unsafe {
            CreateProcessW(
                application.as_ptr(),
                line.as_mut_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                1,
                // DETACHED_PROCESS: no console. A console needs a console
                // host, which Windows starts as a child of the agent: in its
                // job and under its token and policies, all of which forbid
                // it. The standard handles still arrive.
                EXTENDED_STARTUPINFO_PRESENT
                    | CREATE_SUSPENDED
                    | CREATE_UNICODE_ENVIRONMENT
                    | DETACHED_PROCESS,
                block.as_ptr() as *const c_void,
                self.system_directory.as_ptr(),
                // The whole STARTUPINFOEXW: CreateProcessW reads the
                // attribute list beyond the STARTUPINFOW at its head.
                &startup as *const STARTUPINFOEXW as *const STARTUPINFOW,
                &mut information,
            )
        };
        // Read before anything below can overwrite it.
        let failure = (created == 0).then(|| create_error("CreateProcessW"));
        // The child's ends and the passed-on handles are its now, or no
        // one's: this process lets go of them either way. On failure the
        // caller's handles are gone too, so a retry needs new ones.
        drop(attributes);
        drop((child_stdin, child_stdout, stderr, request.inherited));
        if let Some(error) = failure {
            return Err(error);
        }
        // SAFETY: CreateProcessW succeeded, so both handles are open and
        // this process owns them; each is wrapped exactly once.
        let process = unsafe { OwnedHandle::from_raw_handle(information.hProcess as RawHandle) };
        // SAFETY: as above.
        let thread = unsafe { OwnedHandle::from_raw_handle(information.hThread as RawHandle) };

        // S-P4: the process is suspended; nothing of it has run. Any
        // failure below ends it through the job before returning.
        let checked = self.check_launched(&process, &job);
        if let Err(error) = checked {
            // SAFETY: the job handle is open; terminating an empty or
            // already-ended job is harmless.
            unsafe { TerminateJobObject(raw(&job), 1) };
            return Err(error);
        }
        // SAFETY: the thread handle is open; ResumeThread returns the
        // previous suspend count, or u32::MAX on failure.
        let previous = unsafe { ResumeThread(raw(&thread)) };
        if previous != 1 {
            // SAFETY: as above.
            unsafe { TerminateJobObject(raw(&job), 1) };
            return Err(ConfinementError::Verification(format!(
                "ResumeThread returned {previous}, expected 1"
            )));
        }
        drop(thread);
        // The image is mapped now and cannot be written while it runs; the
        // held handle has done its work.
        drop(self.held);
        Ok(ConfinedChild {
            process,
            job,
            id: information.dwProcessId,
            stdin: Some(File::from(parent_stdin)),
            stdout: Some(File::from(parent_stdout)),
        })
    }

    /// Everything S-P4 asks of a created, suspended process.
    fn check_launched(
        &self,
        process: &OwnedHandle,
        job: &OwnedHandle,
    ) -> Result<(), ConfinementError> {
        let refuse = |why: String| ConfinementError::Verification(why);
        let token = process_token(process).map_err(|e| refuse(format!("token: {e}")))?;

        if token_u32(&token, TokenIsAppContainer).map_err(|e| refuse(e.to_string()))? != 1 {
            return Err(refuse("the token is not an AppContainer's".into()));
        }
        if token_u32(&token, TokenIsLessPrivilegedAppContainer)
            .map_err(|e| refuse(e.to_string()))?
            != 1
        {
            return Err(refuse(
                "the token is not a less privileged AppContainer's".into(),
            ));
        }
        let container =
            token_buffer(&token, TokenAppContainerSid).map_err(|e| refuse(e.to_string()))?;
        // SAFETY: the buffer holds a TOKEN_APPCONTAINER_INFORMATION, whose
        // SID pointer points into the same buffer; both SIDs are valid.
        let same = unsafe {
            let information = &*(container.as_ptr() as *const TOKEN_APPCONTAINER_INFORMATION);
            !information.TokenAppContainer.is_null()
                && EqualSid(information.TokenAppContainer, self.sid.0) != 0
        };
        if !same {
            return Err(refuse(
                "the token's AppContainer is not this agent's".into(),
            ));
        }
        let groups = token_buffer(&token, TokenCapabilities).map_err(|e| refuse(e.to_string()))?;
        // SAFETY: the buffer holds a TOKEN_GROUPS; GroupCount is its first
        // field.
        let count = unsafe { (*(groups.as_ptr() as *const TOKEN_GROUPS)).GroupCount };
        if count != 0 {
            return Err(refuse(format!("the token has {count} capabilities")));
        }
        let label = token_buffer(&token, TokenIntegrityLevel).map_err(|e| refuse(e.to_string()))?;
        // SAFETY: the buffer holds a TOKEN_MANDATORY_LABEL whose SID points
        // into it; GetSidSubAuthorityCount and GetSidSubAuthority read that
        // valid SID, and the index is the last sub-authority.
        let level = unsafe {
            let sid = (*(label.as_ptr() as *const TOKEN_MANDATORY_LABEL))
                .Label
                .Sid;
            let count = *GetSidSubAuthorityCount(sid);
            if count == 0 {
                0
            } else {
                *GetSidSubAuthority(sid, u32::from(count) - 1)
            }
        };
        if level != LOW_INTEGRITY {
            return Err(refuse(format!("integrity level {level:#x}, expected Low")));
        }

        let mut in_job = 0;
        // SAFETY: both handles are open; the result is written to a BOOL.
        if unsafe { IsProcessInJob(raw(process), raw(job), &mut in_job) } == 0 || in_job == 0 {
            return Err(refuse("the process is not in its job".into()));
        }

        for (policy, mask, name) in [
            (ProcessDynamicCodePolicy, 0b1, "dynamic code"),
            (ProcessStrictHandleCheckPolicy, 0b1, "strict handle checks"),
            (ProcessSystemCallDisablePolicy, 0b1, "win32k system calls"),
            (ProcessExtensionPointDisablePolicy, 0b1, "extension points"),
            (ProcessSignaturePolicy, 0b1, "Microsoft-signed only"),
            (ProcessFontDisablePolicy, 0b1, "non-system fonts"),
            (ProcessImageLoadPolicy, 0b111, "image loads"),
            (ProcessChildProcessPolicy, 0b1, "child processes"),
        ] {
            let flags = mitigation(process, policy).map_err(|e| refuse(format!("{name}: {e}")))?;
            if flags & mask != mask {
                return Err(refuse(format!("{name}: policy flags {flags:#x}")));
            }
        }

        // The name the child's image was opened by, and the name the held
        // file has now, must both still be the verified path. The held
        // handle keeps the file from being changed, renamed or deleted; these
        // two checks confirm the path still led to it at the creation.
        let image = image_name(process).map_err(|e| refuse(format!("image name: {e}")))?;
        if image != self.image {
            return Err(refuse(format!(
                "the image is {image}, not the verified {}",
                self.image
            )));
        }
        let held = final_path(&self.held).map_err(|e| refuse(format!("final path: {e}")))?;
        if held != self.image {
            return Err(refuse(format!(
                "the verified file is now {held}, not {}",
                self.image
            )));
        }
        Ok(())
    }
}

/// A confined agent. Dropping it kills the agent: its job is closed, and
/// the job kills its process when its last handle closes.
#[derive(Debug)]
pub struct ConfinedChild {
    process: OwnedHandle,
    job: OwnedHandle,
    id: u32,
    /// The agent's stdin.
    pub stdin: Option<File>,
    /// The agent's stdout.
    pub stdout: Option<File>,
}

impl ConfinedChild {
    /// The agent's process id.
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Close its stdin and wait for it to exit.
    pub fn wait(&mut self) -> io::Result<ExitStatus> {
        self.stdin = None;
        // SAFETY: the process handle is open.
        if unsafe { WaitForSingleObject(raw(&self.process), INFINITE) } != WAIT_OBJECT_0 {
            return Err(io::Error::last_os_error());
        }
        self.exit_status()
    }

    /// Its exit status, if it has exited.
    pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        // SAFETY: the process handle is open.
        match unsafe { WaitForSingleObject(raw(&self.process), 0) } {
            WAIT_OBJECT_0 => self.exit_status().map(Some),
            WAIT_TIMEOUT => Ok(None),
            _ => Err(io::Error::last_os_error()),
        }
    }

    /// End it, and anything in its job.
    pub fn kill(&mut self) -> io::Result<()> {
        // SAFETY: the job handle is open.
        if unsafe { TerminateJobObject(raw(&self.job), 1) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    fn exit_status(&self) -> io::Result<ExitStatus> {
        let mut code = 0u32;
        // SAFETY: the process handle is open; the code is written to a u32.
        if unsafe { GetExitCodeProcess(raw(&self.process), &mut code) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(ExitStatus::from_raw(code))
    }
}

// ---------------------------------------------------------------------------
// The pieces
// ---------------------------------------------------------------------------

/// A SID allocated by the AppContainer functions, freed with `FreeSid`.
#[derive(Debug)]
struct OwnedSid(PSID);

impl Drop for OwnedSid {
    fn drop(&mut self) {
        // SAFETY: the SID came from
        // DeriveAppContainerSidFromAppContainerName, whose documented
        // release is FreeSid, and is freed once.
        unsafe { FreeSid(self.0) };
    }
}

/// The AppContainer SID for `name`, derived, with no profile registered.
///
/// A registered profile would give the agent a storage folder under
/// `%LOCALAPPDATA%\Packages` and a registry key, both writable by its SID
/// and both kept across runs: persistent state S-I1 does not allow. The
/// kernel needs only the SID, and the SID is a function of the name alone,
/// so the agent's principal is the same on every launch without one.
fn container_sid(name: &str) -> Result<OwnedSid, ConfinementError> {
    let name = wide(name);
    let mut sid: PSID = std::ptr::null_mut();
    // SAFETY: a NUL-terminated name; the SID is written to `sid` and freed
    // with FreeSid by OwnedSid.
    let derived = unsafe { DeriveAppContainerSidFromAppContainerName(name.as_ptr(), &mut sid) };
    if derived < 0 || sid.is_null() {
        return Err(ConfinementError::Container(format!(
            "DeriveAppContainerSidFromAppContainerName: {derived:#x}"
        )));
    }
    Ok(OwnedSid(sid))
}

/// One ACE on the image itself, through a handle reopened from the held
/// one (never by path): `sid` may read and execute it.
///
/// Reading the access list, adding to it and writing it back is not
/// atomic, so two agents prepared at once from one image could each write
/// a list without the other's entry, and that agent could then not read
/// its own image. The read-modify-write is serialized across this process.
fn grant_image_access(held: &File, sid: &OwnedSid) -> Result<(), ConfinementError> {
    static ACL_UPDATE: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _serialized = ACL_UPDATE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let refuse =
        |why: &str| ConfinementError::ImageAccess(format!("{why}: {}", io::Error::last_os_error()));
    // SAFETY: the held handle is open; ReOpenFile returns a new handle to
    // the same file object, or INVALID_HANDLE_VALUE.
    let reopened = unsafe { ReOpenFile(raw(held), READ_CONTROL | WRITE_DAC, FILE_SHARE_READ, 0) };
    if reopened == INVALID_HANDLE_VALUE {
        return Err(refuse("ReOpenFile"));
    }
    // SAFETY: a fresh handle this process owns; wrapped once.
    let reopened = unsafe { OwnedHandle::from_raw_handle(reopened as RawHandle) };
    let mut dacl: *mut ACL = std::ptr::null_mut();
    let mut descriptor: *mut c_void = std::ptr::null_mut();
    // SAFETY: the handle has READ_CONTROL; the DACL pointer points into
    // the descriptor, which is freed with LocalFree below.
    let read = unsafe {
        GetSecurityInfo(
            raw(&reopened),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut dacl,
            std::ptr::null_mut(),
            &mut descriptor,
        )
    };
    if read != 0 {
        return Err(ConfinementError::ImageAccess(format!(
            "GetSecurityInfo: {read}"
        )));
    }
    let entry = EXPLICIT_ACCESS_W {
        grfAccessPermissions: FILE_GENERIC_READ | FILE_GENERIC_EXECUTE,
        grfAccessMode: GRANT_ACCESS,
        grfInheritance: NO_INHERITANCE,
        Trustee: TRUSTEE_W {
            pMultipleTrustee: std::ptr::null_mut(),
            MultipleTrusteeOperation: NO_MULTIPLE_TRUSTEE,
            TrusteeForm: TRUSTEE_IS_SID,
            TrusteeType: TRUSTEE_IS_UNKNOWN,
            ptstrName: sid.0 as *mut u16,
        },
    };
    let mut updated: *mut ACL = std::ptr::null_mut();
    // SAFETY: one entry, a valid old DACL (or null), the result written to
    // `updated`, which is freed with LocalFree.
    let merged = unsafe { SetEntriesInAclW(1, &entry, dacl, &mut updated) };
    let result = if merged != 0 {
        Err(ConfinementError::ImageAccess(format!(
            "SetEntriesInAclW: {merged}"
        )))
    } else {
        // SAFETY: the handle has WRITE_DAC; the new DACL is valid.
        let set = unsafe {
            SetSecurityInfo(
                raw(&reopened),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                updated,
                std::ptr::null(),
            )
        };
        if set != 0 {
            Err(ConfinementError::ImageAccess(format!(
                "SetSecurityInfo: {set}"
            )))
        } else {
            Ok(())
        }
    };
    // SAFETY: both were allocated by the calls above with LocalAlloc, and
    // each is freed once (null is allowed).
    unsafe {
        LocalFree(updated as _);
        LocalFree(descriptor as _);
    }
    result
}

/// A new job: kill on close, one active process, die on an unhandled
/// exception, every UI restriction.
fn new_job() -> Result<OwnedHandle, ConfinementError> {
    // SAFETY: no security attributes and no name: an anonymous job whose
    // only handle is this one.
    let job = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
    if job == 0 {
        return Err(create_error("CreateJobObjectW"));
    }
    // SAFETY: a fresh handle this process owns; wrapped once.
    let job = unsafe { OwnedHandle::from_raw_handle(job as RawHandle) };
    // SAFETY: plain data; zero is "no limit" for every field not set.
    let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
    limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
        | JOB_OBJECT_LIMIT_ACTIVE_PROCESS
        | JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION;
    limits.BasicLimitInformation.ActiveProcessLimit = 1;
    // SAFETY: the job handle is open; the struct and its size match the
    // information class.
    if unsafe {
        SetInformationJobObject(
            raw(&job),
            JobObjectExtendedLimitInformation,
            &limits as *const _ as *const c_void,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
    } == 0
    {
        return Err(create_error("SetInformationJobObject(limits)"));
    }
    let ui = JOBOBJECT_BASIC_UI_RESTRICTIONS {
        UIRestrictionsClass: JOB_OBJECT_UILIMIT_DESKTOP
            | JOB_OBJECT_UILIMIT_DISPLAYSETTINGS
            | JOB_OBJECT_UILIMIT_EXITWINDOWS
            | JOB_OBJECT_UILIMIT_GLOBALATOMS
            | JOB_OBJECT_UILIMIT_HANDLES
            | JOB_OBJECT_UILIMIT_READCLIPBOARD
            | JOB_OBJECT_UILIMIT_SYSTEMPARAMETERS
            | JOB_OBJECT_UILIMIT_WRITECLIPBOARD,
    };
    // SAFETY: as above.
    if unsafe {
        SetInformationJobObject(
            raw(&job),
            JobObjectBasicUIRestrictions,
            &ui as *const _ as *const c_void,
            std::mem::size_of::<JOBOBJECT_BASIC_UI_RESTRICTIONS>() as u32,
        )
    } == 0
    {
        return Err(create_error("SetInformationJobObject(UI)"));
    }
    Ok(job)
}

/// An anonymous pipe, both ends not inheritable: `(read, write)`.
fn pipe() -> Result<(OwnedHandle, OwnedHandle), ConfinementError> {
    let mut read: HANDLE = 0;
    let mut write: HANDLE = 0;
    // SAFETY: both handles are written by CreatePipe; no security
    // attributes, so neither is inheritable.
    if unsafe { CreatePipe(&mut read, &mut write, std::ptr::null(), 0) } == 0 {
        return Err(create_error("CreatePipe"));
    }
    // SAFETY: fresh handles this process owns; each wrapped once.
    unsafe {
        Ok((
            OwnedHandle::from_raw_handle(read as RawHandle),
            OwnedHandle::from_raw_handle(write as RawHandle),
        ))
    }
}

/// The NUL device, for the agent's stderr.
fn null_device() -> Result<OwnedHandle, ConfinementError> {
    let name = wide("NUL");
    // SAFETY: a NUL-terminated name; no security attributes (not
    // inheritable until marked); no template.
    let handle = unsafe {
        CreateFileW(
            name.as_ptr(),
            FILE_GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            0,
            0,
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(create_error("CreateFileW(NUL)"));
    }
    // SAFETY: a fresh handle this process owns; wrapped once.
    Ok(unsafe { OwnedHandle::from_raw_handle(handle as RawHandle) })
}

/// A process-thread attribute list, freed with
/// `DeleteProcThreadAttributeList` on drop.
struct AttributeList {
    /// Backing storage, `u64` for alignment.
    storage: Vec<u64>,
}

impl AttributeList {
    fn new(count: u32) -> Result<Self, ConfinementError> {
        let mut size = 0usize;
        // SAFETY: the documented size query: a null list returns FALSE and
        // writes the size needed.
        unsafe { InitializeProcThreadAttributeList(std::ptr::null_mut(), count, 0, &mut size) };
        if size == 0 {
            return Err(create_error("InitializeProcThreadAttributeList(size)"));
        }
        let mut list = Self {
            storage: vec![0u64; size.div_ceil(8)],
        };
        // SAFETY: the storage holds at least `size` bytes, aligned to 8.
        if unsafe { InitializeProcThreadAttributeList(list.as_ptr(), count, 0, &mut size) } == 0 {
            // Not initialised, so Drop must not delete it.
            list.storage = Vec::new();
            return Err(create_error("InitializeProcThreadAttributeList"));
        }
        Ok(list)
    }

    fn as_ptr(&mut self) -> LPPROC_THREAD_ATTRIBUTE_LIST {
        self.storage.as_mut_ptr() as LPPROC_THREAD_ATTRIBUTE_LIST
    }

    /// Point `attribute` at `size` bytes at `value`, which must stay alive
    /// and unmoved until the process is created.
    fn set(
        &mut self,
        attribute: u32,
        value: *mut c_void,
        size: usize,
    ) -> Result<(), ConfinementError> {
        // SAFETY: the list is initialised; `value` points to `size` live
        // bytes (the caller keeps them until CreateProcessW returns).
        if unsafe {
            UpdateProcThreadAttribute(
                self.as_ptr(),
                0,
                attribute as usize,
                value,
                size,
                std::ptr::null_mut(),
                std::ptr::null(),
            )
        } == 0
        {
            return Err(create_error("UpdateProcThreadAttribute"));
        }
        Ok(())
    }
}

impl Drop for AttributeList {
    fn drop(&mut self) {
        if !self.storage.is_empty() {
            // SAFETY: the list was initialised and is deleted once.
            unsafe { DeleteProcThreadAttributeList(self.as_ptr()) };
        }
    }
}

fn process_token(process: &OwnedHandle) -> io::Result<OwnedHandle> {
    let mut token: HANDLE = 0;
    // SAFETY: the process handle is open; the token is written to `token`.
    if unsafe { OpenProcessToken(raw(process), TOKEN_QUERY, &mut token) } == 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: a fresh handle this process owns; wrapped once.
    Ok(unsafe { OwnedHandle::from_raw_handle(token as RawHandle) })
}

fn token_u32(token: &OwnedHandle, class: TOKEN_INFORMATION_CLASS) -> io::Result<u32> {
    let mut value = 0u32;
    let mut written = 0u32;
    // SAFETY: the token is open with TOKEN_QUERY; the value is a u32.
    if unsafe {
        GetTokenInformation(
            raw(token),
            class,
            &mut value as *mut _ as *mut c_void,
            4,
            &mut written,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(value)
}

/// A variable-size token information class, in a `u64`-aligned buffer.
fn token_buffer(token: &OwnedHandle, class: TOKEN_INFORMATION_CLASS) -> io::Result<Vec<u64>> {
    let mut needed = 0u32;
    // SAFETY: the documented size query: a null buffer of length 0.
    unsafe { GetTokenInformation(raw(token), class, std::ptr::null_mut(), 0, &mut needed) };
    if needed == 0 {
        return Err(io::Error::last_os_error());
    }
    let mut buffer = vec![0u64; (needed as usize).div_ceil(8)];
    // SAFETY: the buffer holds at least `needed` bytes.
    if unsafe {
        GetTokenInformation(
            raw(token),
            class,
            buffer.as_mut_ptr() as *mut c_void,
            needed,
            &mut needed,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(buffer)
}

/// The flags word of one mitigation policy of `process`.
fn mitigation(process: &OwnedHandle, policy: PROCESS_MITIGATION_POLICY) -> io::Result<u32> {
    let mut flags = 0u32;
    // SAFETY: every policy queried is a 32-bit flags union.
    if unsafe {
        GetProcessMitigationPolicy(raw(process), policy, &mut flags as *mut _ as *mut c_void, 4)
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(flags)
}

/// `QueryFullProcessImageNameW`, in Win32 form (`C:\...`).
fn image_name(process: &OwnedHandle) -> io::Result<String> {
    let mut buffer = vec![0u16; 32_768];
    let mut length = buffer.len() as u32;
    // SAFETY: the buffer holds `length` UTF-16 units.
    if unsafe { QueryFullProcessImageNameW(raw(process), 0, buffer.as_mut_ptr(), &mut length) } == 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(String::from_utf16_lossy(&buffer[..length as usize]))
}

/// The final path of an open file, without `\\?\`.
fn final_path(file: &File) -> io::Result<String> {
    let mut buffer = vec![0u16; 32_768];
    // SAFETY: the handle is open; the buffer holds `len` units; flags 0 is
    // FILE_NAME_NORMALIZED | VOLUME_NAME_DOS.
    let length = unsafe {
        GetFinalPathNameByHandleW(raw(file), buffer.as_mut_ptr(), buffer.len() as u32, 0)
    } as usize;
    if length == 0 || length >= buffer.len() {
        return Err(io::Error::last_os_error());
    }
    Ok(strip_verbatim_prefix(&String::from_utf16_lossy(&buffer[..length])).to_string())
}

/// A system directory, NUL-terminated.
fn system_path(
    query: unsafe extern "system" fn(*mut u16, u32) -> u32,
) -> Result<Vec<u16>, ConfinementError> {
    let mut buffer = vec![0u16; 1024];
    // SAFETY: the buffer holds `len` units; the call writes a terminated
    // path and returns its length.
    let length = unsafe { query(buffer.as_mut_ptr(), buffer.len() as u32) } as usize;
    if length == 0 || length >= buffer.len() {
        return Err(create_error("system directory"));
    }
    buffer.truncate(length + 1);
    Ok(buffer)
}

fn strip_nul(mut text: Vec<u16>) -> Vec<u16> {
    while text.last() == Some(&0) {
        text.pop();
    }
    text
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn raw(handle: &impl AsRawHandle) -> HANDLE {
    handle.as_raw_handle() as HANDLE
}

fn create_error(call: &str) -> ConfinementError {
    ConfinementError::Create(format!("{call}: {}", io::Error::last_os_error()))
}
