use std::ptr;
use std::net::{TcpStream, TcpListener};
use std::os::unix::io::AsRawFd;
use std::io::{Read, Write};
use futures::io::ErrorKind;
use crate::response::Response;
use crate::request::Request;
use crate::Directory;
use crate::event_loop::unsafe_c::{Timespec, create_kqueue, put_kevent_in_kqueue, poll_kevents_from_q, create_k_read_event, create_k_write_event, KeventInternal};

pub struct Queue<T> where T: GeneralEvent {
    pub events: Vec<T>,
    pub wait_timeout: Timespec,
    pub dir: Directory,
    pub(crate) fd: i32,
}


impl<T> Queue<T> where T: GeneralEvent {
    pub fn new(dir: Directory) -> Result<Queue<T>, String> {
        Ok(Self {
            events: vec![],
            wait_timeout: Timespec::zero(),
            fd: create_kqueue()?,
            dir,
        })
    }

    pub fn add(&mut self, event: T) -> Result<(), String> {
        let kevent = event.get_kevent();
        self.events.push(event);
        put_kevent_in_kqueue(self.fd, kevent, &self.wait_timeout)
    }

    pub fn poll(&mut self) -> Result<Vec<T>, String> {
        let finished_events = loop {
            let results = poll_kevents_from_q(self.fd)?;
            if results.len() > 0 {
                break results;
            }
        };

        let mut indexes = Vec::with_capacity(8);
        for event in finished_events {
            let index =
                self.events
                    .iter()
                    .position(|ev| ev.get_ident() == event.ident);
            if let Some(idx) = index {
                indexes.push(idx)
            }
        }

        let mut events = Vec::with_capacity(8);
        for index in indexes {
            events.push(self.events.remove(index))
        }
        Ok(events)
    }
}

//identified by ident,filter and udata
// #[derive(Debug)]
pub struct Event {
    //todo change to request or sth like that
    pub data: [u8; 2048],
    pub stream: TcpStream,
    // the internal C representation of the Event
    pub kevent: [KeventInternal; 1],
}

pub struct ListenerEvent {
    //todo change to request or sth like that
    pub data: [u8; 2048],
    pub listener: TcpListener,
    // the internal C representation of the Event
    pub kevent: [KeventInternal; 1],
}


trait GeneralEvent {
    fn get_ident(&self) -> u64;
    fn get_kevent(&self) -> *const KeventInternal;
}

impl Event {
    pub(crate) fn new_read(stream: TcpStream, data: [u8; 2048]) -> Self {
        Self {
            data,
            kevent: [create_k_read_event(stream.as_raw_fd())],
            stream,
        }
    }
    pub(crate) fn new_write(stream: TcpStream, data: [u8; 2048]) -> Self {
        Self {
            data,
            kevent: [create_k_write_event(stream.as_raw_fd())],
            stream,
        }
    }
}

impl GeneralEvent for Event {
    fn get_ident(&self) -> u64 {
        self.kevent[0].ident
    }

    fn get_kevent(&self) -> *const KeventInternal {
        self.kevent.as_ptr()
    }
}

impl GeneralEvent for ListenerEvent {
    fn get_ident(&self) -> u64 {
        self.kevent[0].ident
    }

    fn get_kevent(&self) -> *const KeventInternal {
        self.kevent.as_ptr()
    }
}

impl ListenerEvent {
    pub(crate) fn new(listener: TcpListener, data: [u8; 2048]) -> Self {
        Self {
            data,
            kevent: [create_k_read_event(listener.as_raw_fd())],
            listener,
        }
    }
}
