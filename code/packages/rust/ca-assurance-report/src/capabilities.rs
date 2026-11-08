// Generated from required_capabilities.json. Do not edit.
// Package: rust/ca-assurance-report
// Declared capabilities:
//   - fs:read:*
//   - fs:list:*
//   - fs:write:*
//   - stdout:write:*
//   - env:read:*
//   - proc:signal:*

#![deny(unsafe_code)]

/// Package capability authority token.
///
/// Exposes only the capabilities explicitly authorized in required_capabilities.json.
#[derive(Debug)]
pub struct PackageCapabilities {
    pub(crate) cap_fs_read_0: FsReadAllCap,
    pub(crate) cap_fs_list_1: FsListAllCap,
    pub(crate) cap_fs_write_2: FsWriteAllCap,
    pub(crate) cap_stdout_write_3: StdoutWriteAllCap,
    pub(crate) cap_env_read_4: EnvReadAllCap,
    pub(crate) cap_proc_signal_5: ProcSignalAllCap,
}

/// Authority witness for `fs:read:*`.
#[derive(Debug, Clone, Copy)]
pub struct FsReadAllCap;

/// Authority witness for `fs:list:*`.
#[derive(Debug, Clone, Copy)]
pub struct FsListAllCap;

/// Authority witness for `fs:write:*`.
#[derive(Debug, Clone, Copy)]
pub struct FsWriteAllCap;

/// Authority witness for `stdout:write:*`.
#[derive(Debug, Clone, Copy)]
pub struct StdoutWriteAllCap;

/// Authority witness for `env:read:*`.
#[derive(Debug, Clone, Copy)]
pub struct EnvReadAllCap;

/// Authority witness for `proc:signal:*`.
#[derive(Debug, Clone, Copy)]
pub struct ProcSignalAllCap;

