use libc::{
    c_ulong,
    c_int,
    c_void,
    MS_NOSUID,
    MS_NOEXEC,
    MS_NODEV,
};
use std::{
    mem::zeroed,
    mem::size_of,
    ffi::CString,
    fs::{File, read_to_string},
    os::unix::io::AsRawFd,
    fmt,
};

struct SystemError {
    message: String,
}
impl fmt::Display for SystemError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", boot_time(), self.message)
    }
}

// Log dmesg formatted log to console
fn dmesg(message: String){
    println!("{} {}", boot_time(), message);
}

// Dmesg formatted seconds since boot
fn boot_time() -> String {
    use libc::{clock_gettime, timespec, CLOCK_BOOTTIME};
    let mut t = timespec { tv_sec: 0, tv_nsec: 0 };
    unsafe { clock_gettime(CLOCK_BOOTTIME, &mut t as *mut timespec); }
    format!("[ {: >4}.{}]", t.tv_sec, t.tv_nsec / 1000).to_string()
}

fn reboot(){
    use libc::{reboot, RB_AUTOBOOT};
    unsafe {
        reboot(RB_AUTOBOOT);
    }
}

// libc::mount casting/error wrapper
fn mount(
    src: &str,
    target: &str,
    fstype: &str,
    flags: c_ulong,
    data: &str,
) -> Result<(), SystemError> {
    use libc::mount;
    let src_cs = CString::new(src).unwrap();
    let fstype_cs = CString::new(fstype).unwrap();
    let data_cs = CString::new(data).unwrap();
    let target_cs = CString::new(target).unwrap();
    if unsafe {
        mount(
            src_cs.as_ptr(),
            target_cs.as_ptr(),
            fstype_cs.as_ptr(),
            flags,
            data_cs.as_ptr() as *const c_void
        )
    } != 0 {
        Err(SystemError { message: format!("Failed to mount: {}", target) })
    } else {
        Ok(())
    }
}

// libc::freopen casting/error wrapper
fn freopen(
    filename: &str,
    mode: &str,
    file: c_int,
) -> Result<(), SystemError> {
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
        Err(SystemError { message: format!("Failed to freopen: {}", filename) })
    } else {
        Ok(())
    }
}

// Insert kernel module into memory
fn insmod(path: &str) -> Result<(), SystemError> {
    use libc::{syscall, SYS_finit_module};
    let file = File::open(path).unwrap();
    let fd = file.as_raw_fd();
    if unsafe { syscall(SYS_finit_module, fd, &[0u8; 1], 0) } < 0 {
        Err(SystemError {
            message: format!("Failed to insert kernel module: {}", path)
        })
    } else {
        Ok(())
    }
}

// Instantiate a socket
fn socket_connect(
    family: c_int,
    port: u32,
    cid: u32,
) -> Result<c_int, SystemError> {
    use libc::{connect, socket, sockaddr, sockaddr_vm, SOCK_STREAM};
    let fd = unsafe { socket(family, SOCK_STREAM, 0) };
    if unsafe {
        let mut sa: sockaddr_vm = zeroed();
        sa.svm_family = family as _;
        sa.svm_port = port;
        sa.svm_cid = cid;
        connect(
            fd,
            &sa as *const _ as *mut sockaddr,
            size_of::<sockaddr_vm>() as _,
        )
    } < 0 {
        Err(SystemError {
            message: format!("Failed to connect to socket: {}", family)
        })
    } else {
        Ok(fd)
    }
}

// Signal to Nitro hypervisor that booting was successful
fn nitro_heartbeat() {
    use libc::{write, read, close, AF_VSOCK};
    let mut buf: [u8; 1] = [0; 1];
    buf[0] = 0xB7; // AWS Nitro heartbeat value
    let fd = match socket_connect(AF_VSOCK, 9000, 3) {
        Ok(f)=> f,
        Err(e)=> {
            eprintln!("{}", e);
            return
        },
    };
    unsafe {
        write(fd, buf.as_ptr() as _, 1);
        read(fd, buf.as_ptr() as _, 1);
        close(fd);
    }
    dmesg(format!("Sent NSM heartbeat"));
}

// Get entropy sample from Nitro device
fn nitro_get_entropy() -> Result<[u8; 256], SystemError> {
    use nsm_api::api::ErrorCode;
    use nsm_lib::{nsm_get_random, nsm_lib_init};

    let nsm_fd = nsm_lib_init();
    if nsm_fd < 0 {
        return Err(SystemError {
            message: String::from("Failed to connect to NSM device")
        });
    };

    let mut dest = [0u8; 256];
    let mut dest_len = dest.len();

    let status = unsafe {
        nsm_get_random(nsm_fd, dest.as_mut_ptr(), &mut dest_len)
    };
    match status {
        ErrorCode::Success => {
            Ok(dest)
        },
        _ => Err(SystemError {
            message: String::from("Failed to get entropy from NSM device")
        })
    }
}

fn get_random_poolsize() -> Result<usize, SystemError> {
    let ps_path = "/proc/sys/kernel/random/poolsize";
    let size_s = read_to_string(ps_path).unwrap_or_else(|_| String::new());
    if size_s.is_empty(){
		return Err(SystemError {
            message: String::from("Failed to read kernel random poolsize"),
        })
    };
	match size_s.parse::<usize>() {
		Ok(size) => Ok(size),
		Err(_) => Err(SystemError {
            message: String::from("Failed to parse kernel random poolsize"),
        }),
    }
}

// Initialize nitro device
fn init_nitro(){
    // TODO: error handling
    nitro_heartbeat();

	match insmod("/nsm.ko") {
        Ok(())=> dmesg(format!("Loaded nsm.ko")),
        Err(e)=> eprintln!("{}", e)
    };
	match get_random_poolsize() {
        Ok(size)=> dmesg(format!("Kernel entropy pool size: {}", size)),
        Err(e)=> eprintln!("{}", e)
    };
    match nitro_get_entropy() {
        Ok(_)=> dmesg(format!("Got NSM Entropy sample")),
        Err(e)=> eprintln!("{}", e)
    };
}

// Initialize console with stdin/stdout/stderr
fn init_console() {
    let args = [
        ("/dev/console", "r", 0),
        ("/dev/console", "w", 1),
        ("/dev/console", "w", 2),
    ];
    for (filename, mode, file) in args {
        match freopen(filename, mode, file) {
            Ok(())=> {},
            Err(e)=> eprintln!("{}", e),
        }
    }
}

// Mount common filesystems with conservative permissions
fn init_rootfs() {
    let args = [
        ("devtmpfs", "/dev", "devtmpfs", MS_NOSUID | MS_NOEXEC, "mode=0755"),
        ("devtmpfs", "/dev", "devtmpfs", MS_NOSUID | MS_NOEXEC, "mode=0755"),
        ("proc", "/proc", "proc", MS_NODEV | MS_NOSUID | MS_NOEXEC, "hidepid=2"),
        ("tmpfs", "/run", "tmpfs", MS_NODEV | MS_NOSUID | MS_NOEXEC, "mode=0755"),
        ("tmpfs", "/tmp", "tmpfs", MS_NODEV | MS_NOSUID | MS_NOEXEC, ""),
        ("shm", "/dev/shm", "tmpfs", MS_NODEV | MS_NOSUID | MS_NOEXEC, "mode=0755"),
        ("devpts", "/dev/pts", "devpts", MS_NOSUID | MS_NOEXEC, ""),
        ("sysfs", "/sys", "sysfs", MS_NODEV | MS_NOSUID | MS_NOEXEC, ""),
        ("cgroup_root", "/sys/fs/cgroup", "tmpfs", MS_NODEV | MS_NOSUID | MS_NOEXEC, "mode=0755"),
    ];
    for (src, target, fstype, flags, data) in args {
        match mount(src, target, fstype, flags, data) {
            Ok(())=> dmesg(format!("Mounted {}", target)),
            Err(e)=> eprintln!("{}", e),
        }
    }
}

fn boot(){
    init_rootfs();
    init_console();
    init_nitro();
}

fn main() {
    boot();
    dmesg("EnclaveOS Booted".to_string());
    reboot();
}
