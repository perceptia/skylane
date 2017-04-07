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

//! Common definitions for server and client parts of `skylane` crate.

use std;
use std::error::Error;

use nix;

use object::{Object, ObjectId};

// -------------------------------------------------------------------------------------------------

/// Enumeration for all `skylane` errors.
#[derive(Debug)]
pub enum SkylaneError {
    /// Wrapper for `std::io::Error`.
    IO {
        /// Description of the error.
        description: String,
    },

    /// Wrapper for `nix::Error`.
    Socket {
        /// Description of the error.
        description: String,
    },

    /// Error emitted when trying to access not existing object.
    WrongObject {
        /// ID of requested object.
        object_id: ObjectId,
    },

    /// Error emitted when requested method does not exist in given interface.
    WrongOpcode {
        /// Name of interface.
        name: &'static str,
        /// Referred object ID.
        object_id: u32,
        /// Requested method.
        opcode: u16,
    },

    /// Other errors.
    Other(String),
}

impl std::convert::From<std::io::Error> for SkylaneError {
    fn from(error: std::io::Error) -> Self {
        SkylaneError::IO { description: error.description().to_owned() }
    }
}

impl std::convert::From<nix::Error> for SkylaneError {
    fn from(error: nix::Error) -> Self {
        SkylaneError::Socket { description: error.description().to_owned() }
    }
}

impl std::convert::From<std::env::VarError> for SkylaneError {
    fn from(error: std::env::VarError) -> Self {
        SkylaneError::Other(error.description().to_owned())
    }
}

// -------------------------------------------------------------------------------------------------

/// Header of Wayland message.
#[repr(C)]
#[derive(Debug)]
pub struct Header {
    /// ID of the referred object.
    pub object_id: u32,

    /// ID of the called method.
    pub opcode: u16,

    /// Size of the message including header.
    pub size: u16,
}

// -------------------------------------------------------------------------------------------------

/// Type alias for logging function.
pub type Logger = Option<fn(String) -> ()>;

// -------------------------------------------------------------------------------------------------

/// Return enumeration for callbacks.
///
/// This enumeration will be removed. It proved it is insufficient on client side. `Bundle` should
/// be used instead.
pub enum Task {
    /// Requests creation of object.
    Create {
        /// New object ID.
        id: ObjectId,
        /// Object to be added.
        object: Box<Object>,
    },

    /// Requests destruction of object.
    Destroy {
        /// ID of object to be destroyed.
        id: ObjectId,
    },

    /// Requests nothing.
    None,
}

// -------------------------------------------------------------------------------------------------
