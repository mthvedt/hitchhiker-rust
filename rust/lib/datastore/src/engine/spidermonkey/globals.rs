use std::cell::{Cell, RefCell};
use std::io::{stderr, Write};
use std::mem;
use std::ptr;

use js::jsapi::{JSContext, JSErrorReport};
use libc::c_char;

use engine::error::{Exception, ErrorReporter};

use super::error::error_from_js;

thread_local! {
    // Safety check. Must be equal to the executing context.
    // TODO: we will probably need this to be a stack.
    static CURRENT_CONTEXT: Cell<*const JSContext> = Cell::new(ptr::null());

    // The error reporter for the currently executing context. Must be set while JS is executing.
    // TODO: we will probably need this to be an 'option stack', where contexts may optionally push/pop.
    static CURRENT_ERROR_REPORTER: RefCell<Option<Box<ErrorReporter>>> = RefCell::new(None);
}

pub struct ActiveGlobals;

impl ActiveGlobals {
    pub fn set_scoped<E: ErrorReporter + 'static>(cx_p: *const JSContext, reporter: E) -> Self {
        assert!(CURRENT_CONTEXT.with(|cx_p_cell| cx_p_cell.get() == ptr::null()));
        assert!(CURRENT_ERROR_REPORTER.with(|r| r.borrow().is_none()));

        CURRENT_CONTEXT.with(|cx_p_cell| cx_p_cell.set(cx_p));
        CURRENT_ERROR_REPORTER.with(|r| *r.borrow_mut() = Some(Box::new(reporter)));

        ActiveGlobals
    }

    pub extern "C" fn report_warning(cx: *mut JSContext, message: *const c_char, report: *mut JSErrorReport) {
        // The current context and error queue *must* be set.
        // TODO: when ptr_eq is stable, use that
        assert!(CURRENT_CONTEXT.with(|cx_p_cell| cx_p_cell.get() == cx));
        let err = error_from_js(message, report);
        CURRENT_ERROR_REPORTER.with(|e| e.borrow_mut().as_mut().unwrap().report_warning(err));
    }

    pub fn report_exception(&self, cx: *mut JSContext, ex: Exception) {
        // The current context and error queue *must* be set.
        // TODO: when ptr_eq is stable, use that
        assert!(CURRENT_CONTEXT.with(|cx_p_cell| cx_p_cell.get() == cx));
        CURRENT_ERROR_REPORTER.with(|e| e.borrow_mut().as_mut().unwrap().report_exception(ex));
    }
}

impl Drop for ActiveGlobals {
    fn drop(&mut self) {
        CURRENT_CONTEXT.with(|cx_p_cell| cx_p_cell.set(ptr::null()));
        CURRENT_ERROR_REPORTER.with(|r| {
            match r.try_borrow_mut() {
                Ok(mut rref) => match mem::replace(&mut *rref, None) {
                    Some(r) => if !r.is_empty() {
                        // We use unwrap()--if we can't writeln to stderr, there's no hope for us!
                        writeln!(&mut stderr(),
                            "WARNING: javascript errors ignored during unwinding").unwrap();
                        // TODO: print errors to stderr
                    },
                    None => {
                        writeln!(&mut stderr(),
                            "WARNING: javascript reporter in invalid state during unwinding").unwrap();
                    }
                },
                Err(_) => {
                    // Panic during drop (probably terminating the program), but explain why
                    writeln!(&mut stderr(),
                        "FATAL: couldn't access javascript error queue during unwinding").unwrap();
                    panic!();
                },
            }
        });
    }
}
