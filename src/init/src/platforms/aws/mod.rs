use crate::sys::{insmod, SystemError};

mod sys;

/// Get entropy sample from Nitro hardware.
///
/// # Errors
///
/// This function returns an error if the `nsm_lib::nsm_lib_init` function or
/// `nsm_lib::nsm_get_random` function fails.
pub fn get_entropy(size: usize) -> Result<Vec<u8>, SystemError> {
    use nsm_api::api::ErrorCode;
    use nsm_lib::{nsm_get_random, nsm_lib_init};
    let nsm_fd = nsm_lib_init();
    if nsm_fd < 0 {
        return Err(SystemError {
            message: String::from("Failed to connect to NSM device"),
        });
    };
    let mut dest = Vec::with_capacity(size);
    while dest.len() < size {
        let mut buf = [0u8; 256];
        let mut buf_len = buf.len();
        let status = unsafe { nsm_get_random(nsm_fd, buf.as_mut_ptr(), &mut buf_len) };
        match status {
            ErrorCode::Success => {
                dest.extend_from_slice(&buf);
            }
            _ => {
                return Err(SystemError {
                    message: String::from("Failed to get entropy from NSM device"),
                });
            }
        };
    }
    Ok(dest)
}

/// Initialize nitro device by signaling a nitro heartbeat and inserting the nsm.ko kernel module.
pub fn init_platform() -> Result<(), SystemError> {

    sys::nitro_heartbeat()?;
    insmod("/nsm.ko")?;

    Ok(())
}
