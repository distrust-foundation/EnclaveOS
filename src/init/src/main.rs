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
fn init_entropy() {
    #[allow(unused_mut, unused_assignments, clippy::type_complexity)]
    let mut get_entropy: Option<fn(usize) -> Result<Vec<u8>>> = None;

    #[cfg(feature = "platform-aws")]
    {
        get_entropy = Some(platforms::aws::get_entropy);
    }

    if let Some(get_entropy) = get_entropy {
        match sys::seed_entropy(4096, get_entropy) {
            Ok(size) => dprintln!("Seeded kernel with entropy: {size}"),
            Err(e) => deprintln!("Unable to seed kernel with entropy: {e}"),
        };
    }
}

fn boot() {
    init_rootfs();
    init_console();

    // TODO: should a failure loading AWS components continue? AWS components are only loaded when
    // building with the AWS target enabled, so by non-AWS usage this component should not be
    // loaded.
    #[cfg(feature = "platform-aws")]
    match platforms::aws::init_platform() {
        Ok(_) => dprintln!("Successfully sent Nitro heartbeat and loaded necessary kernel modules"),
        Err(e) => deprintln!("Error when initializing AWS functionality: {e}"),
    }

    init_entropy();
}

fn main() {
    boot();
    deprintln!("EnclaveOS Booted");
    reboot();
}
