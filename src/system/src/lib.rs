use libc::{c_int, c_ulong};
use std::{
    ffi::CString,
    fmt,
    fs::File,
    mem::size_of,
    os::unix::io::AsRawFd,
};

pub struct SystemError {
    pub message: String,
}

impl fmt::Display for SystemError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.message.as_str())
    }
}

impl From<std::ffi::NulError> for SystemError {
    fn from(value: std::ffi::NulError) -> Self {
        SystemError {
            message: value.to_string(),
        }
    }
}

/// Print dmesg formatted log to standard output
pub fn dmesg(message: impl std::fmt::Display) {
    println!("{} {}", boot_time(), message);
}

/// Print dmesg formatted error message to standard error
pub fn dmesg_err(message: impl std::fmt::Display) {
    eprintln!("{} {}", boot_time(), message);
}

/// Dmesg-formatted seconds since boot
#[allow(clippy::must_use_candidate)]
pub fn boot_time() -> String {
    use libc::{clock_gettime, timespec, CLOCK_BOOTTIME};
    let mut t = timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    unsafe {
        clock_gettime(CLOCK_BOOTTIME, std::ptr::addr_of_mut!(t));
    }
    format!("[ {: >4}.{}]", t.tv_sec, t.tv_nsec / 1000)
}

/// Unconditionally reboot the system immediately
pub fn reboot() {
    use libc::{reboot, RB_AUTOBOOT};
    unsafe {
        reboot(RB_AUTOBOOT);
    }
}

/// Call the `libc::mount` system call with converted arguments.
///
/// # Errors
///
/// This function will return an error if any of the input strings contain a null byte (a byte with
/// the value of 0), or if the call to `mount(2)` fails.
pub fn mount(
    src: &str,
    target: &str,
    fstype: &str,
    flags: c_ulong,
    data: &str,
) -> Result<(), SystemError> {
    use libc::mount;
    let src_cs = CString::new(src)?;
    let fstype_cs = CString::new(fstype)?;
    let data_cs = CString::new(data)?;
    let target_cs = CString::new(target)?;
    #[allow(clippy::if_not_else)]
    if unsafe {
        mount(
            src_cs.as_ptr(),
            target_cs.as_ptr(),
            fstype_cs.as_ptr(),
            flags,
            data_cs.as_ptr().cast(),
        )
    } != 0
    {
        Err(SystemError {
            message: format!("Failed to mount: {target}"),
        })
    } else {
        Ok(())
    }
}

/// Call the `libc::freopen` library function with converted arguments.
///
/// # Errors
///
/// This function will return an error if any of the input strings contain a null byte (a byte with
/// the value of 0), or if the call to `mount(3)` fails.
pub fn freopen(filename: &str, mode: &str, file: c_int) -> Result<(), SystemError> {
    use libc::{fdopen, freopen};
    let filename_cs = CString::new(filename)?;
    let mode_cs = CString::new(mode)?;
    if unsafe {
        freopen(
            filename_cs.as_ptr(),
            mode_cs.as_ptr(),
            fdopen(file, mode_cs.as_ptr().cast()),
        )
    }
    .is_null()
    {
        Err(SystemError {
            message: format!("Failed to freopen: {filename}"),
        })
    } else {
        Ok(())
    }
}

/// Insert a kernel module located at the given path into memory.
///
/// # Errors
///
/// This function returns an error when the kernel module's file is unable to be opened or when the
/// kernel module is unable to be inserted.
pub fn insmod(path: &str) -> Result<(), SystemError> {
    use libc::{syscall, SYS_finit_module};
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) => {
            return Err(SystemError {
                message: format!("Failed to open kernel module: {path} ({error})"),
            })
        }
    };
    let fd = file.as_raw_fd();
    if unsafe { syscall(SYS_finit_module, fd, &[0u8; 1], 0) } < 0 {
        Err(SystemError {
            message: format!("Failed to insert kernel module: {path}"),
        })
    } else {
        Ok(())
    }
}

/// Instantiate a socket with the given family, port, and context identifier.
///
/// # Errors
///
/// This function returns an error if the `libc::connect` system call fails.
pub fn socket_connect(
    family: libc::sa_family_t,
    port: u32,
    cid: u32,
) -> Result<c_int, SystemError> {
    use libc::{connect, sockaddr_vm, socket, SOCK_STREAM};
    let fd = unsafe { socket(c_int::from(family), SOCK_STREAM, 0) };
    if unsafe {
        let sa = sockaddr_vm {
            svm_family: family,
            svm_reserved1: Default::default(),
            svm_port: port,
            svm_cid: cid,
            svm_zero: Default::default(),
        };
        connect(
            fd,
            std::ptr::addr_of!(sa).cast(),
            libc::socklen_t::try_from(size_of::<sockaddr_vm>())
                .expect("sizeof sockaddr_vm is larger than socklen_t"),
        )
    } < 0
    {
        Err(SystemError {
            message: format!(
                "Failed to connect to socket: family={family}, port={port}, cid={cid}"
            ),
        })
    } else {
        Ok(fd)
    }
}

/// Seed an entropy sample into the kernel randomness pool, generating bytes of a given `size` from
/// the given source-generating function.
///
/// # Errors
///
/// This function will return an error if the source function fails to generate entropy of the
/// given data size, if the random file is unable to open, or if the data is unable to be written
/// to the file.
pub fn seed_entropy(
    size: usize,
    source: fn(usize) -> Result<Vec<u8>, SystemError>,
) -> Result<usize, SystemError> {
    use std::{fs::OpenOptions, io::Write};

    let entropy_sample = source(size)?;

    let mut random_fd = match OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/urandom") {
            Ok(fd) => fd,
            Err(e) => return Err(SystemError {
                message: format!("Failed to open /dev/urandom: {e}"),
            })
        };

    // 5.10+ kernel entropy pools are made of BLAKE2 hashes fixed at 256 bit
    // The RNDADDENTROPY crediting system is now complexity with no gain.
    // We just simply write samples to /dev/urandom now.
    // See: https://cdn.kernel.org/pub/linux/kernel/v5.x/ChangeLog-5.10.119
    match random_fd.write_all(&entropy_sample) {
        Ok(()) => Ok(entropy_sample.len()),
        Err(e) => Err(SystemError {
            message: format!("Failed to write to /dev/urandom: {e}"),
        }),
    }
}
