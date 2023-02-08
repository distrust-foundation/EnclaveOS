use libc::{c_int, c_ulong};
use std::{ffi::CString, fs::File, os::unix::io::AsRawFd};

use crate::error::{error, Context, Result};

/// When calling a libc function, determine whether the return value of that function is an error,
/// and if so, wrap into an error using the `libc::strerror` function. This assumes that the return
/// value is -1 on failure.
///
/// # Errors
///
/// This function returns a Result<T> to convert the given potentially invalid value to an Error.
pub fn strerror(input: c_int) -> Result<c_int> {
    use libc::strerror_r;
    let errno = std::io::Error::last_os_error().raw_os_error().unwrap();
    if input == -1 {
        let mut buf = [0; 128];
        let size = 128;
        let result = unsafe { strerror_r(errno, buf.as_mut_ptr().cast(), size) };
        if result == 0 {
            // find index of 0 byte
            let position = buf.iter().position(|&v| v == 0).unwrap_or(buf.len());
            match CString::new(&buf[..position]).map(CString::into_string) {
                Ok(Ok(error_message)) => Err(error!("{error_message}")),
                Err(e) => Err(error!("Unable to retrieve error: {errno} ({e})")),
                Ok(Err(e)) => Err(error!("Badly retrieved error: {errno} ({e})")),

            }
        } else {
            Err(error!("Unknown error: {errno}"))
        }
    } else {
        Ok(input)
    }
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
pub fn mount(src: &str, target: &str, fstype: &str, flags: c_ulong, data: &str) -> Result<()> {
    use libc::mount;
    let src_cs = CString::new(src)?;
    let fstype_cs = CString::new(fstype)?;
    let data_cs = CString::new(data)?;
    let target_cs = CString::new(target)?;

    strerror(unsafe {
        mount(
            src_cs.as_ptr(),
            target_cs.as_ptr(),
            fstype_cs.as_ptr(),
            flags,
            data_cs.as_ptr().cast(),
        )
    })
    .with_context(|| format!("Failed to mount: {target}"))?;
    Ok(())
}

/// Call the `libc::freopen` library function with converted arguments.
///
/// # Errors
///
/// This function will return an error if any of the input strings contain a null byte (a byte with
/// the value of 0), or if the call to `mount(3)` fails.
pub fn freopen(filename: &str, mode: &str, file: c_int) -> Result<()> {
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
        Err(error!("Failed to freopen: {filename}"))
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
pub fn insmod(path: &str) -> Result<()> {
    use libc::{syscall, SYS_finit_module};
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) => return Err(error!("Failed to open kernel module: {path} ({error})")),
    };
    let fd = file.as_raw_fd();
    strerror(unsafe { syscall(SYS_finit_module, fd, &[0u8; 1], 0) }.try_into()?)
        .with_context(|| "failed to insert kernel module: {path}")?;
    Ok(())
}

/// Seed an entropy sample into the kernel randomness pool, generating bytes of a given `size` from
/// the given source-generating function.
///
/// # Errors
///
/// This function will return an error if the source function fails to generate entropy of the
/// given data size, if the random file is unable to open, or if the data is unable to be written
/// to the file.
pub fn seed_entropy(size: usize, source: fn(usize) -> Result<Vec<u8>>) -> Result<usize> {
    use std::{fs::OpenOptions, io::Write};

    let dev_urandom = "/dev/urandom";
    let entropy_sample = source(size)?;

    let mut random_fd = OpenOptions::new()
        .read(true)
        .write(true)
        .open(dev_urandom)
        .with_context(|| format!("Failed to open {dev_urandom}"))?;

    // 5.10+ kernel entropy pools are made of BLAKE2 hashes fixed at 256 bit
    // The RNDADDENTROPY crediting system is now complexity with no gain.
    // We just simply write samples to /dev/urandom now.
    // See: https://cdn.kernel.org/pub/linux/kernel/v5.x/ChangeLog-5.10.119
    random_fd
        .write_all(&entropy_sample)
        .with_context(|| format!("Failed to write to {dev_urandom}"))?;
    Ok(entropy_sample.len())
}
