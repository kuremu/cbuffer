use queues::{CircularBuffer, IsQueue};
use std::io;
use std::io::prelude::{Read, Write};
use std::sync::{Arc, Mutex};

pub const BUFSIZE: usize = 1024;

#[derive(PartialEq, Debug)]
pub enum Action {
    Buffer,
    Record,
    Finish,
}

#[derive(Debug)]
pub struct State {
    pub action: Action,
    pub buffered: usize,
    pub written: usize,
    pub bitrate: usize,
    pub seconds: usize,
}

pub type StateMut = Arc<Mutex<State>>;

pub fn record<T: Read, U: Write>(
    mut reader: T,
    mut writer: U,
    len: usize,
    state: &StateMut,
) -> io::Result<()> {
    let mut buffer = [0; BUFSIZE];
    let mut circ = CircularBuffer::<Vec<u8>>::new(len);
    let offset = if let Ok(state) = state.lock() {
        let len2 = (state.bitrate as f64 / 8f64 * state.seconds as f64) as usize;
        len * BUFSIZE - len2
    } else {
        0
    };

    // switch from record to pause until quit
    while !state_is(state, Action::Finish) {
        // record to buffer
        while state_is(state, Action::Buffer) {
            reader.read_exact(&mut buffer)?;
            if let Ok(mut state) = state.lock() {
                circ.add(buffer.to_vec()).expect("couldn't add to queue");
                state.buffered = circ.size() * BUFSIZE;
                if circ.size() == circ.capacity() {
                    state.buffered -= offset;
                }
            }
        }

        if state_is(state, Action::Record) {
            // write buffer to stdout
            if let Ok(mut state) = state.lock() {
                state.written += circ.size() * BUFSIZE;
                if circ.size() == circ.capacity() {
                    // trim first buffer to real buffer time
                    let buffer = circ.remove().expect("couldn't remove from queue");
                    writer.write_all(&buffer[offset..])?;
                    state.written -= offset;
                }
                while circ.peek().is_ok() {
                    let buffer = circ.remove().expect("couldn't remove from queue");
                    writer.write_all(&buffer)?;
                }
                state.buffered = 0;
            }
            writer.flush()?;

            // write stdin to stdout
            while state_is(state, Action::Record) {
                let bytes = reader.read(&mut buffer)?;
                if bytes == 0 {
                    break;
                }
                writer.write_all(&buffer[..bytes])?;
                writer.flush()?;
                if let Ok(mut state) = state.lock() {
                    state.written += bytes;
                }
            }
        }
    }

    Ok(())
}

pub fn state_is(state: &StateMut, action: Action) -> bool {
    state
        .lock()
        .map(|state| state.action == action)
        .unwrap_or(false)
}
