extern crate libc;
use libc::c_ulong;
use libc::c_int;
use libc::read;
use libc::write;
use libc::close;
use libc::reboot;
use libc::socket;
use libc::connect;
use libc::c_void;
use libc::sockaddr;
use libc::sockaddr_vm;
use libc::SOCK_STREAM;
use libc::AF_VSOCK;
use libc::MS_NOSUID;
use libc::MS_NOEXEC;
use libc::MS_NODEV;
use libc::RB_AUTOBOOT;
use std::mem::zeroed;
use std::mem::size_of;
use std::ffi::CString;

// Log errors to console
pub fn error(message: String){
    eprintln!("{} {}", boot_time(), message);
}

// Log info to console
pub fn info(message: String){
    println!("{} {}", boot_time(), message);
}

// Dmesg formatted seconds since boot
pub fn boot_time() -> String {
    use libc::{clock_gettime, timespec, CLOCK_BOOTTIME};
    let mut t = timespec { tv_sec: 0, tv_nsec: 0 };
    unsafe { clock_gettime(CLOCK_BOOTTIME, &mut t as *mut timespec); }
    format!("[ {: >4}.{}]", t.tv_sec, t.tv_nsec / 1000).to_string()
}

// libc::mount casting/error wrapper
pub fn mount(
    src: &str,
    target: &str,
    fstype: &str,
    flags: c_ulong,
    data: &str,
) {
    use libc::mount;
    let src_cs = CString::new(src).unwrap();
    let target_cs = CString::new(target).unwrap();
    let fstype_cs = CString::new(fstype).unwrap();
    let data_cs = CString::new(data).unwrap();
    if unsafe {
        mount(
            src_cs.as_ptr(),
            target_cs.as_ptr(),
            fstype_cs.as_ptr(),
            flags,
            data_cs.as_ptr() as *const c_void
        )
    } != 0 {
        error(format!("Failed to mount: {}", target));
    }
}

// libc::freopen casting/error wrapper
pub fn freopen(
    filename: &str,
    mode: &str,
    file: c_int,
) {
    use libc::{freopen, fdopen};
    let filename_cs = CString::new(filename).unwrap();
    let mode_cs = CString::new(mode).unwrap();
    if unsafe {
        freopen(
            filename_cs.as_ptr(),
            mode_cs.as_ptr(),
            fdopen(file, mode_cs.as_ptr() as *const i8)
        )
    }.is_null() {
        error(format!("Failed to freopen: {}", filename));
    }
}

// Signal to hypervisor that booting was successful
pub fn heartbeat(){
    let mut buf: [u8; 1] = [0; 1];
    buf[0] = 0xB7; // AWS Nitro heartbeat value
    unsafe {
        let mut sa: sockaddr_vm = zeroed();
        sa.svm_family = AF_VSOCK as _;
        sa.svm_port = 9000;
        sa.svm_cid = 3;
        let fd = socket(AF_VSOCK, SOCK_STREAM, 0);
        connect(
            fd,
            &sa as *const _ as *mut sockaddr,
            size_of::<sockaddr_vm>() as _,
        );
        write(fd, buf.as_ptr() as _, 1);
        read(fd, buf.as_ptr() as _, 1);
        close(fd);
    }
}

// Initialize console with stdin/stdout/stderr
pub fn init_console() {
    freopen("/dev/console", "r", 0);
    freopen("/dev/console", "w", 1);
    freopen("/dev/console", "w", 2);
}

// Mount common filesystems with conservative permissions
pub fn init_rootfs() {
    mount("devtmpfs", "/dev", "devtmpfs", MS_NOSUID | MS_NOEXEC, "mode=0755");
    mount("proc", "/proc", "proc", MS_NODEV | MS_NOSUID | MS_NOEXEC, "hidepid=2");
    mount("tmpfs", "/run", "tmpfs", MS_NODEV | MS_NOSUID | MS_NOEXEC, "mode=0755");
    mount("tmpfs", "/tmp", "tmpfs", MS_NODEV | MS_NOSUID | MS_NOEXEC, "");
    mount("shm", "/dev/shm", "tmpfs", MS_NODEV | MS_NOSUID | MS_NOEXEC, "mode=0755");
    mount("devpts", "/dev/pts", "devpts", MS_NOSUID | MS_NOEXEC, "");
    mount("sysfs", "/sys", "sysfs", MS_NODEV | MS_NOSUID | MS_NOEXEC, "");
    mount("cgroup_root", "/sys/fs/cgroup", "tmpfs", MS_NODEV | MS_NOSUID | MS_NOEXEC, "mode=0755");
}

fn main() {
    init_rootfs();
    init_console();
    heartbeat();
    info("EnclaveOS Booted".to_string());
    unsafe {
        reboot(RB_AUTOBOOT);
    }
}
