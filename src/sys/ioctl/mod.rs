//! Provide helpers for making ioctl system calls.
//!
//! This library is pretty low-level and messy. `ioctl` is not fun.
//!
//! What is an `ioctl`?
//! ===================
//!
//! The `ioctl` syscall is the grab-bag syscall on POSIX systems. Don't want to add a new
//! syscall? Make it an `ioctl`! `ioctl` refers to both the syscall, and the commands that can be
//! sent with it. `ioctl` stands for "IO control", and the commands are always sent to a file
//! descriptor.
//!
//! It is common to see `ioctl`s used for the following purposes:
//!
//!   * Provide read/write access to out-of-band data related to a device such as configuration
//!     (for instance, setting serial port options)
//!   * Provide a mechanism for performing full-duplex data transfers (for instance, xfer on SPI
//!     devices).
//!   * Provide access to control functions on a device (for example, on Linux you can send
//!     commands like pause, resume, and eject to the CDROM device.
//!   * Do whatever else the device driver creator thought made most sense.
//!
//! `ioctl`s are synchronous system calls and are similar to read and write calls in that regard.
//! They operate on file descriptors and have an identifier that specifies what the ioctl is.
//! Additionally they may read or write data and therefore need to pass along a data pointer.
//! Besides the semantics of the ioctls being confusing, the generation of this identifer can also
//! be difficult.
//!
//! Historically `ioctl` numbers were arbitrary hard-coded values. In Linux (before 2.6) and some
//! unices this has changed to a more-ordered system where the ioctl numbers are partitioned into
//! subcomponents (For linux this is documented in
//! [`Documentation/ioctl/ioctl-number.rst`](https://elixir.bootlin.com/linux/latest/source/Documentation/userspace-api/ioctl/ioctl-number.rst)):
//!
//!   * Number: The actual ioctl ID
//!   * Type: A grouping of ioctls for a common purpose or driver
//!   * Size: The size in bytes of the data that will be transferred
//!   * Direction: Whether there is any data and if it's read, write, or both
//!
//! Newer drivers should not generate complete integer identifiers for their `ioctl`s instead
//! preferring to use the 4 components above to generate the final ioctl identifier. Because of
//! how old `ioctl`s are, however, there are many hard-coded `ioctl` identifiers. These are
//! commonly referred to as "bad" in `ioctl` documentation.
//!
//! Defining `ioctl`s
//! =================
//!
//! This library provides several `ioctl_*!` macros for binding `ioctl`s. These generate public
//! unsafe functions that can then be used for calling the ioctl. This macro has a few different
//! ways it can be used depending on the specific ioctl you're working with.
//!
//! A simple `ioctl` is `SPI_IOC_RD_MODE`. This ioctl works with the SPI interface on Linux. This
//! specific `ioctl` reads the mode of the SPI device as a `u8`. It's declared in
//! `/include/uapi/linux/spi/spidev.h` as `_IOR(SPI_IOC_MAGIC, 1, __u8)`. Since it uses the `_IOR`
//! macro, we know it's a `read` ioctl and can use the `ioctl_read!` macro as follows:
//!
//! ```
//! # #[macro_use] extern crate nix;
//! const SPI_IOC_MAGIC: u8 = b'k'; // Defined in linux/spi/spidev.h
//! const SPI_IOC_TYPE_MODE: u8 = 1;
//! ioctl_read!(spi_read_mode, SPI_IOC_MAGIC, SPI_IOC_TYPE_MODE, u8);
//! # fn main() {}
//! ```
//!
//! This generates the function:
//!
//! ```
//! # #[macro_use] extern crate nix;
//! # use std::mem;
//! # use nix::{libc, Result};
//! # use nix::errno::Errno;
//! # use nix::libc::c_int as c_int;
//! # const SPI_IOC_MAGIC: u8 = b'k'; // Defined in linux/spi/spidev.h
//! # const SPI_IOC_TYPE_MODE: u8 = 1;
//! pub unsafe fn spi_read_mode(fd: c_int, data: *mut u8) -> Result<c_int> {
//!     let res = unsafe { libc::ioctl(fd, request_code_read!(SPI_IOC_MAGIC, SPI_IOC_TYPE_MODE, mem::size_of::<u8>()), data) };
//!     Errno::result(res)
//! }
//! # fn main() {}
//! ```
//!
//! The return value for the wrapper functions generated by the `ioctl_*!` macros are `nix::Error`s.
//! These are generated by assuming the return value of the ioctl is `-1` on error and everything
//! else is a valid return value. If this is not the case, `Result::map` can be used to map some
//! of the range of "good" values (-Inf..-2, 0..Inf) into a smaller range in a helper function.
//!
//! Writing `ioctl`s generally use pointers as their data source and these should use the
//! `ioctl_write_ptr!`. But in some cases an `int` is passed directly. For these `ioctl`s use the
//! `ioctl_write_int!` macro. This variant does not take a type as the last argument:
//!
//! ```
//! # #[macro_use] extern crate nix;
//! const HCI_IOC_MAGIC: u8 = b'k';
//! const HCI_IOC_HCIDEVUP: u8 = 1;
//! ioctl_write_int!(hci_dev_up, HCI_IOC_MAGIC, HCI_IOC_HCIDEVUP);
//! # fn main() {}
//! ```
//!
//! Some `ioctl`s don't transfer any data, and those should use `ioctl_none!`. This macro
//! doesn't take a type and so it is declared similar to the `write_int` variant shown above.
//!
//! The mode for a given `ioctl` should be clear from the documentation if it has good
//! documentation. Otherwise it will be clear based on the macro used to generate the `ioctl`
//! number where `_IO`, `_IOR`, `_IOW`, and `_IOWR` map to "none", "read", "write_*", and "readwrite"
//! respectively. To determine the specific `write_` variant to use you'll need to find
//! what the argument type is supposed to be. If it's an `int`, then `write_int` should be used,
//! otherwise it should be a pointer and `write_ptr` should be used. On Linux the
//! [`ioctl_list` man page](https://man7.org/linux/man-pages/man2/ioctl_list.2.html) describes a
//! large number of `ioctl`s and describes their argument data type.
//!
//! Using "bad" `ioctl`s
//! --------------------
//!
//! As mentioned earlier, there are many old `ioctl`s that do not use the newer method of
//! generating `ioctl` numbers and instead use hardcoded values. These can be used with the
//! `ioctl_*_bad!` macros. This naming comes from the Linux kernel which refers to these
//! `ioctl`s as "bad". These are a different variant as they bypass calling the macro that generates
//! the ioctl number and instead use the defined value directly.
//!
//! For example the `TCGETS` `ioctl` reads a `termios` data structure for a given file descriptor.
//! It's defined as `0x5401` in `ioctls.h` on Linux and can be implemented as:
//!
//! ```
//! # #[macro_use] extern crate nix;
//! # #[cfg(linux_android)]
//! # use nix::libc::TCGETS as TCGETS;
//! # #[cfg(linux_android)]
//! # use nix::libc::termios as termios;
//! # #[cfg(linux_android)]
//! ioctl_read_bad!(tcgets, TCGETS, termios);
//! # fn main() {}
//! ```
//!
//! The generated function has the same form as that generated by `ioctl_read!`:
//!
//! ```text
//! pub unsafe fn tcgets(fd: c_int, data: *mut termios) -> Result<c_int>;
//! ```
//!
//! Working with Arrays
//! -------------------
//!
//! Some `ioctl`s work with entire arrays of elements. These are supported by the `ioctl_*_buf`
//! family of macros: `ioctl_read_buf`, `ioctl_write_buf`, and `ioctl_readwrite_buf`. Note that
//! there are no "bad" versions for working with buffers. The generated functions include a `len`
//! argument to specify the number of elements (where the type of each element is specified in the
//! macro).
//!
//! Again looking to the SPI `ioctl`s on Linux for an example, there is a `SPI_IOC_MESSAGE` `ioctl`
//! that queues up multiple SPI messages by writing an entire array of `spi_ioc_transfer` structs.
//! `linux/spi/spidev.h` defines a macro to calculate the `ioctl` number like:
//!
//! ```C
//! #define SPI_IOC_MAGIC 'k'
//! #define SPI_MSGSIZE(N) ...
//! #define SPI_IOC_MESSAGE(N) _IOW(SPI_IOC_MAGIC, 0, char[SPI_MSGSIZE(N)])
//! ```
//!
//! The `SPI_MSGSIZE(N)` calculation is already handled by the `ioctl_*!` macros, so all that's
//! needed to define this `ioctl` is:
//!
//! ```
//! # #[macro_use] extern crate nix;
//! const SPI_IOC_MAGIC: u8 = b'k'; // Defined in linux/spi/spidev.h
//! const SPI_IOC_TYPE_MESSAGE: u8 = 0;
//! # pub struct spi_ioc_transfer(u64);
//! ioctl_write_buf!(spi_transfer, SPI_IOC_MAGIC, SPI_IOC_TYPE_MESSAGE, spi_ioc_transfer);
//! # fn main() {}
//! ```
//!
//! This generates a function like:
//!
//! ```
//! # #[macro_use] extern crate nix;
//! # use std::mem;
//! # use nix::{libc, Result};
//! # use nix::errno::Errno;
//! # use nix::libc::c_int as c_int;
//! # const SPI_IOC_MAGIC: u8 = b'k';
//! # const SPI_IOC_TYPE_MESSAGE: u8 = 0;
//! # pub struct spi_ioc_transfer(u64);
//! pub unsafe fn spi_message(fd: c_int, data: &mut [spi_ioc_transfer]) -> Result<c_int> {
//!     let res = unsafe {
//!         libc::ioctl(
//!             fd,
//!             request_code_write!(SPI_IOC_MAGIC, SPI_IOC_TYPE_MESSAGE, data.len() * mem::size_of::<spi_ioc_transfer>()),
//!             data
//!         )
//!     };
//!     Errno::result(res)
//! }
//! # fn main() {}
//! ```
//!
//! Finding `ioctl` Documentation
//! -----------------------------
//!
//! For Linux, look at your system's headers. For example, `/usr/include/linux/input.h` has a lot
//! of lines defining macros which use `_IO`, `_IOR`, `_IOW`, `_IOC`, and `_IOWR`. Some `ioctl`s are
//! documented directly in the headers defining their constants, but others have more extensive
//! documentation in man pages (like termios' `ioctl`s which are in `tty_ioctl(4)`).
//!
//! Documenting the Generated Functions
//! ===================================
//!
//! In many cases, users will wish for the functions generated by the `ioctl`
//! macro to be public and documented. For this reason, the generated functions
//! are public by default. If you wish to hide the ioctl, you will need to put
//! them in a private module.
//!
//! For documentation, it is possible to use doc comments inside the `ioctl_*!` macros. Here is an
//! example :
//!
//! ```
//! # #[macro_use] extern crate nix;
//! # use nix::libc::c_int;
//! ioctl_read! {
//!     /// Make the given terminal the controlling terminal of the calling process. The calling
//!     /// process must be a session leader and not have a controlling terminal already. If the
//!     /// terminal is already the controlling terminal of a different session group then the
//!     /// ioctl will fail with **EPERM**, unless the caller is root (more precisely: has the
//!     /// **CAP_SYS_ADMIN** capability) and arg equals 1, in which case the terminal is stolen
//!     /// and all processes that had it as controlling terminal lose it.
//!     tiocsctty, b't', 19, c_int
//! }
//!
//! # fn main() {}
//! ```
use cfg_if::cfg_if;

#[cfg(any(linux_android, target_os = "fuchsia", target_os = "redox"))]
#[macro_use]
mod linux;

#[cfg(any(linux_android, target_os = "fuchsia", target_os = "redox"))]
pub use self::linux::*;

#[cfg(any(bsd, solarish, target_os = "haiku",))]
#[macro_use]
mod bsd;

#[cfg(any(bsd, solarish, target_os = "haiku",))]
pub use self::bsd::*;

/// Convert raw ioctl return value to a Nix result
#[macro_export]
#[doc(hidden)]
macro_rules! convert_ioctl_res {
    ($w:expr) => {{
        $crate::errno::Errno::result($w)
    }};
}

/// Generates a wrapper function for an ioctl that passes no data to the kernel.
///
/// The arguments to this macro are:
///
/// * The function name
/// * The ioctl identifier
/// * The ioctl sequence number
///
/// The generated function has the following signature:
///
/// ```rust,ignore
/// pub unsafe fn FUNCTION_NAME(fd: libc::c_int) -> Result<libc::c_int>
/// ```
///
/// For a more in-depth explanation of ioctls, see [`::sys::ioctl`](sys/ioctl/index.html).
///
/// # Example
///
/// The `videodev2` driver on Linux defines the `log_status` `ioctl` as:
///
/// ```C
/// #define VIDIOC_LOG_STATUS         _IO('V', 70)
/// ```
///
/// This can be implemented in Rust like:
///
/// ```no_run
/// # #[macro_use] extern crate nix;
/// ioctl_none!(log_status, b'V', 70);
/// fn main() {}
/// ```
#[macro_export(local_inner_macros)]
macro_rules! ioctl_none {
    ($(#[$attr:meta])* $name:ident, $ioty:expr, $nr:expr) => (
        $(#[$attr])*
        pub unsafe fn $name(fd: $crate::libc::c_int)
                            -> $crate::Result<$crate::libc::c_int> {
            unsafe {
                convert_ioctl_res!($crate::libc::ioctl(fd, request_code_none!($ioty, $nr) as $crate::sys::ioctl::ioctl_num_type))
            }
        }
    )
}

/// Generates a wrapper function for a "bad" ioctl that passes no data to the kernel.
///
/// The arguments to this macro are:
///
/// * The function name
/// * The ioctl request code
///
/// The generated function has the following signature:
///
/// ```rust,ignore
/// pub unsafe fn FUNCTION_NAME(fd: libc::c_int) -> Result<libc::c_int>
/// ```
///
/// For a more in-depth explanation of ioctls, see [`::sys::ioctl`](sys/ioctl/index.html).
///
/// # Example
///
/// ```no_run
/// # #[macro_use] extern crate nix;
/// # use libc::TIOCNXCL;
/// # use std::fs::File;
/// # use std::os::unix::io::AsRawFd;
/// ioctl_none_bad!(tiocnxcl, TIOCNXCL);
/// fn main() {
///     let file = File::open("/dev/ttyUSB0").unwrap();
///     unsafe { tiocnxcl(file.as_raw_fd()) }.unwrap();
/// }
/// ```
// TODO: add an example using request_code_*!()
#[macro_export(local_inner_macros)]
macro_rules! ioctl_none_bad {
    ($(#[$attr:meta])* $name:ident, $nr:expr) => (
        $(#[$attr])*
        pub unsafe fn $name(fd: $crate::libc::c_int)
                            -> $crate::Result<$crate::libc::c_int> {
            unsafe {
                convert_ioctl_res!($crate::libc::ioctl(fd, $nr as $crate::sys::ioctl::ioctl_num_type))
            }
        }
    )
}

/// Generates a wrapper function for an ioctl that reads data from the kernel.
///
/// The arguments to this macro are:
///
/// * The function name
/// * The ioctl identifier
/// * The ioctl sequence number
/// * The data type passed by this ioctl
///
/// The generated function has the following signature:
///
/// ```rust,ignore
/// pub unsafe fn FUNCTION_NAME(fd: libc::c_int, data: *mut DATA_TYPE) -> Result<libc::c_int>
/// ```
///
/// For a more in-depth explanation of ioctls, see [`::sys::ioctl`](sys/ioctl/index.html).
///
/// # Example
///
/// ```
/// # #[macro_use] extern crate nix;
/// const SPI_IOC_MAGIC: u8 = b'k'; // Defined in linux/spi/spidev.h
/// const SPI_IOC_TYPE_MODE: u8 = 1;
/// ioctl_read!(spi_read_mode, SPI_IOC_MAGIC, SPI_IOC_TYPE_MODE, u8);
/// # fn main() {}
/// ```
#[macro_export(local_inner_macros)]
macro_rules! ioctl_read {
    ($(#[$attr:meta])* $name:ident, $ioty:expr, $nr:expr, $ty:ty) => (
        $(#[$attr])*
        pub unsafe fn $name(fd: $crate::libc::c_int,
                            data: *mut $ty)
                            -> $crate::Result<$crate::libc::c_int> {
            unsafe {
                convert_ioctl_res!($crate::libc::ioctl(fd, request_code_read!($ioty, $nr, ::std::mem::size_of::<$ty>()) as $crate::sys::ioctl::ioctl_num_type, data))
            }
        }
    )
}

/// Generates a wrapper function for a "bad" ioctl that reads data from the kernel.
///
/// The arguments to this macro are:
///
/// * The function name
/// * The ioctl request code
/// * The data type passed by this ioctl
///
/// The generated function has the following signature:
///
/// ```rust,ignore
/// pub unsafe fn FUNCTION_NAME(fd: libc::c_int, data: *mut DATA_TYPE) -> Result<libc::c_int>
/// ```
///
/// For a more in-depth explanation of ioctls, see [`::sys::ioctl`](sys/ioctl/index.html).
///
/// # Example
///
/// ```
/// # #[macro_use] extern crate nix;
/// # #[cfg(linux_android)]
/// ioctl_read_bad!(tcgets, libc::TCGETS, libc::termios);
/// # fn main() {}
/// ```
#[macro_export(local_inner_macros)]
macro_rules! ioctl_read_bad {
    ($(#[$attr:meta])* $name:ident, $nr:expr, $ty:ty) => (
        $(#[$attr])*
        pub unsafe fn $name(fd: $crate::libc::c_int,
                            data: *mut $ty)
                            -> $crate::Result<$crate::libc::c_int> {
            unsafe {
                convert_ioctl_res!($crate::libc::ioctl(fd, $nr as $crate::sys::ioctl::ioctl_num_type, data))
            }
        }
    )
}

/// Generates a wrapper function for an ioctl that writes data through a pointer to the kernel.
///
/// The arguments to this macro are:
///
/// * The function name
/// * The ioctl identifier
/// * The ioctl sequence number
/// * The data type passed by this ioctl
///
/// The generated function has the following signature:
///
/// ```rust,ignore
/// pub unsafe fn FUNCTION_NAME(fd: libc::c_int, data: *const DATA_TYPE) -> Result<libc::c_int>
/// ```
///
/// For a more in-depth explanation of ioctls, see [`::sys::ioctl`](sys/ioctl/index.html).
///
/// # Example
///
/// ```
/// # #[macro_use] extern crate nix;
/// # pub struct v4l2_audio {}
/// ioctl_write_ptr!(s_audio, b'V', 34, v4l2_audio);
/// # fn main() {}
/// ```
#[macro_export(local_inner_macros)]
macro_rules! ioctl_write_ptr {
    ($(#[$attr:meta])* $name:ident, $ioty:expr, $nr:expr, $ty:ty) => (
        $(#[$attr])*
        pub unsafe fn $name(fd: $crate::libc::c_int,
                            data: *const $ty)
                            -> $crate::Result<$crate::libc::c_int> {
            unsafe {
                convert_ioctl_res!($crate::libc::ioctl(fd, request_code_write!($ioty, $nr, ::std::mem::size_of::<$ty>()) as $crate::sys::ioctl::ioctl_num_type, data))
            }
        }
    )
}

/// Generates a wrapper function for a "bad" ioctl that writes data through a pointer to the kernel.
///
/// The arguments to this macro are:
///
/// * The function name
/// * The ioctl request code
/// * The data type passed by this ioctl
///
/// The generated function has the following signature:
///
/// ```rust,ignore
/// pub unsafe fn FUNCTION_NAME(fd: libc::c_int, data: *const DATA_TYPE) -> Result<libc::c_int>
/// ```
///
/// For a more in-depth explanation of ioctls, see [`::sys::ioctl`](sys/ioctl/index.html).
///
/// # Example
///
/// ```
/// # #[macro_use] extern crate nix;
/// # #[cfg(linux_android)]
/// ioctl_write_ptr_bad!(tcsets, libc::TCSETS, libc::termios);
/// # fn main() {}
/// ```
#[macro_export(local_inner_macros)]
macro_rules! ioctl_write_ptr_bad {
    ($(#[$attr:meta])* $name:ident, $nr:expr, $ty:ty) => (
        $(#[$attr])*
        pub unsafe fn $name(fd: $crate::libc::c_int,
                            data: *const $ty)
                            -> $crate::Result<$crate::libc::c_int> {
            unsafe {
                convert_ioctl_res!($crate::libc::ioctl(fd, $nr as $crate::sys::ioctl::ioctl_num_type, data))
            }
        }
    )
}

cfg_if! {
    if #[cfg(freebsdlike)] {
        /// Generates a wrapper function for a ioctl that writes an integer to the kernel.
        ///
        /// The arguments to this macro are:
        ///
        /// * The function name
        /// * The ioctl identifier
        /// * The ioctl sequence number
        ///
        /// The generated function has the following signature:
        ///
        /// ```rust,ignore
        /// pub unsafe fn FUNCTION_NAME(fd: libc::c_int, data: nix::sys::ioctl::ioctl_param_type) -> Result<libc::c_int>
        /// ```
        ///
        /// `nix::sys::ioctl::ioctl_param_type` depends on the OS:
        /// *   BSD - `libc::c_int`
        /// *   Linux - `libc::c_ulong`
        ///
        /// For a more in-depth explanation of ioctls, see [`::sys::ioctl`](sys/ioctl/index.html).
        ///
        /// # Example
        ///
        /// ```
        /// # #[macro_use] extern crate nix;
        /// ioctl_write_int!(vt_activate, b'v', 4);
        /// # fn main() {}
        /// ```
        #[macro_export(local_inner_macros)]
        macro_rules! ioctl_write_int {
            ($(#[$attr:meta])* $name:ident, $ioty:expr, $nr:expr) => (
                $(#[$attr])*
                pub unsafe fn $name(fd: $crate::libc::c_int,
                                    data: $crate::sys::ioctl::ioctl_param_type)
                                    -> $crate::Result<$crate::libc::c_int> {
                    unsafe {
                        convert_ioctl_res!($crate::libc::ioctl(fd, request_code_write_int!($ioty, $nr) as $crate::sys::ioctl::ioctl_num_type, data))
                    }
                }
            )
        }
    } else {
        /// Generates a wrapper function for a ioctl that writes an integer to the kernel.
        ///
        /// The arguments to this macro are:
        ///
        /// * The function name
        /// * The ioctl identifier
        /// * The ioctl sequence number
        ///
        /// The generated function has the following signature:
        ///
        /// ```rust,ignore
        /// pub unsafe fn FUNCTION_NAME(fd: libc::c_int, data: nix::sys::ioctl::ioctl_param_type) -> Result<libc::c_int>
        /// ```
        ///
        /// `nix::sys::ioctl::ioctl_param_type` depends on the OS:
        /// *   BSD - `libc::c_int`
        /// *   Linux - `libc::c_ulong`
        ///
        /// For a more in-depth explanation of ioctls, see [`::sys::ioctl`](sys/ioctl/index.html).
        ///
        /// # Example
        ///
        /// ```
        /// # #[macro_use] extern crate nix;
        /// const HCI_IOC_MAGIC: u8 = b'k';
        /// const HCI_IOC_HCIDEVUP: u8 = 1;
        /// ioctl_write_int!(hci_dev_up, HCI_IOC_MAGIC, HCI_IOC_HCIDEVUP);
        /// # fn main() {}
        /// ```
        #[macro_export(local_inner_macros)]
        macro_rules! ioctl_write_int {
            ($(#[$attr:meta])* $name:ident, $ioty:expr, $nr:expr) => (
                $(#[$attr])*
                pub unsafe fn $name(fd: $crate::libc::c_int,
                                    data: $crate::sys::ioctl::ioctl_param_type)
                                    -> $crate::Result<$crate::libc::c_int> {
                    unsafe {
                        convert_ioctl_res!($crate::libc::ioctl(fd, request_code_write!($ioty, $nr, ::std::mem::size_of::<$crate::libc::c_int>()) as $crate::sys::ioctl::ioctl_num_type, data))
                    }
                }
            )
        }
    }
}

/// Generates a wrapper function for a "bad" ioctl that writes an integer to the kernel.
///
/// The arguments to this macro are:
///
/// * The function name
/// * The ioctl request code
///
/// The generated function has the following signature:
///
/// ```rust,ignore
/// pub unsafe fn FUNCTION_NAME(fd: libc::c_int, data: libc::c_int) -> Result<libc::c_int>
/// ```
///
/// For a more in-depth explanation of ioctls, see [`::sys::ioctl`](sys/ioctl/index.html).
///
/// # Examples
///
/// ```
/// # #[macro_use] extern crate nix;
/// # #[cfg(linux_android)]
/// ioctl_write_int_bad!(tcsbrk, libc::TCSBRK);
/// # fn main() {}
/// ```
///
/// ```rust
/// # #[macro_use] extern crate nix;
/// const KVMIO: u8 = 0xAE;
/// ioctl_write_int_bad!(kvm_create_vm, request_code_none!(KVMIO, 0x03));
/// # fn main() {}
/// ```
#[macro_export(local_inner_macros)]
macro_rules! ioctl_write_int_bad {
    ($(#[$attr:meta])* $name:ident, $nr:expr) => (
        $(#[$attr])*
        pub unsafe fn $name(fd: $crate::libc::c_int,
                            data: $crate::libc::c_int)
                            -> $crate::Result<$crate::libc::c_int> {
            unsafe {
                convert_ioctl_res!($crate::libc::ioctl(fd, $nr as $crate::sys::ioctl::ioctl_num_type, data))
            }
        }
    )
}

/// Generates a wrapper function for an ioctl that reads and writes data to the kernel.
///
/// The arguments to this macro are:
///
/// * The function name
/// * The ioctl identifier
/// * The ioctl sequence number
/// * The data type passed by this ioctl
///
/// The generated function has the following signature:
///
/// ```rust,ignore
/// pub unsafe fn FUNCTION_NAME(fd: libc::c_int, data: *mut DATA_TYPE) -> Result<libc::c_int>
/// ```
///
/// For a more in-depth explanation of ioctls, see [`::sys::ioctl`](sys/ioctl/index.html).
///
/// # Example
///
/// ```
/// # #[macro_use] extern crate nix;
/// # pub struct v4l2_audio {}
/// ioctl_readwrite!(enum_audio, b'V', 65, v4l2_audio);
/// # fn main() {}
/// ```
#[macro_export(local_inner_macros)]
macro_rules! ioctl_readwrite {
    ($(#[$attr:meta])* $name:ident, $ioty:expr, $nr:expr, $ty:ty) => (
        $(#[$attr])*
        pub unsafe fn $name(fd: $crate::libc::c_int,
                            data: *mut $ty)
                            -> $crate::Result<$crate::libc::c_int> {
            let ioty = $ioty;
            let nr = $nr;
            unsafe {
                convert_ioctl_res!($crate::libc::ioctl(fd, request_code_readwrite!(ioty, nr, ::std::mem::size_of::<$ty>()) as $crate::sys::ioctl::ioctl_num_type, data))
            }
        }
    )
}

/// Generates a wrapper function for a "bad" ioctl that reads and writes data to the kernel.
///
/// The arguments to this macro are:
///
/// * The function name
/// * The ioctl request code
/// * The data type passed by this ioctl
///
/// The generated function has the following signature:
///
/// ```rust,ignore
/// pub unsafe fn FUNCTION_NAME(fd: libc::c_int, data: *mut DATA_TYPE) -> Result<libc::c_int>
/// ```
///
/// For a more in-depth explanation of ioctls, see [`::sys::ioctl`](sys/ioctl/index.html).
// TODO: Find an example for ioctl_readwrite_bad
#[macro_export(local_inner_macros)]
macro_rules! ioctl_readwrite_bad {
    ($(#[$attr:meta])* $name:ident, $nr:expr, $ty:ty) => (
        $(#[$attr])*
        pub unsafe fn $name(fd: $crate::libc::c_int,
                            data: *mut $ty)
                            -> $crate::Result<$crate::libc::c_int> {
            unsafe {
                convert_ioctl_res!($crate::libc::ioctl(fd, $nr as $crate::sys::ioctl::ioctl_num_type, data))
            }
        }
    )
}

/// Generates a wrapper function for an ioctl that reads an array of elements from the kernel.
///
/// The arguments to this macro are:
///
/// * The function name
/// * The ioctl identifier
/// * The ioctl sequence number
/// * The data type passed by this ioctl
///
/// The generated function has the following signature:
///
/// ```rust,ignore
/// pub unsafe fn FUNCTION_NAME(fd: libc::c_int, data: &mut [DATA_TYPE]) -> Result<libc::c_int>
/// ```
///
/// For a more in-depth explanation of ioctls, see [`::sys::ioctl`](sys/ioctl/index.html).
// TODO: Find an example for ioctl_read_buf
#[macro_export(local_inner_macros)]
macro_rules! ioctl_read_buf {
    ($(#[$attr:meta])* $name:ident, $ioty:expr, $nr:expr, $ty:ty) => (
        $(#[$attr])*
        pub unsafe fn $name(fd: $crate::libc::c_int,
                            data: &mut [$ty])
                            -> $crate::Result<$crate::libc::c_int> {
            unsafe {
                convert_ioctl_res!($crate::libc::ioctl(fd, request_code_read!($ioty, $nr, ::std::mem::size_of_val(data)) as $crate::sys::ioctl::ioctl_num_type, data.as_mut_ptr()))
            }
        }
    )
}

/// Generates a wrapper function for an ioctl that writes an array of elements to the kernel.
///
/// The arguments to this macro are:
///
/// * The function name
/// * The ioctl identifier
/// * The ioctl sequence number
/// * The data type passed by this ioctl
///
/// The generated function has the following signature:
///
/// ```rust,ignore
/// pub unsafe fn FUNCTION_NAME(fd: libc::c_int, data: &[DATA_TYPE]) -> Result<libc::c_int>
/// ```
///
/// For a more in-depth explanation of ioctls, see [`::sys::ioctl`](sys/ioctl/index.html).
///
/// # Examples
///
/// ```
/// # #[macro_use] extern crate nix;
/// const SPI_IOC_MAGIC: u8 = b'k'; // Defined in linux/spi/spidev.h
/// const SPI_IOC_TYPE_MESSAGE: u8 = 0;
/// # pub struct spi_ioc_transfer(u64);
/// ioctl_write_buf!(spi_transfer, SPI_IOC_MAGIC, SPI_IOC_TYPE_MESSAGE, spi_ioc_transfer);
/// # fn main() {}
/// ```
#[macro_export(local_inner_macros)]
macro_rules! ioctl_write_buf {
    ($(#[$attr:meta])* $name:ident, $ioty:expr, $nr:expr, $ty:ty) => (
        $(#[$attr])*
        pub unsafe fn $name(fd: $crate::libc::c_int,
                            data: &[$ty])
                            -> $crate::Result<$crate::libc::c_int> {
            unsafe {
                convert_ioctl_res!($crate::libc::ioctl(fd, request_code_write!($ioty, $nr, ::std::mem::size_of_val(data)) as $crate::sys::ioctl::ioctl_num_type, data.as_ptr()))
            }
        }
    )
}

/// Generates a wrapper function for an ioctl that reads and writes an array of elements to the kernel.
///
/// The arguments to this macro are:
///
/// * The function name
/// * The ioctl identifier
/// * The ioctl sequence number
/// * The data type passed by this ioctl
///
/// The generated function has the following signature:
///
/// ```rust,ignore
/// pub unsafe fn FUNCTION_NAME(fd: libc::c_int, data: &mut [DATA_TYPE]) -> Result<libc::c_int>
/// ```
///
/// For a more in-depth explanation of ioctls, see [`::sys::ioctl`](sys/ioctl/index.html).
// TODO: Find an example for readwrite_buf
#[macro_export(local_inner_macros)]
macro_rules! ioctl_readwrite_buf {
    ($(#[$attr:meta])* $name:ident, $ioty:expr, $nr:expr, $ty:ty) => (
        $(#[$attr])*
        pub unsafe fn $name(fd: $crate::libc::c_int,
                            data: &mut [$ty])
                            -> $crate::Result<$crate::libc::c_int> {
            unsafe {
                convert_ioctl_res!($crate::libc::ioctl(fd, request_code_readwrite!($ioty, $nr, ::std::mem::size_of_val(data)) as $crate::sys::ioctl::ioctl_num_type, data.as_mut_ptr()))
            }
        }
    )
}
