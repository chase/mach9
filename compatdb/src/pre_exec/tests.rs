use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn query(image: &CStr, appid: Option<&CStr>, out: &mut [u8]) -> usize {
    // SAFETY: both strings are NUL-terminated and the output is disjoint.
    unsafe {
        compatdb_lookup_display_name(
            image.as_ptr(),
            appid.map_or(ptr::null(), CStr::as_ptr),
            out.as_mut_ptr().cast(),
            out.len(),
        )
    }
}

#[test]
fn ffi_reads_the_current_file_and_uses_counted_utf8_output() -> TestResult {
    // Set the Unix environment in a subprocess, not the parallel test runner.
    const CHILD: &str = "MACH9_COMPATDB_TEST_PRE_EXEC_CHILD";
    if let Some(path) = std::env::var_os(CHILD) {
        let image = c"C:\\Games\\Game.exe";
        let appid = Some(c"440");
        let mut buf = [0xff; 6];
        std::fs::write(&path, "exe=steam:440;display_name=Game")?;
        for image in [image, c"/games/Game.exe", c"Game.exe"] {
            assert_eq!(query(image, appid, &mut buf), 4);
            assert_eq!(buf, *b"Game\0\xff");
        }
        // At this stage Wine has only the target path and AppID.
        std::fs::write(
            &path,
            "exe=Game.exe;display_name=Plain\nexe=Game.exe;company=Acme;display_name=Pinned",
        )?;
        for image in [
            c"C:\\Games\\Game.exe",
            c"C:\\..\\Game.exe. ",
            c"Game.exe.",
            c"D:Game.exe",
            c"\\\\host\\share\\Game.exe",
        ] {
            assert_eq!(query(image, None, &mut buf), 5, "{image:?}");
            assert_eq!(buf, *b"Plain\0");
        }
        assert_eq!(query(image, appid, &mut buf), 5);
        assert_eq!(buf, *b"Plain\0");
        std::fs::write(
            &path,
            "exe=steam:440;display_name=Game\nexe=Game.exe;display_name=default",
        )?;
        assert_eq!(query(image, appid, &mut buf), 0);
        buf.fill(0xff);
        std::fs::write(&path, "exe=steam:440;display_name=Game")?;
        assert_eq!(query(image, appid, &mut buf), 4);
        assert_eq!(buf, *b"Game\0\xff");

        std::fs::write(&path, "exe=steam:440;display_name=café")?;
        let mut short = [0xff; 5];
        assert_eq!(query(image, appid, &mut short), 0);
        assert_eq!(short, [0xff; 5]);
        assert_eq!(query(image, appid, &mut buf), 5);
        assert_eq!(buf.as_slice(), "café\0".as_bytes());
        assert_eq!(query(image, Some(c"440x"), &mut buf), 0);
        assert_eq!(query(image, None, &mut buf), 0);
        assert_eq!(query(c"", appid, &mut buf), 0);
        assert_eq!(buf.as_slice(), "café\0".as_bytes());
        return Ok(());
    }
    let dir = std::env::temp_dir().join(format!("compatdb-pre-exec-{}", std::process::id()));
    std::fs::create_dir(&dir)?;
    let path = dir.join("rules");
    let status = std::process::Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            "pre_exec::tests::ffi_reads_the_current_file_and_uses_counted_utf8_output",
        ])
        .env(CHILD, &path)
        .env(compatdb_core::FILE_ENV_VAR, &path)
        .env(compatdb_core::ENV_VAR, "exe=steam:440;display_name=Inline")
        .status();
    std::fs::remove_dir_all(dir)?;
    assert!(status?.success());
    Ok(())
}
