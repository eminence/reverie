#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unreachable_code)]
#![allow(unused_variables)]

#[macro_use]
extern crate lazy_static;

use clap::{App, Arg, SubCommand};
use libc;
use fern;
use nix::sys::wait::WaitStatus;
use nix::sys::{ptrace, signal, wait};
use nix::unistd;
use nix::unistd::ForkResult;
use std::collections::HashMap;
use std::env::current_exe;
use std::ffi::CString;
use std::io::{Error, ErrorKind, Result};
use std::path::PathBuf;

mod consts;
mod hooks;
mod nr;
mod ns;
mod proc;
mod remote;
mod remote_rwlock;
mod sched;
mod sched_wait;
mod stubs;
mod task;
mod traced_task;

use remote::*;
use sched::Scheduler;
use sched_wait::SchedWait;
use task::{RunTask, Task};
use traced_task::TracedTask;

// install seccomp-bpf filters
extern "C" {
    fn bpf_install();
}

#[test]
fn can_resolve_syscall_hooks() -> Result<()> {
    let parsed = hooks::resolve_syscall_hooks_from(PathBuf::from("lib").join(consts::SYSTRACE_SO))?;
    assert_ne!(parsed.len(), 0);
    Ok(())
}

#[test]
fn libsystrace_trampoline_within_first_page() -> Result<()> {
    let parsed = hooks::resolve_syscall_hooks_from(PathBuf::from("lib").join(consts::SYSTRACE_SO))?;
    let filtered: Vec<_> = parsed.iter().filter(|hook| hook.offset < 0x1000).collect();
    assert_eq!(parsed.len(), filtered.len());
    Ok(())
}

struct Arguments<'a> {
    debug_level: i32,
    library_path: PathBuf,
    env_all: bool,
    envs: HashMap<String, String>,
    program: &'a str,
    program_args: Vec<&'a str>,
}

fn run_tracer_main(sched: &mut SchedWait) -> Result<i32> {
    let mut exit_code = 0i32;
    while let Some(task) = sched.next() {
        let run_result = task.run()?;
        match run_result {
            RunTask::Exited(_code) => exit_code = _code,
            RunTask::Runnable(task1) => sched.add(task1),
            RunTask::Forked(parent, child) => {
                sched.add(child);
                sched.add_and_schedule(parent);
            }
        }
    }
    Ok(exit_code)
}

fn wait_sigstop(pid: unistd::Pid) -> Result<()> {
    match wait::waitpid(Some(pid), None).expect("waitpid failed") {
        WaitStatus::Stopped(new_pid, signal) if signal == signal::SIGSTOP && new_pid == pid => {
            Ok(())
        }
        _ => Err(Error::new(ErrorKind::Other, "expect SIGSTOP")),
    }
}

fn from_nix_error(err: nix::Error) -> Error {
    Error::new(ErrorKind::Other, err)
}

// hardcoded because `libc` does not export
const ADDR_NO_RANDOMIZE: u64 = 0x0040000;

fn run_tracee(argv: &Arguments) -> Result<i32> {
    // FIXME: There should NOT be a hardcoded tool name!:
    let libs: Result<Vec<PathBuf>> = ["libechotool.so", "libsystrace.so"]
        .iter()
        .map(|so| argv.library_path.join(so).canonicalize())
        .collect();
    let ldpreload = String::from("LD_PRELOAD=")
        + &libs?
            .iter()
            .map(|p| p.to_str().unwrap())
            .collect::<Vec<_>>()
            .join(":");

    unsafe {
        assert!(libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) == 0);
        assert!(libc::personality(ADDR_NO_RANDOMIZE) != -1);
    };

    ptrace::traceme()
        .and_then(|_| signal::raise(signal::SIGSTOP))
        .map_err(from_nix_error)?;

    let root_pid = unistd::getpid();
    unistd::setpgid(root_pid, root_pid).expect("setpgid");

    // println!("launching program: {} {:?}", &argv.program, &argv.program_args);

    // install seccomp-bpf filters
    // NB: the only syscall beyond this point should be
    // execvpe only.
    unsafe { bpf_install() };

    let mut envs: Vec<String> = Vec::new();

    if argv.env_all {
        std::env::vars().for_each(|(k, v)| {
            envs.push(format!("{}={}", k, v));
        });
    } else {
        envs.push(String::from("PATH=/bin/:/usr/bin"));
    }

    argv.envs.iter().for_each(|(k, v)| {
        if v.len() == 0 {
            envs.push(k.to_string())
        } else {
            envs.push(format!("{}={}", k, v));
        }
    });

    envs.push(ldpreload);
    let program = CString::new(argv.program)?;
    let mut args: Vec<CString> = Vec::new();
    CString::new(argv.program).map(|s| args.push(s))?;
    for v in argv.program_args.clone() {
        CString::new(v).map(|s| args.push(s))?;
    }
    let envp: Vec<CString> = envs
        .into_iter()
        .map(|s| CString::new(s.as_bytes()).unwrap())
        .collect();
    unistd::execvpe(&program, args.as_slice(), envp.as_slice()).map_err(from_nix_error)?;
    panic!("exec failed: {} {:?}", &argv.program, &argv.program_args);
}

fn run_tracer(
    starting_pid: unistd::Pid,
    starting_uid: unistd::Uid,
    starting_gid: unistd::Gid,
    argv: &Arguments,
) -> Result<i32> {
    ns::init_ns(starting_pid, starting_uid, starting_gid)?;

    // tracer is the 1st process in the new namespace.
    assert!(unistd::getpid() == unistd::Pid::from_raw(1));

    match unistd::fork().expect("fork failed") {
        ForkResult::Child => {
            return run_tracee(argv);
        }
        ForkResult::Parent { child } => {
            // wait for sigstop
            wait_sigstop(child)?;
            ptrace::setoptions(
                child,
                ptrace::Options::PTRACE_O_TRACEEXEC
                    | ptrace::Options::PTRACE_O_EXITKILL
                    | ptrace::Options::PTRACE_O_TRACECLONE
                    | ptrace::Options::PTRACE_O_TRACEFORK
                    | ptrace::Options::PTRACE_O_TRACEVFORK
                    | ptrace::Options::PTRACE_O_TRACEVFORKDONE
                    | ptrace::Options::PTRACE_O_TRACEEXIT
                    | ptrace::Options::PTRACE_O_TRACESECCOMP
                    | ptrace::Options::PTRACE_O_TRACESYSGOOD,
            )
            .map_err(|e| Error::new(ErrorKind::Other, e))?;
            ptrace::cont(child, None).map_err(|e| Error::new(ErrorKind::Other, e))?;
            let tracee = task::Task::new(child);
            let mut sched: SchedWait = Scheduler::new();
            sched.add(tracee);
            run_tracer_main(&mut sched)
        }
    }
}

fn run_app(argv: &Arguments) -> Result<i32> {
    let (starting_pid, starting_uid, starting_gid) =
        (unistd::getpid(), unistd::getuid(), unistd::getgid());
    unsafe {
        assert!(
            libc::unshare(
                libc::CLONE_NEWUSER | libc::CLONE_NEWPID | libc::CLONE_NEWNS | libc::CLONE_NEWUTS
            ) == 0
        );
    };

    match unistd::fork().expect("fork failed") {
        ForkResult::Child => run_tracer(starting_pid, starting_uid, starting_gid, argv),
        ForkResult::Parent { child } => match wait::waitpid(Some(child), None) {
            Ok(wait::WaitStatus::Exited(_, exit_code)) => Ok(exit_code),
            Ok(wait::WaitStatus::Signaled(_, sig, _)) => Ok(0x80 | sig as i32),
            otherwise => panic!("unexpected status from waitpid: {:?}", otherwise),
        },
    }
}

fn main() {
    let matches = App::new("systrace - a fast syscall tracer and interceper")
        .version("0.0.1")
        .arg(
            Arg::with_name("debug")
                .long("debug")
                .value_name("DEBUG_LEVEL")
                .help("Set debug level [0..5]")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("library-path")
                .long("library-path")
                .value_name("LIBRARY_PATH")
                // FIXME: There should NOT be a hardcoded tool name!:
                .help("set library search path for libsystrace.so, libechotool.so")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("env-all")
                .long("env-all")
                .value_name("ENV-ALL")
                .help("inherits all environment variables")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("env")
                .long("env")
                .value_name("ENV")
                .multiple(true)
                .help("set environment variable")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("program")
                .value_name("PROGRAM")
                .required(true)
                .help("PROGRAM")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("program_args")
                .value_name("PROGRAM_ARGS")
                .allow_hyphen_values(true)
                .multiple(true)
                .help("[PROGRAM_ARGUMENTS..]"),
        )
        .get_matches();

    let argv = Arguments {
        debug_level: matches
            .value_of("debug")
            .and_then(|x| x.parse::<i32>().ok())
            .unwrap_or(0),
        library_path: matches
            .value_of("library-path")
            .and_then(|p| PathBuf::from(p).canonicalize().ok())
            .or_else(|| PathBuf::from("lib").canonicalize().ok())
            .or_else(|| PathBuf::from(".").canonicalize().ok())
            .expect("cannot find library path"),
        env_all: matches.is_present("env-all"),
        envs: matches
            .values_of("env")
            .unwrap_or_default()
            .map(|s| {
                let t: Vec<&str> = s.clone().split('=').collect();
                debug_assert!(t.len() > 0);
                (t[0].to_string(), t[1..].join("="))
            })
            .collect(),
        program: matches.value_of("program").unwrap_or(""),
        program_args: matches
            .values_of("program_args")
            .map(|v| v.collect())
            .unwrap_or_else(|| Vec::new()),
    };

    setup_logger(argv.debug_level).expect("set log level");
    std::env::set_var(consts::SYSTRACE_LIBRARY_PATH, &argv.library_path);
    match run_app(&argv) {
        Ok(exit_code) => std::process::exit(exit_code),
        err => panic!("run app failed with error: {:?}", err),
    }
}

fn setup_logger(level: i32) -> Result<()> {
    let log_level = match level {
        0 => log::LevelFilter::Off,
        1 => log::LevelFilter::Error,
        2 => log::LevelFilter::Warn,
        3 => log::LevelFilter::Info,
        4 => log::LevelFilter::Debug,
        5 => log::LevelFilter::Trace,
        _ => log::LevelFilter::Trace,
    };

    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}",
                message
            ))
        })
        .level(log_level)
        .chain(std::io::stdout())
        .chain(fern::log_file("output.log")?)
        .apply().map_err(|e| Error::new(ErrorKind::Other, e))
}
