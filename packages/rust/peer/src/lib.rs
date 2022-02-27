mod io;
/// The handlers should be fully complaint with the spec. They will only produce errors when the protocol is violated. Handled errors (such as a protocol mismatch) will result in an error event but return Ok().
pub mod services;
