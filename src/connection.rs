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

//! Functionality related to controlling connection.

use std::io::{Cursor, SeekFrom, Seek};

use byteorder::{NativeEndian, ReadBytesExt};

use defs::{Header, SkylaneError, Task};
use object::{Object, ObjectId};
use bundle::{Bundle, BundleInternal};
use sockets::Socket;

// -------------------------------------------------------------------------------------------------

/// Structure providing control over connection. Allows adding and removing objects but processing
/// messages is left for `Connection`.
pub struct Controller {
    bundle: Bundle,
}

impl Controller {
    /// Constructs new `Controller`.
    fn new(bundle: Bundle) -> Self {
        Controller {
            bundle: bundle,
        }
    }

    /// Returns connection socket.
    pub fn get_socket(&self) -> Socket {
        self.bundle.get_socket()
    }

    /// Returns next available client object ID.
    ///
    /// See `Bundle::get_next_available_client_object_id`.
    pub fn get_next_available_client_object_id(&self) -> ObjectId {
        self.bundle.get_next_available_client_object_id()
    }

    /// Adds new object.
    ///
    /// See `Bundle::add_object`.
    pub fn add_object(&mut self, id: ObjectId, object: Box<Object>) {
        self.bundle.add_object(id, object);
    }

    /// Adds next object.
    ///
    /// See `Bundle::add_next_client_object`.
    pub fn add_next_client_object(&mut self, object: Box<Object>) -> ObjectId {
        self.bundle.add_next_client_object(object)
    }
}

/// `Bundle` does not implement `Clone`, so `Controller` must implement it manually.
impl Clone for Controller {
    fn clone(&self) -> Self {
        Controller::new(self.bundle.duplicate())
    }
}

// -------------------------------------------------------------------------------------------------

/// Structure aggregating all information about connection. Precesses events and dispatches them to
/// registered listeners.
pub struct Connection {
    bundle: Bundle,
}

impl Connection {
    /// Constructs new `Connection`.
    pub fn new(socket: Socket) -> Connection {
        Connection {
            bundle: Bundle::new(socket),
        }
    }

    /// Returns connection socket.
    pub fn get_socket(&self) -> Socket {
        self.bundle.get_socket()
    }

    /// Returns new `Controller` for the connection.
    pub fn get_controller(&self) -> Controller {
        Controller::new(self.bundle.duplicate())
    }

    /// Adds new object.
    ///
    /// See `Bundle::add_object`.
    pub fn add_object(&mut self, id: ObjectId, object: Box<Object>) {
        self.bundle.add_object(id, object);
    }

    /// Adds new object.
    ///
    /// See `Bundle::add_next_client_object`.
    pub fn add_next_client_object(&mut self, object: Box<Object>) -> ObjectId {
        self.bundle.add_next_client_object(object)
    }

    /// Removes object with given `id`.
    ///
    /// See `Bundle::remove_object`.
    pub fn remove_object(&mut self, id: ObjectId) {
        self.bundle.remove_object(id);
    }

    /// Reads data from socket and dispatches messages to registered objects.
    pub fn process_events(&mut self) -> Result<(), SkylaneError> {
        // TODO: What is more optimal - allocation these buffers here, or in struct? They don't
        // have to be zeroed every time, right? What buffer sizes are enough?
        let mut bytes: [u8; 1024] = [0; 1024];
        let mut fds: [u8; 24] = [0; 24];

        let (bytes_size, _fds_size) = self.bundle.get_socket()
                                                 .receive_message(&mut bytes, &mut fds)?;

        let mut bytes_buf = Cursor::new(&bytes[..]);
        let mut fds_buf = Cursor::new(&fds[..]);

        let mut position = 0;
        while position < bytes_size {
            bytes_buf.seek(SeekFrom::Start(position as u64))?;
            let header = Header {
                object_id: bytes_buf.read_u32::<NativeEndian>()?,
                opcode: bytes_buf.read_u16::<NativeEndian>()?,
                size: bytes_buf.read_u16::<NativeEndian>()?,
            };

            self.process_event(&header, &mut bytes_buf, &mut fds_buf)?;
            position += header.size as usize;
        }
        Ok(())
    }
}

/// Private methods.
impl Connection {
    /// Processes events:
    ///
    /// 1. searches for handler
    /// 2. calls `dispatch` method on handler
    /// 3. handles return code from `dispatch`.
    ///
    /// TODO: Remove third step.
    fn process_event(&mut self,
                     header: &Header,
                     mut bytes_buf: &mut Cursor<&[u8]>,
                     mut fds_buf: &mut Cursor<&[u8]>)
                     -> Result<(), SkylaneError> {
        let task = {
            let object_id = ObjectId::new(header.object_id);
            let handler_ref = self.bundle.get_handler(object_id)?;
            let mut handler = handler_ref.borrow_mut();
            handler.dispatch(&mut self.bundle, &header, bytes_buf, fds_buf)?
        };

        match task {
            Task::Create { id, object } => {
                self.add_object(id, object);
            }
            Task::Destroy { id } => {
                self.remove_object(id);
            }
            Task::None => {}
        }
        Ok(())
    }
}

// -------------------------------------------------------------------------------------------------
