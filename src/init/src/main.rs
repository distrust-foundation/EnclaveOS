use std::fmt::Display;

mod sys;
use sys::{freopen, mount, reboot};

mod dmesg;
use dmesg::{deprintln, dprintln};

mod error;
mod platforms;

use error::Result;

/// Mount common filesystems with conservative permissions.
fn init_rootfs() {
    use libc::{MS_NODEV, MS_NOEXEC, MS_NOSUID};
    let no_dse = MS_NODEV | MS_NOSUID | MS_NOEXEC;
    let no_se = MS_NOSUID | MS_NOEXEC;
    let args = [
        ("devtmpfs", "/dev", "devtmpfs", no_se, "mode=0755"),
        ("devtmpfs", "/dev", "devtmpfs", no_se, "mode=0755"),
        ("devpts", "/dev/pts", "devpts", no_se, ""),
        ("shm", "/dev/shm", "tmpfs", no_dse, "mode=0755"),
        ("proc", "/proc", "proc", no_dse, "hidepid=2"),
        ("tmpfs", "/run", "tmpfs", no_dse, "mode=0755"),
        ("tmpfs", "/tmp", "tmpfs", no_dse, ""),
        ("sysfs", "/sys", "sysfs", no_dse, ""),
        (
            "cgroup_root",
            "/sys/fs/cgroup",
            "tmpfs",
            no_dse,
            "mode=0755",
        ),
    ];
    for (src, target, fstype, flags, data) in args {
        match mount(src, target, fstype, flags, data) {
            Ok(()) => dprintln!("Mounted {target}"),
            Err(e) => deprintln!("Unable to mount {target} ({e})"),
        }
    }
}

/// Initialize console with stdin/stdout/stderr.
fn init_console() {
    let args = [
        ("/dev/console", "r", 0),
        ("/dev/console", "w", 1),
        ("/dev/console", "w", 2),
    ];
    for (filename, mode, file) in args {
        match freopen(filename, mode, file) {
            Ok(()) => {}
            Err(e) => deprintln!("Unable to open {filename} ({e})"),
        }
    }
}

/// Use a `get_entropy` function (if available) to seed the RNG.
fn init_entropy() -> Result<()> {
    #[allow(unused_mut, unused_assignments, clippy::type_complexity)]
    let mut get_entropy: Option<fn(usize) -> Result<Vec<u8>>> = None;

    #[cfg(feature = "platform-aws")]
    {
        get_entropy = Some(platforms::aws::get_entropy);
    }

    if let Some(get_entropy) = get_entropy {
        let size = sys::seed_entropy(4096, get_entropy)?;
        dprintln!("Seeded kernel with entropy: {size}");
    }

    Ok(())
}

fn assert_ok<T, D>(value: Result<T>, context: D) where D: Display + Send + Sync + 'static {
    if let Err(error) = value.as_ref() {
        deprintln!("{context}: {error}");
        #[cfg(not(debug_assertions))]
        {
            deprintln!("Unable to recover from above system error, rebooting");
            reboot();
        }
    }
}

fn assert_ok_or<T, D, F>(value: Result<T>, context_fn: F) where D: Display + Send + Sync + 'static, F: FnOnce() -> D {
    if let Err(error) = value.as_ref() {
        deprintln!("{}: {}", context_fn(), error);
        #[cfg(not(debug_assertions))]
        {
            deprintln!("Unable to recover from above system error, rebooting");
            reboot();
        }
    }
}

fn boot() {
    init_rootfs();
    init_console();

    #[cfg(feature = "platform-aws")]
    assert_ok(platforms::aws::init_platform(), "Error when initializing AWS functionality");
    assert_ok(init_entropy(), "Unable to seed kernel with entropy");
}

fn main() {
    boot();
    deprintln!("EnclaveOS Booted");
    reboot();
}
