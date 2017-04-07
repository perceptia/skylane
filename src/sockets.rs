// Copyright 2016-2017 The Perceptia Project Developers
//
// Permission is hereby granted, free of charge, to any person obtaining a copy of this software
// and associated documentation files (the "Software"), to deal in the Software without
// restriction, including without limitation the rights to use, copy, modify, merge, publish,
// distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the
// Software is furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all copies or
// substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING
// BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
// NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
// DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

//! This module provides functionality for connecting, reading and writing sockets.

use std;
use std::error::Error;
use std::io::Cursor;
use std::os::unix::io::RawFd;

use byteorder::{NativeEndian, WriteBytesExt};

use nix;
use nix::sys::socket;
use nix::sys::uio;

use defs::{Logger, SkylaneError};

// -------------------------------------------------------------------------------------------------

/// Helper macro for creating meaningful error reports.
/// NOTE: Would be nice if `nix` put more information in errors.
macro_rules! try_sock {
    ($action:expr, $path:expr, $expr:expr) => {
        match $expr {
            Ok(result) => result,
            Err(err) => {
                return Err(SkylaneError::Other(
                    format!("{} {:?}: {:?}", $action, $path, err.description())
                ));
            }
        }
    }
}

// -------------------------------------------------------------------------------------------------

/// Returns default server socket path.
///
/// Path is created from system variables: `$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY` or
/// `$XDG_RUNTIME_DIR/wayland-0` if `$WAYLAND_DISPLAY` is not set.
pub fn get_default_socket_path() -> Result<std::path::PathBuf, SkylaneError> {
    let mut path = std::path::PathBuf::from(std::env::var("XDG_RUNTIME_DIR")?);
    if let Ok(sock) = std::env::var("WAYLAND_DISPLAY") {
        path.push(sock);
    } else {
        path.push("wayland-0");
    }
    Ok(path)
}

// -------------------------------------------------------------------------------------------------

/// Structure representing connection between server and client.
#[derive(Clone)]
pub struct Socket {
    fd: RawFd,
    next_serial: std::cell::Cell<u32>,
    logger: Logger,
}

// -------------------------------------------------------------------------------------------------

impl Socket {
    /// Connects to display socket.
    pub fn connect(path: &std::path::Path) -> Result<Self, SkylaneError> {
        let sockfd = try_sock!("Creating",
                               path,
                               socket::socket(socket::AddressFamily::Unix,
                                              socket::SockType::Stream,
                                              socket::SOCK_CLOEXEC,
                                              0));

        let unix_addr = try_sock!("Linking", path, socket::UnixAddr::new(path));
        let sock_addr = socket::SockAddr::Unix(unix_addr);
        try_sock!("Connecting", path, socket::connect(sockfd, &sock_addr));

        Ok(Socket {
               fd: sockfd,
                next_serial: std::cell::Cell::new(0),
                logger: None,
           })
    }

    /// Connects to display socket on default path.
    ///
    /// See `get_default_socket_path`.
    pub fn connect_default() -> Result<Self, SkylaneError> {
        let path = get_default_socket_path()?;
        Self::connect(&path)
    }

    /// Returns raw file descriptor.
    pub fn get_fd(&self) -> RawFd {
        self.fd
    }

    /// Increments and return next serial.
    pub fn get_next_serial(&self) -> u32 {
        let serial = self.next_serial.get();
        self.next_serial.set(serial + 1);
        serial
    }

    /// Sets logger.
    pub fn set_logger(&mut self, logger: Logger) {
        self.logger = logger;
    }

    /// Returns logger.
    pub fn get_logger(&self) -> Logger {
        self.logger
    }

    /// Reads from sockets.
    ///
    /// Writes data read from socket to passed buffers. `bytes` is used for raw data and `fds` is
    /// used for file descriptors.
    ///
    /// Returns number of bytes written to `bytes` and number of file descriptors written to `fds`.
    pub fn receive_message(&self,
                           bytes: &mut [u8],
                           fds: &mut [u8])
                           -> Result<(usize, usize), SkylaneError> {
        let mut cmsg: socket::CmsgSpace<[RawFd; 1]> = socket::CmsgSpace::new();
        let mut iov: [uio::IoVec<&mut [u8]>; 1] = [uio::IoVec::from_mut_slice(&mut bytes[..]); 1];

        let msg = socket::recvmsg(self.fd, &mut iov[..], Some(&mut cmsg), socket::MSG_DONTWAIT)?;

        let mut num_fds = 0;
        let mut buf = Cursor::new(fds);
        for cmsg in msg.cmsgs() {
            match cmsg {
                socket::ControlMessage::ScmRights(newfds) => {
                    buf.write_i32::<NativeEndian>(newfds[0])?;
                    num_fds += 1;
                }
                _ => {}
            }
        }
        Ok((msg.bytes, num_fds))
    }

    /// Writes given data to socket.
    pub fn write(&self, bytes: &[u8]) -> Result<(), SkylaneError> {
        let iov: [uio::IoVec<&[u8]>; 1] = [uio::IoVec::from_slice(&bytes[..]); 1];
        let cmsgs: [socket::ControlMessage; 0] = unsafe { std::mem::uninitialized() };

        socket::sendmsg(self.fd, &iov[..], &cmsgs[..], socket::MSG_DONTWAIT, None)?;
        Ok(())
    }

    /// Writes given data to socket.
    pub fn write_with_control_data(&self, bytes: &[u8], fds: &[RawFd]) -> Result<(), SkylaneError> {
        let iov: [uio::IoVec<&[u8]>; 1] = [uio::IoVec::from_slice(&bytes[..]); 1];
        let cmsgs = [socket::ControlMessage::ScmRights(fds)];

        socket::sendmsg(self.fd, &iov[..], &cmsgs[..], socket::MSG_DONTWAIT, None)?;
        Ok(())
    }
}

// -------------------------------------------------------------------------------------------------

/// Private methods.
impl Socket {
    /// Constructs new `Socket`.
    ///
    /// This method is used by `DisplaySocket` when connection was accepted.
    fn new(fd: RawFd) -> Self {
        Socket {
            fd: fd,
            next_serial: std::cell::Cell::new(0),
            logger: None,
        }
    }
}

// -------------------------------------------------------------------------------------------------

/// Structure representing global socket on server side.
///
/// After client connects to this socket `Socket` is created which can be then used for further
/// communication with this particular client.
#[derive(Clone)]
pub struct DisplaySocket {
    fd: RawFd,
    path: std::path::PathBuf,
}

// -------------------------------------------------------------------------------------------------

impl DisplaySocket {
    /// Creates new `DisplaySocket`.
    pub fn new(path: &std::path::Path) -> Result<Self, SkylaneError> {
        let sockfd = try_sock!("Creating",
                               path,
                               socket::socket(socket::AddressFamily::Unix,
                                              socket::SockType::Stream,
                                              socket::SOCK_CLOEXEC,
                                              0));

        let unix_addr = try_sock!("Linking", path, socket::UnixAddr::new(path));
        let sock_addr = socket::SockAddr::Unix(unix_addr);
        try_sock!("Binding", path, socket::bind(sockfd, &sock_addr));
        try_sock!("Listening", path, socket::listen(sockfd, 128));

        Ok(DisplaySocket {
               fd: sockfd,
               path: path.to_owned(),
           })
    }

    /// Creates new `DisplaySocket` on default path.
    ///
    /// See `get_default_socket_path`.
    pub fn new_default() -> Result<Self, SkylaneError> {
        let path = get_default_socket_path()?;
        Self::new(&path)
    }

    /// Accepts client connection and return new `Socket`.
    pub fn accept(&self) -> Result<Socket, SkylaneError> {
        let fd = socket::accept(self.fd)?;
        Ok(Socket::new(fd))
    }

    /// Returns socket file descriptor.
    pub fn get_fd(&self) -> RawFd {
        self.fd
    }
}

// -------------------------------------------------------------------------------------------------

impl Drop for DisplaySocket {
    fn drop(&mut self) {
        // Remove socket path. Nothing to do with result.
        let _ = nix::unistd::unlink(self.path.as_path());
    }
}

// -------------------------------------------------------------------------------------------------
