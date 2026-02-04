pub mod wall_clock;

// Can only get working for macos for now
// #[cfg(target_os = "macos")]
pub mod qbdi;

#[cfg(target_family = "unix")]
pub mod perf_events;

#[cfg(any(target_arch = "x86", target_arch = "x86_64", debug_assertions))]
pub mod sde;

pub fn install_qbdi() {
    use std::process::{Command, Stdio};

    let file = "qbdi.pkg";

    Command::new("curl")
        .arg("https://github.com/QBDI/QBDI/releases/download/v0.12.0/QBDI-0.12.0-osx-AARCH64.pkg")
        .arg("-L")
        .arg("-o")
        .arg(file)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap()
        .wait()
        .unwrap();

    let dest = std::env::home_dir().unwrap().join("qbdi-out");
    std::fs::create_dir_all(&dest).unwrap();

    Command::new("sudo")
        .arg("installer")
        .arg("-pkg")
        .arg(file)
        .arg("-target")
        .arg(dest)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap()
        .wait()
        .unwrap();

    std::fs::remove_file(file).unwrap();
}

pub fn install_sde() {
    // Based on https://github.com/petarpetrovt/setup-sde/blob/main/index.ts

    use std::process::{Command, Stdio};

    let base = "https://downloadmirror.intel.com";

    #[cfg(target_os = "linux")]
    let platform = "lin";

    #[cfg(target_os = "macos")]
    let platform = "mac";

    #[cfg(target_os = "windows")]
    let platform = "win";

    let url = format!("{base}/859732/sde-external-9.58.0-2025-06-16-{platform}.tar.xz");
    let file = "sde-temp-file.tar.xz";

    Command::new("curl")
        .arg(url)
        .arg("-o")
        .arg(file)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap()
        .wait()
        .unwrap();

    // exec.exec(`"${tarExePath}"`, [`x`, `--force-local`, `-C`, `${extractedFilesPath}`, `-f`, `${tarFilePath}`]);

    Command::new("tar")
        // -x extract, -v verbose, -j archive with gzip/bzip2/xz/lzma, -f pass filename
        .arg("-xvf") // -xvjf
        .arg(file)
        // .stdout(Stdio::inherit())
        // .stderr(Stdio::inherit())
        .spawn()
        .unwrap()
        .wait()
        .unwrap();

    let bin_dest = adjacent_sde_path().unwrap();
    let dest = bin_dest.parent().unwrap();
    let target = format!("sde-external-9.58.0-2025-06-16-{platform}");
    std::fs::rename(target, dest).unwrap();
}

#[cfg(target_os = "windows")]
pub fn adjacent_sde_path() -> Option<std::path::PathBuf> {
    let path = std::env::current_exe().ok()?;
    Some(path.parent()?.join("sde").join("sde").with_extension("exe"))
}

#[cfg(unix)]
pub fn adjacent_sde_path() -> Option<std::path::PathBuf> {
    let path = std::env::current_exe().ok()?;
    Some(path.parent()?.join("sde").join("sde"))
}
