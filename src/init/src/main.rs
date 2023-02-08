mod sys;
use sys::{freopen, mount, reboot};

mod dmesg;
use dmesg::{deprintln, dprintln};

mod platforms;

// TODO: call seed_entropy with a generic source on non-aws targets, maybe by providing get_entropy
// as an Option<fn> rather than fn.
#[cfg(feature = "platform-aws")]
use sys::seed_entropy;

#[cfg(feature = "platform-aws")]
use platforms::aws::{get_entropy, init_platform};

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
    #[cfg(feature = "platform-aws")]
    match seed_entropy(4096, get_entropy) {
        Ok(size) => println!("Seeded kernel with entropy: {size}"),
        Err(e) => eprintln!("Unable to seed kernel with entropy: {e}"),
    };
}

fn main() {
    boot();
    deprintln!("EnclaveOS Booted");
    reboot();
}
