//! Rendering a [`RuntimePolicy`] as a classic-BPF seccomp program.
//!
//! seccomp filters are tiny programs run by the kernel on every syscall. The
//! input is `struct seccomp_data`:
//!
//! ```text
//!   offset 0   nr                 the syscall number
//!   offset 4   arch               AUDIT_ARCH_* (which ABI the number is for)
//!   offset 8   instruction_pointer
//!   offset 16  args[0..6]         six 64-bit integer arguments
//! ```
//!
//! The program this module emits has a fixed shape:
//!
//! ```text
//!   LD  arch ; JEQ expected ? continue : KILL     <- wrong ABI: kill
//!   LD  nr
//!   block(write):  JEQ nr_write ? (LD arg0; JEQ 1 -> ALLOW; JEQ 2 -> ALLOW; KILL)
//!   block(brk):    JEQ nr_brk   ? ALLOW
//!   block(mmap):   JEQ nr_mmap  ? (LD arg2; JSET PROT_EXEC -> KILL; ALLOW)
//!   ...
//!   KILL_PROCESS                                   <- default: deny
//! ```
//!
//! Each block is `JEQ nr` that falls through to the next block on mismatch
//! (its false-jump skips the block's body). Predicates only run *after* the
//! number matched and always end in a `RET`, so the accumulator still holds
//! `nr` for the next block.
//!
//! The default action is `SECCOMP_RET_KILL_PROCESS`, not `ERRNO`: a library
//! handed `EPERM` can swallow it and carry on, hiding the attempt; a kill is
//! loud (SIGSYS) and cannot be ignored. x32-ABI numbers (bit 30 set) never
//! equal an allowed number, so they fall to the default as well.

use crate::policy::{Pred, RuntimePolicy};
use std::collections::BTreeMap;

// Classic BPF opcode pieces (linux/bpf_common.h).
const BPF_LD: u16 = 0x00;
const BPF_JMP: u16 = 0x05;
const BPF_RET: u16 = 0x06;
const BPF_W: u16 = 0x00;
const BPF_ABS: u16 = 0x20;
const BPF_JEQ: u16 = 0x10;
const BPF_JSET: u16 = 0x40;
const BPF_K: u16 = 0x00;

/// `SECCOMP_RET_KILL_PROCESS` (kernel 4.14+).
pub const RET_KILL_PROCESS: u32 = 0x8000_0000;
/// `SECCOMP_RET_ALLOW`.
pub const RET_ALLOW: u32 = 0x7fff_0000;

const OFFSET_NR: u32 = 0;
const OFFSET_ARCH: u32 = 4;
const OFFSET_ARGS: u32 = 16;
const PROT_EXEC: u32 = 4;

/// One classic-BPF instruction (layout-compatible with `struct sock_filter`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Insn {
    /// Opcode.
    pub code: u16,
    /// Jump-if-true offset.
    pub jt: u8,
    /// Jump-if-false offset.
    pub jf: u8,
    /// Immediate operand.
    pub k: u32,
}

const fn stmt(code: u16, k: u32) -> Insn {
    Insn {
        code,
        jt: 0,
        jf: 0,
        k,
    }
}

const fn jump(code: u16, k: u32, jt: u8, jf: u8) -> Insn {
    Insn { code, jt, jf, k }
}

/// `AUDIT_ARCH_*` of the architecture this build targets.
#[cfg(target_arch = "x86_64")]
pub const AUDIT_ARCH: u32 = 0xc000_003e;
/// `AUDIT_ARCH_*` of the architecture this build targets.
#[cfg(target_arch = "aarch64")]
pub const AUDIT_ARCH: u32 = 0xc000_00b7;

/// The syscall number of `name` on this architecture, if the gate knows it.
pub fn syscall_number(name: &str) -> Option<u32> {
    macro_rules! sys {
        ($($n:ident => $c:ident),* $(,)?) => {
            match name { $( stringify!($n) => Some(libc::$c as u32), )* _ => None }
        };
    }
    let common = sys! {
        read => SYS_read, write => SYS_write, close => SYS_close, fstat => SYS_fstat,
        lseek => SYS_lseek, mmap => SYS_mmap, mprotect => SYS_mprotect, munmap => SYS_munmap,
        brk => SYS_brk, rt_sigprocmask => SYS_rt_sigprocmask, rt_sigreturn => SYS_rt_sigreturn,
        pread64 => SYS_pread64, pwrite64 => SYS_pwrite64, readv => SYS_readv,
        writev => SYS_writev, mremap => SYS_mremap, madvise => SYS_madvise,
        dup => SYS_dup, dup3 => SYS_dup3, nanosleep => SYS_nanosleep, getpid => SYS_getpid,
        socket => SYS_socket, connect => SYS_connect, accept => SYS_accept,
        sendto => SYS_sendto, recvfrom => SYS_recvfrom, sendmsg => SYS_sendmsg,
        recvmsg => SYS_recvmsg, shutdown => SYS_shutdown, bind => SYS_bind,
        listen => SYS_listen, getsockname => SYS_getsockname, getpeername => SYS_getpeername,
        setsockopt => SYS_setsockopt, getsockopt => SYS_getsockopt, clone => SYS_clone,
        execve => SYS_execve, exit => SYS_exit, wait4 => SYS_wait4, kill => SYS_kill,
        fcntl => SYS_fcntl, fsync => SYS_fsync, fdatasync => SYS_fdatasync,
        ftruncate => SYS_ftruncate, getdents64 => SYS_getdents64, readlinkat => SYS_readlinkat,
        mkdirat => SYS_mkdirat, unlinkat => SYS_unlinkat, openat => SYS_openat,
        newfstatat => SYS_newfstatat, faccessat => SYS_faccessat, ppoll => SYS_ppoll,
        epoll_create1 => SYS_epoll_create1, epoll_ctl => SYS_epoll_ctl,
        epoll_pwait => SYS_epoll_pwait, accept4 => SYS_accept4, pipe2 => SYS_pipe2,
        preadv => SYS_preadv, pwritev => SYS_pwritev, execveat => SYS_execveat,
        getrandom => SYS_getrandom, statx => SYS_statx, clone3 => SYS_clone3,
        faccessat2 => SYS_faccessat2, rseq => SYS_rseq, exit_group => SYS_exit_group,
        tgkill => SYS_tgkill, tkill => SYS_tkill, rt_sigqueueinfo => SYS_rt_sigqueueinfo,
        waitid => SYS_waitid, clock_gettime => SYS_clock_gettime,
        clock_getres => SYS_clock_getres, clock_nanosleep => SYS_clock_nanosleep,
        gettimeofday => SYS_gettimeofday, futex => SYS_futex,
        set_robust_list => SYS_set_robust_list, sched_yield => SYS_sched_yield,
        sigaltstack => SYS_sigaltstack, sched_getaffinity => SYS_sched_getaffinity,
        ptrace => SYS_ptrace, bpf => SYS_bpf, io_uring_setup => SYS_io_uring_setup,
        io_uring_enter => SYS_io_uring_enter, io_uring_register => SYS_io_uring_register,
        mount => SYS_mount, umount2 => SYS_umount2, pivot_root => SYS_pivot_root,
        chroot => SYS_chroot, setns => SYS_setns, unshare => SYS_unshare,
        perf_event_open => SYS_perf_event_open, keyctl => SYS_keyctl,
        add_key => SYS_add_key, request_key => SYS_request_key,
        init_module => SYS_init_module, finit_module => SYS_finit_module,
        kexec_load => SYS_kexec_load, reboot => SYS_reboot, seccomp => SYS_seccomp,
        prctl => SYS_prctl,
    };
    // Landlock numbers are the same on every architecture (kernel UAPI).
    let landlock = match name {
        "landlock_create_ruleset" => Some(444),
        "landlock_add_rule" => Some(445),
        "landlock_restrict_self" => Some(446),
        _ => None,
    };
    common.or(landlock).or_else(|| legacy_number(name))
}

/// Legacy syscalls that exist only on x86_64 (aarch64 has only the `*at`
/// variants). They are still named in the policy so x86_64 hosts block them;
/// on aarch64 they simply have no number.
#[cfg(target_arch = "x86_64")]
fn legacy_number(name: &str) -> Option<u32> {
    macro_rules! sys {
        ($($n:ident => $c:ident),* $(,)?) => {
            match name { $( stringify!($n) => Some(libc::$c as u32), )* _ => None }
        };
    }
    sys! {
        open => SYS_open, stat => SYS_stat, lstat => SYS_lstat, access => SYS_access,
        readlink => SYS_readlink, creat => SYS_creat, mkdir => SYS_mkdir,
        unlink => SYS_unlink, rmdir => SYS_rmdir, getdents => SYS_getdents,
        poll => SYS_poll, epoll_wait => SYS_epoll_wait, pipe => SYS_pipe,
        dup2 => SYS_dup2, vfork => SYS_vfork, fork => SYS_fork, time => SYS_time,
    }
}

#[cfg(not(target_arch = "x86_64"))]
fn legacy_number(_name: &str) -> Option<u32> {
    None
}

/// Build the seccomp program for `policy`.
///
/// Names the architecture has no number for (x86_64-only legacy calls on
/// aarch64) are skipped: they cannot be issued there at all.
pub fn build_program(policy: &RuntimePolicy) -> Vec<Insn> {
    let mut program = vec![
        stmt(BPF_LD | BPF_W | BPF_ABS, OFFSET_ARCH),
        jump(BPF_JMP | BPF_JEQ | BPF_K, AUDIT_ARCH, 1, 0),
        stmt(BPF_RET | BPF_K, RET_KILL_PROCESS),
        stmt(BPF_LD | BPF_W | BPF_ABS, OFFSET_NR),
    ];
    // Deterministic order (by number) so the program is reproducible.
    let mut by_number: BTreeMap<u32, Pred> = BTreeMap::new();
    for (name, pred) in &policy.allow {
        if let Some(nr) = syscall_number(name) {
            by_number.insert(nr, *pred);
        }
    }
    for (nr, pred) in by_number {
        match pred {
            Pred::Any => {
                program.push(jump(BPF_JMP | BPF_JEQ | BPF_K, nr, 0, 1));
                program.push(stmt(BPF_RET | BPF_K, RET_ALLOW));
            }
            Pred::ArgIn(index, values) => {
                // body: LD arg[index]; one JEQ per value -> ALLOW; KILL; ALLOW
                let body_len = 1 + values.len() + 2;
                program.push(jump(BPF_JMP | BPF_JEQ | BPF_K, nr, 0, body_len as u8));
                program.push(stmt(
                    BPF_LD | BPF_W | BPF_ABS,
                    OFFSET_ARGS + 8 * index as u32,
                ));
                for (i, value) in values.iter().enumerate() {
                    // jump to the trailing ALLOW: remaining JEQs + the KILL.
                    let to_allow = (values.len() - i) as u8;
                    program.push(jump(BPF_JMP | BPF_JEQ | BPF_K, *value, to_allow, 0));
                }
                program.push(stmt(BPF_RET | BPF_K, RET_KILL_PROCESS));
                program.push(stmt(BPF_RET | BPF_K, RET_ALLOW));
            }
            Pred::NoExecProt(index) => {
                // body: LD arg[index]; JSET PROT_EXEC -> KILL; ALLOW; KILL
                program.push(jump(BPF_JMP | BPF_JEQ | BPF_K, nr, 0, 4));
                program.push(stmt(
                    BPF_LD | BPF_W | BPF_ABS,
                    OFFSET_ARGS + 8 * index as u32,
                ));
                program.push(jump(BPF_JMP | BPF_JSET | BPF_K, PROT_EXEC, 1, 0));
                program.push(stmt(BPF_RET | BPF_K, RET_ALLOW));
                program.push(stmt(BPF_RET | BPF_K, RET_KILL_PROCESS));
            }
        }
    }
    program.push(stmt(BPF_RET | BPF_K, RET_KILL_PROCESS));
    program
}

/// The `seccomp_data` a syscall presents to the filter.
#[derive(Debug, Clone, Copy)]
pub struct SeccompData {
    /// Syscall number.
    pub nr: u32,
    /// `AUDIT_ARCH_*`.
    pub arch: u32,
    /// Raw argument registers.
    pub args: [u64; 6],
}

/// A tiny software evaluator for the instruction subset this module emits.
/// It lets the tests prove allow/deny behaviour of the *generated program*
/// without installing it into the test process.
pub fn evaluate(program: &[Insn], data: &SeccompData) -> u32 {
    let mut acc: u32 = 0;
    let mut pc = 0usize;
    while pc < program.len() {
        let insn = program[pc];
        pc += 1;
        match insn.code {
            c if c == BPF_LD | BPF_W | BPF_ABS => {
                acc = match insn.k {
                    OFFSET_NR => data.nr,
                    OFFSET_ARCH => data.arch,
                    k if (OFFSET_ARGS..OFFSET_ARGS + 48).contains(&k) => {
                        let index = ((k - OFFSET_ARGS) / 8) as usize;
                        data.args[index] as u32
                    }
                    _ => return RET_KILL_PROCESS,
                };
            }
            c if c == BPF_JMP | BPF_JEQ | BPF_K => {
                pc += usize::from(if acc == insn.k { insn.jt } else { insn.jf });
            }
            c if c == BPF_JMP | BPF_JSET | BPF_K => {
                pc += usize::from(if acc & insn.k != 0 { insn.jt } else { insn.jf });
            }
            c if c == BPF_RET | BPF_K => return insn.k,
            _ => return RET_KILL_PROCESS,
        }
    }
    RET_KILL_PROCESS
}
