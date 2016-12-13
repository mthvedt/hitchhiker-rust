use std::ffi::CStr;
use std::ptr;
use std::slice;

use js::jsapi::JSErrorReport;
use libc::c_char;

use engine::error::Error;

// TODO: don't have spidermonkey-dependent code here.
pub fn error_from_js(message: *const c_char, report: *const JSErrorReport) -> Error {
    // TODO: JSREPORT_EXCEPTION?
    assert!(report != ptr::null());
    let report = unsafe { &*report };
    assert!(!report.isMuted); // We don't know how to handle this yet

    let message2_len = 65536;

    Error {
        filename: unsafe { CStr::from_ptr(report.filename).to_string_lossy().into_owned() },
        line: String::from_utf16_lossy(unsafe { slice::from_raw_parts( report.linebuf_, report.linebufLength_) }),
        lineno: report.lineno,
        column: report.column,
        is_muted: report.isMuted,
        js_flags: report.flags,
        js_errno: report.errorNumber,
        message: unsafe { CStr::from_ptr(message).to_string_lossy().into_owned() },
        // TODO: We have to use JS to parse the error message. Yuck.
        // TODO: incredibly unsafe! probably remove this!
        message2: String::from_utf16_lossy(unsafe { slice::from_raw_parts( report.ucmessage, message2_len) }),
        js_exntype: report.exnType,
    }
}
