use core::ptr;
use errno;
use std::ffi::{CString, OsString};

pub mod commit;
pub mod diff;
pub mod status;

const PAGER_CMD: &str = "less";
const PAGER_ENV: &str = "FRX";

fn pager() {
    if !atty::is(atty::Stream::Stdout) {
        return;
    }
    std::env::set_var("LESS", PAGER_ENV);
    let (pg_stdin, main_stdout) = pipe();
    let pid = fork();
    match pid {
        -1 => {
            close(pg_stdin);
            close(main_stdout);
        }
        0 => {
            dup2(main_stdout, libc::STDOUT_FILENO);
            close(pg_stdin);
        }
        _ => {
            dup2(pg_stdin, libc::STDIN_FILENO);
            close(main_stdout);
            execvp(PAGER_CMD);
        }
    }
}

fn close(fd: i32) {
    assert_eq!(unsafe { libc::close(fd) }, 0);
}

fn dup2(fd1: i32, fd2: i32) {
    assert!(unsafe { libc::dup2(fd1, fd2) } > -1);
}

fn execvp(cmd: &str) {
    let mut args: Vec<_> = vec![CString::new(cmd).unwrap().as_ptr()];
    args.push(ptr::null());
    errno::set_errno(errno::Errno(0));
    unsafe { libc::execvp(args[0], args.as_ptr()) };
}

fn fork() -> libc::pid_t {
    unsafe { libc::fork() }
}

fn pipe() -> (i32, i32) {
    let mut fds = [0; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    (fds[0], fds[1])
}
