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

//! Defines `Bundle`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use defs::SkylaneError;
use object::{Object, ObjectId, DISPLAY_ID, SERVER_START_ID};
use sockets::Socket;

// -------------------------------------------------------------------------------------------------

/// `Bundle` is passed to objects while invocation of their methods and can be used by them to
/// add/remove new objects or access socket. It also serves this crate internally as data store.
pub struct Bundle {
    socket: Socket,
    objects: Rc<RefCell<HashMap<ObjectId, Rc<RefCell<Box<Object>>>>>>,
}

impl Bundle {
    /// Returns connection socket.
    pub fn get_socket(&self) -> Socket {
        self.socket.clone()
    }

    /// Returns next available client object ID.
    ///
    /// If no objects are registered this will be `DISPLAY_ID`. Otherwise ID one bigger than the
    /// biggest ID.
    ///
    /// TODO: This implementation is naive and invalid. Check how it is implemented in `libwayland`
    /// and do the same here for compability. `get_next_available_server_object_id` may also need a
    /// change.
    ///
    /// TODO: Move `get_next_available_client_object_id` and `get_next_available_server_object_id`
    /// to trait available only in celit or server side respectively.
    pub fn get_next_available_client_object_id(&self) -> ObjectId {
        if let Some(max) = self.objects.borrow().keys().max() {
            if *max >= DISPLAY_ID {
                max.incremented()
            } else {
                DISPLAY_ID
            }
        } else {
            DISPLAY_ID
        }
    }

    /// Returns next available server object ID.
    pub fn get_next_available_server_object_id(&self) -> ObjectId {
        if let Some(max) = self.objects.borrow().keys().max() {
            if *max >= SERVER_START_ID {
                max.incremented()
            } else {
                SERVER_START_ID
            }
        } else {
            SERVER_START_ID
        }
    }

    /// Adds new object. From now client requests or server events to object with given `id` will
    /// be passed to this `object`. If another object is already assigned to this `id` the
    /// assignment will be overridden.
    ///
    /// Here the only requirement for the object is to implement `Object` trait. In practical use
    /// one will pass implementations of `Interface` traits from protocol definitions wrapped in
    /// `Handler` structure with `Dispatcher` attached as defined in `skylane_protocols` crate.
    pub fn add_object(&mut self, id: ObjectId, object: Box<Object>) {
        self.objects.borrow_mut().insert(id, Rc::new(RefCell::new(object)));
    }

    /// Gets next available client object ID and adds new object. Returns ID of newly added object.
    pub fn add_next_client_object(&mut self, object: Box<Object>) -> ObjectId {
        let id = self.get_next_available_client_object_id();
        self.add_object(id, object);
        id
    }

    /// Gets next available server object ID and adds new object. Returns ID of newly added object.
    pub fn add_next_server_object(&mut self, object: Box<Object>) -> ObjectId {
        let id = self.get_next_available_server_object_id();
        self.add_object(id, object);
        id
    }

    /// Removes object with given `id`.
    pub fn remove_object(&mut self, id: ObjectId) {
        self.objects.borrow_mut().remove(&id);
    }
}

// -------------------------------------------------------------------------------------------------

/// Methods of `Bundle` available in this crate but not exported.
pub trait BundleInternal {
    /// Constructs new `Bundle`.
    fn new(socket: Socket) -> Self;

    /// Clones the `Bundle`.
    ///
    /// `Bundle`'s public API does not allow cloning, but `Bundle` is also used in this crate as
    /// helper structure and must be shared between `Connection` and `Controller`.
    fn duplicate(&self) -> Self;

    /// Returns object of given ID.
    fn get_handler(&self, object_id: ObjectId) -> Result<Rc<RefCell<Box<Object>>>, SkylaneError>;
}

impl BundleInternal for Bundle {
    fn new(socket: Socket) -> Self {
        Bundle {
            socket: socket,
            objects: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    fn duplicate(&self) -> Self {
        Bundle {
            socket: self.socket.clone(),
            objects: self.objects.clone(),
        }
    }

    fn get_handler(&self, object_id: ObjectId) -> Result<Rc<RefCell<Box<Object>>>, SkylaneError> {
        if let Some(object) = self.objects.borrow().get(&object_id) {
            Ok(object.clone())
        } else {
            Err(SkylaneError::WrongObject { object_id: object_id })
        }
    }
}

// -------------------------------------------------------------------------------------------------
