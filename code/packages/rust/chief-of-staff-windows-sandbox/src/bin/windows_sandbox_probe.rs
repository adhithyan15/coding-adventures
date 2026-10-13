//! `windows-sandbox-probe`: a stand-in agent for the Windows applier's
//! tests (D18S step 8). Each mode tries one thing and prints what
//! happened, `ok` or `error=<code>`, so a test can run it once confined and
//! once as a control and compare.
//!
//! ```text
//!   mode                 tries                                  confined
//!   -------------------  -------------------------------------  ---------
//!   hello                nothing                                hello
//!   argv                 prints its arguments, one per line     same
//!   env                  prints its environment, sorted         given only
//!   read PATH            opens a file for reading               denied
//!   write PATH           creates a file                         denied
//!   connect ADDR         a TCP connection                       denied
//!   spawn EXE            starts a process                       denied
//!   openprocess PID      reads another process's memory         denied
//!   registry             opens HKCU\Software                    denied
//!   dynamic-code         allocates executable memory            denied
//!   user32               loads user32.dll (win32k)              denied
//!   handle VALUE         uses a handle it was not given         fatal
//!   inherited            writes to each handle it was given     ok
//!   token                its token: AppContainer, LPAC, level   1 1 0x1000
//! ```

#[cfg(windows)]
fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let mode = arguments.first().map(String::as_str).unwrap_or("hello");
    let operand = arguments.get(1).map(String::as_str).unwrap_or("");
    let result = match mode {
        "hello" => Ok("hello".to_string()),
        "argv" => Ok(arguments[1..].join("\n")),
        "env" => {
            let mut variables: Vec<String> = std::env::vars()
                .map(|(name, value)| format!("{name}={value}"))
                .collect();
            variables.sort();
            Ok(variables.join("\n"))
        }
        "read" => std::fs::File::open(operand).map(|_| ok()),
        "write" => std::fs::File::create(operand).map(|_| ok()),
        "connect" => operand
            .parse()
            .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))
            .and_then(|address| {
                std::net::TcpStream::connect_timeout(&address, std::time::Duration::from_secs(3))
            })
            .map(|_| ok()),
        "spawn" => std::process::Command::new(operand)
            .args(["/c", "exit 0"])
            .output()
            .map(|_| ok()),
        "openprocess" => windows::open_process(operand),
        "registry" => windows::registry(),
        "dynamic-code" => windows::dynamic_code(),
        "user32" => windows::user32(),
        "handle" => windows::handle(operand),
        "inherited" => windows::inherited(),
        "token" => windows::token(),
        _ => Err(std::io::Error::from(std::io::ErrorKind::InvalidInput)),
    };
    match result {
        Ok(text) => println!("{text}"),
        Err(error) => println!("error={}", error.raw_os_error().unwrap_or(-1)),
    }
}

#[cfg(windows)]
fn ok() -> String {
    "ok".to_string()
}

/// Elsewhere there is no AppContainer to probe.
#[cfg(not(windows))]
fn main() {
    eprintln!("windows-sandbox-probe: Windows only");
    std::process::exit(127)
}

#[cfg(windows)]
mod windows {
    use std::ffi::c_void;
    use std::io::{self, Write};
    use std::os::windows::io::{FromRawHandle, RawHandle};
    use windows_sys::Win32::Foundation::{CloseHandle, GetHandleInformation, HANDLE};
    use windows_sys::Win32::Security::{
        GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation, TokenIntegrityLevel,
        TokenIsAppContainer, TokenIsLessPrivilegedAppContainer, TOKEN_MANDATORY_LABEL, TOKEN_QUERY,
    };
    use windows_sys::Win32::System::LibraryLoader::LoadLibraryW;
    use windows_sys::Win32::System::Memory::{
        VirtualAlloc, VirtualFree, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_EXECUTE_READWRITE,
    };
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER, KEY_READ,
    };
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
        PROCESS_VM_READ,
    };

    fn ok() -> String {
        "ok".to_string()
    }

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub fn open_process(pid: &str) -> io::Result<String> {
        let pid: u32 = pid
            .parse()
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
        // SAFETY: integer arguments; the handle is closed if returned.
        let handle =
            unsafe { OpenProcess(PROCESS_VM_READ | PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if handle == 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: a handle this process owns, closed once.
        unsafe { CloseHandle(handle) };
        Ok(ok())
    }

    pub fn registry() -> io::Result<String> {
        let name = wide("Software");
        let mut key: HKEY = 0;
        // SAFETY: a predefined root key, a terminated name, a key out.
        let result =
            unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, name.as_ptr(), 0, KEY_READ, &mut key) };
        if result != 0 {
            return Err(io::Error::from_raw_os_error(result as i32));
        }
        // SAFETY: the key was opened above.
        unsafe { RegCloseKey(key) };
        Ok(ok())
    }

    pub fn dynamic_code() -> io::Result<String> {
        // SAFETY: a fresh allocation, released at once if made.
        let memory = unsafe {
            VirtualAlloc(
                std::ptr::null(),
                4096,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_EXECUTE_READWRITE,
            )
        };
        if memory.is_null() {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: the allocation made above, released once.
        unsafe { VirtualFree(memory, 0, MEM_RELEASE) };
        Ok(ok())
    }

    pub fn user32() -> io::Result<String> {
        let name = wide("user32.dll");
        // SAFETY: a terminated name; the module is left loaded.
        let module = unsafe { LoadLibraryW(name.as_ptr()) };
        if module == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(ok())
    }

    pub fn handle(value: &str) -> io::Result<String> {
        let value: usize = value
            .parse()
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
        let mut flags = 0u32;
        // SAFETY: GetHandleInformation only reads the handle table; under
        // strict handle checks an invalid value ends the process, which is
        // the outcome the test looks for.
        if unsafe { GetHandleInformation(value as HANDLE, &mut flags) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(ok())
    }

    pub fn inherited() -> io::Result<String> {
        let list = std::env::var("CHIEF_OF_STAFF_HANDLES").unwrap_or_default();
        let mut written = 0;
        for value in list.split(',').filter(|value| !value.is_empty()) {
            let value: usize = value
                .parse()
                .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;
            // SAFETY: the applier passed this handle on and named it here;
            // this process owns it and closes it when the File drops.
            let mut file = unsafe { std::fs::File::from_raw_handle(value as RawHandle) };
            file.write_all(b"inherited\n")?;
            written += 1;
        }
        Ok(format!("wrote {written}"))
    }

    pub fn token() -> io::Result<String> {
        let mut token: HANDLE = 0;
        // SAFETY: this process's own token, queried and closed.
        if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
            return Err(io::Error::last_os_error());
        }
        let query_u32 = |class| {
            let mut value = 0u32;
            let mut written = 0u32;
            // SAFETY: a u32 information class into a u32.
            unsafe {
                GetTokenInformation(
                    token,
                    class,
                    &mut value as *mut _ as *mut c_void,
                    4,
                    &mut written,
                )
            };
            value
        };
        let container = query_u32(TokenIsAppContainer);
        let lpac = query_u32(TokenIsLessPrivilegedAppContainer);
        let mut buffer = vec![0u64; 16];
        let mut written = 0u32;
        // SAFETY: a 128-byte buffer, ample for a TOKEN_MANDATORY_LABEL and
        // its SID; the SID is read only if the call succeeded.
        let level = unsafe {
            if GetTokenInformation(
                token,
                TokenIntegrityLevel,
                buffer.as_mut_ptr() as *mut c_void,
                128,
                &mut written,
            ) == 0
            {
                0
            } else {
                let sid = (*(buffer.as_ptr() as *const TOKEN_MANDATORY_LABEL))
                    .Label
                    .Sid;
                let count = *GetSidSubAuthorityCount(sid);
                *GetSidSubAuthority(sid, u32::from(count) - 1)
            }
        };
        // SAFETY: the token opened above, closed once.
        unsafe { CloseHandle(token) };
        Ok(format!(
            "appcontainer={container} lpac={lpac} integrity={level:#x}"
        ))
    }
}
