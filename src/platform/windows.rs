// Copyright (c) 2017 CtrlC developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

pub extern crate winapi;

use self::winapi::ctypes::c_long;
use self::winapi::shared::minwindef::{BOOL, DWORD, FALSE, TRUE};
use self::winapi::shared::ntdef::HANDLE;
use self::winapi::um::consoleapi::SetConsoleCtrlHandler;
use self::winapi::um::handleapi::CloseHandle;
use self::winapi::um::synchapi::{ReleaseSemaphore, WaitForSingleObject};
use self::winapi::um::winbase::{CreateSemaphoreA, INFINITE, WAIT_FAILED, WAIT_OBJECT_0};
use self::winapi::um::wincon::{CTRL_BREAK_EVENT, CTRL_C_EVENT, CTRL_SHUTDOWN_EVENT};
use crate::signalevent::SignalEvent;
use signal::SignalType;
use std::io;
use std::ops::Range;
use std::ptr;

/// Platform specific error type
pub type Error = io::Error;

/// Platform specific signal type
pub type Signal = DWORD;

/// TODO Platform specific pipe handle type
pub type SignalEmitter = HANDLE;
impl SignalEvent for SignalEmitter {
    fn emit(&self, _signal: &Signal) {
        unsafe { ReleaseSemaphore(*self, 1, ptr::null_mut()) };
    }
}

pub const CTRL_C_SIGNAL: Signal = CTRL_C_EVENT;
pub const TERMINATION_SIGNAL: Signal = CTRL_BREAK_EVENT;
pub const UNINITIALIZED_SIGNAL_EMITTER: HANDLE = winapi::um::handleapi::INVALID_HANDLE_VALUE;

/// Iterator returning available signals on this system
pub fn signal_iterator() -> Range<DWORD> {
    (CTRL_C_EVENT..CTRL_SHUTDOWN_EVENT + 1)
}

pub const MAX_SEM_COUNT: c_long = 255;
static mut SEMAPHORE: HANDLE = 0 as HANDLE;

impl SignalType {
    /// Get the underlying platform specific signal
    pub fn to_platform_signal(&self) -> Signal {
        match *self {
            SignalType::Ctrlc => CTRL_C_EVENT,
            SignalType::Termination => CTRL_BREAK_EVENT,
            SignalType::Other(s) => s,
        }
    }
}

unsafe extern "system" fn os_handler(_: DWORD) -> BOOL {
    // Assuming this always succeeds. Can't really handle errors in any meaningful way.
    ReleaseSemaphore(SEMAPHORE, 1, ptr::null_mut());
    TRUE
}

/// Registers an os signal handler.
///
/// Must be called before calling [`block_ctrl_c()`](fn.block_ctrl_c.html)
/// and should only be called once.
///
/// # Errors
/// Will return an error if a system error occurred.
///
#[inline]
pub unsafe fn init_os_handler() -> Result<(), Error> {
    SEMAPHORE = CreateSemaphoreA(ptr::null_mut(), 0, MAX_SEM_COUNT, ptr::null());
    if SEMAPHORE.is_null() {
        return Err(io::Error::last_os_error());
    }

    if SetConsoleCtrlHandler(Some(os_handler), TRUE) == FALSE {
        let e = io::Error::last_os_error();
        CloseHandle(SEMAPHORE);
        SEMAPHORE = 0 as HANDLE;
        return Err(e);
    }

    Ok(())
}

/// Blocks until a Ctrl-C signal is received.
///
/// Must be called after calling [`init_os_handler()`](fn.init_os_handler.html).
///
/// # Errors
/// Will return an error if a system error occurred.
///
#[inline]
pub unsafe fn block_ctrl_c() -> Result<(), Error> {
    match WaitForSingleObject(SEMAPHORE, INFINITE) {
        WAIT_OBJECT_0 => Ok(()),
        WAIT_FAILED => Err(io::Error::last_os_error()),
        ret => Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "WaitForSingleObject(), unexpected return value \"{:x}\"",
                ret
            ),
        )),
    }
}