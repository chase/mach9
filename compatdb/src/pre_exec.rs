//! Resolve the app name before exec without applying any runtime settings.

use std::{ffi::CStr, mem::MaybeUninit};

use compatdb_core::runtime::{basename_of, steam_app, steam_app_id};

use crate::{RULE_SOURCES, load_table};

/// Look up a name before forking a Windows child. Use the target's
/// environment: a game launched by Steam can have a
/// `SteamAppId` that the Steam client itself does not. This lookup must not
/// apply the target's settings to the process asking for its name.
///
/// Counts are UTF-16 code units, not bytes. The counted DOS image path
/// excludes NUL; the environment includes its final empty entry (an empty
/// environment may also be passed as null with length zero).
///
/// Return the number of UTF-8 bytes written, excluding the terminating NUL,
/// or zero without writing on no match, invalid input, or insufficient space.
/// The result is display text, not a path component.
///
/// # Safety
/// Nonempty input regions must be readable, initialized, aligned and valid for
/// their counts. The output must be writable for its count and not overlap
/// either input.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn compatdb_resolve_display_name(
    image: *const u16,
    image_len: usize,
    env: *const u16,
    env_len: usize,
    out: *mut libc::c_char,
    out_size: usize,
) -> usize {
    if image_len == 0
        || image.is_null()
        || !image.is_aligned()
        || image_len > isize::MAX as usize / size_of::<u16>()
        || env_len > isize::MAX as usize / size_of::<u16>()
        || (env_len != 0 && (env.is_null() || !env.is_aligned()))
        || out.is_null()
        || out_size == 0
        || out_size > isize::MAX as usize
    {
        return 0;
    }
    // SAFETY: the caller provides valid regions; checked the slice size,
    // nullness and alignment above. A zero-length env need not have a pointer.
    let (image, env) = unsafe {
        (
            std::slice::from_raw_parts(image, image_len),
            if env_len == 0 {
                &[]
            } else {
                std::slice::from_raw_parts(env, env_len)
            },
        )
    };
    if image.contains(&0) || !valid_env(env) {
        return 0;
    }
    let exe = basename_of(image);
    if exe.is_empty() {
        return 0;
    }

    // SAFETY: output region and its bound were checked above.
    let output = unsafe { std::slice::from_raw_parts_mut(out.cast(), out_size) };
    lookup(&exe, steam_app(env), output)
}

/// Look up a direct launch using argv and the Unix `SteamAppId`.
///
/// # Safety
/// `image` and non-null `appid` must be NUL-terminated. `out` must be
/// writable for `out_size` bytes and disjoint from both inputs.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn compatdb_resolve_unix_display_name(
    image: *const libc::c_char,
    appid: *const libc::c_char,
    out: *mut libc::c_char,
    out_size: usize,
) -> usize {
    if image.is_null() || out.is_null() || out_size == 0 || out_size > isize::MAX as usize {
        return 0;
    }
    // SAFETY: the caller guarantees NUL termination and writable disjoint output.
    let (image, appid, output) = unsafe {
        (
            CStr::from_ptr(image).to_bytes(),
            (!appid.is_null()).then(|| CStr::from_ptr(appid)),
            std::slice::from_raw_parts_mut(out.cast(), out_size),
        )
    };
    let Some(exe) = initial_basename(image) else {
        return 0;
    };
    lookup(
        exe,
        appid.and_then(|s| s.to_str().ok()).and_then(steam_app_id),
        output,
    )
}

fn initial_basename(image: &[u8]) -> Option<&str> {
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

fn lookup(exe: &str, steam: Option<u32>, output: &mut [MaybeUninit<u8>]) -> usize {
    let Some(sources) = RULE_SOURCES.get() else {
        return 0;
    };
    if sources.inline.is_none() && sources.file.is_none() {
        return 0;
    }
    let (table, _) = load_table(sources);
    let resolution = table.resolve_with_optional_version(exe, None, steam);
    let Some(name) = resolution.display_name() else {
        return 0;
    };
    let Some((bytes, tail)) = output.split_at_mut_checked(name.len()) else {
        return 0;
    };
    let Some(nul) = tail.first_mut() else {
        return 0;
    };
    bytes.write_copy_of_slice(name.as_bytes());
    nul.write(0);
    name.len()
}

fn valid_env(env: &[u16]) -> bool {
    match env {
        [] | [0] | [0, 0] => true,
        [entries @ .., 0, 0] => {
            entries.first() != Some(&0)
                && entries.last() != Some(&0)
                && !entries.windows(2).any(|pair| pair == [0, 0])
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests;
