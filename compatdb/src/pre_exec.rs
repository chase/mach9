//! Resolve the app name before exec without applying runtime settings.

use std::{ffi::CStr, ptr};

use compatdb_core::runtime::steam_app_id;

use crate::{RULE_SOURCES, load_table};

/// Look up the name for Wine's target image and its optional `SteamAppId`.
/// Return the number of UTF-8 bytes written, excluding the terminating NUL,
/// or zero without writing if no name matches or the buffer is too small.
///
/// # Safety
/// `image` and non-null `appid` must be NUL-terminated. `out` must be writable
/// for `out_size` bytes and disjoint from both inputs.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn compatdb_lookup_display_name(
    image: *const libc::c_char,
    appid: *const libc::c_char,
    out: *mut libc::c_char,
    out_size: usize,
) -> usize {
    let Some(sources) = RULE_SOURCES.get() else {
        return 0;
    };
    if image.is_null() || out.is_null() || (sources.inline.is_none() && sources.file.is_none()) {
        return 0;
    }
    // SAFETY: the caller guarantees NUL termination for non-null inputs.
    let image = unsafe { CStr::from_ptr(image) };
    let Some(exe) = launch_basename(image.to_bytes()) else {
        return 0;
    };
    let appid = if appid.is_null() {
        None
    } else {
        // SAFETY: the caller guarantees NUL termination for non-null inputs.
        unsafe { CStr::from_ptr(appid) }
            .to_str()
            .ok()
            .and_then(steam_app_id)
    };
    let (table, _) = load_table(sources);
    let resolution = table.resolve_with_optional_version(exe, None, appid);
    let Some(name) = resolution
        .display_name()
        .filter(|name| name.len() < out_size)
    else {
        return 0;
    };
    // SAFETY: Wine provides a writable output buffer disjoint from the name.
    // The capacity check leaves room for the terminator.
    unsafe {
        ptr::copy_nonoverlapping(name.as_ptr().cast(), out, name.len());
        out.add(name.len()).write(0);
    }
    name.len()
}

fn launch_basename(image: &[u8]) -> Option<&str> {
    let exe = if image.starts_with(b"/") {
        // Wine tries absolute Unix paths as written first.
        image.rsplit(|&b| b == b'/').next()?
    } else {
        let path = match image {
            [drive, b':', rest @ ..] if drive.is_ascii_alphabetic() => rest,
            _ => image,
        };
        let leaf = path.rsplit(|&b| matches!(b, b'/' | b'\\')).next()?;
        // DOS lookup strips trailing spaces and dots from the last component.
        let end = leaf.iter().rposition(|b| !matches!(b, b' ' | b'.'))? + 1;
        leaf.get(..end)?
    };
    if matches!(exe, b"" | b"." | b"..") {
        return None;
    }
    std::str::from_utf8(exe).ok()
}

#[cfg(test)]
mod tests;
