use system::{dmesg, dmesg_err, freopen, mount, reboot};

// TODO: call seed_entropy with a generic source on non-aws targets, maybe by providing get_entropy
// as an Option<fn> rather than fn.
#[cfg(feature = "aws")]
use system::seed_entropy;

#[cfg(feature = "aws")]
use aws::{get_entropy, init_platform};

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
            Ok(()) => dmesg(format!("Mounted {target}")),
            Err(e) => dmesg_err(format!("Unable to mount {target} ({e})")),
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
            Err(e) => dmesg_err(format!("Unable to open {filename} ({e})")),
        }
    }
}

fn boot() {
    init_rootfs();
    init_console();
    #[cfg(feature = "aws")]
    init_platform();
    #[cfg(feature = "aws")]
    match seed_entropy(4096, get_entropy) {
        Ok(size) => dmesg(format!("Seeded kernel with entropy: {size}")),
        Err(e) => dmesg_err(format!("Unable to seed kernel with entropy: {e}")),
    };
}

fn main() {
    boot();
    dmesg("EnclaveOS Booted".to_string());
    reboot();
}
