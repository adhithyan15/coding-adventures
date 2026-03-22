/// Pipe -- a unidirectional byte stream between two endpoints.
///
/// A pipe is the simplest IPC mechanism. Think of a pneumatic tube in an old
/// bank: you stuff a message capsule in one end, and it arrives at the other
/// end. The tube only goes one direction, and messages arrive in the order
/// they were sent (FIFO).
///
/// Under the hood, a pipe is a **circular buffer** -- a fixed-size array where
/// the write position wraps around to the beginning when it reaches the end.
/// This avoids the need to shift data forward after each read.
///
/// ## Circular Buffer Mechanics
///
/// Imagine a buffer of 8 bytes (we use 4096 in practice, but 8 is easier to
/// draw):
///
/// ```text
///   Initial state (empty):
///     +-+-+-+-+-+-+-+-+
///     | | | | | | | | |    read_pos = 0, write_pos = 0
///     +-+-+-+-+-+-+-+-+
///      ^R              ^W  (R and W at same position = empty)
///
///   After writing "hello" (5 bytes):
///     +-+-+-+-+-+-+-+-+
///     |h|e|l|l|o| | | |    read_pos = 0, write_pos = 5
///     +-+-+-+-+-+-+-+-+
///      ^R          ^W
///
///   After reading 3 bytes ("hel"):
///     +-+-+-+-+-+-+-+-+
///     | | | |l|o| | | |    read_pos = 3, write_pos = 5
///     +-+-+-+-+-+-+-+-+
///            ^R    ^W
///
///   Wrapping -- write "fghij" (wraps around the end):
///     +-+-+-+-+-+-+-+-+
///     |i|j| |l|o|f|g|h|    read_pos = 3, write_pos = 2
///     +-+-+-+-+-+-+-+-+
///        ^W  ^R             write_pos BEHIND read_pos = wrapped
/// ```
///
/// ## EOF and Broken Pipe
///
/// Two important signals arise from reference counting:
///
/// ```text
///   +---------------------+------------------------------------------+
///   | Condition           | Result                                   |
///   +---------------------+------------------------------------------+
///   | writer_count == 0   | Read returns empty (EOF). No more data   |
///   | AND buffer empty    | will ever arrive.                        |
///   +---------------------+------------------------------------------+
///   | reader_count == 0   | Write returns Err(BrokenPipe). Nobody    |
///   |                     | is reading.                              |
///   +---------------------+------------------------------------------+
/// ```
///
/// In a shell pipeline like `ls | grep foo`, when `ls` finishes and closes
/// its write end, `grep` sees EOF on the read end. Conversely, if `grep`
/// exits early, `ls` gets SIGPIPE (broken pipe).

use crate::IpcError;

/// Default pipe buffer size: 4096 bytes, matching one memory page.
pub const DEFAULT_PIPE_CAPACITY: usize = 4096;

pub struct Pipe {
    /// The circular buffer. One slot larger than `capacity` to distinguish
    /// "full" from "empty" -- both would otherwise have write_pos == read_pos.
    buffer: Vec<u8>,

    /// The usable capacity (buffer.len() - 1).
    capacity: usize,

    /// Index of the next byte to be read. Advances on each read, wraps around.
    read_pos: usize,

    /// Index of the next byte to be written. Advances on each write, wraps around.
    write_pos: usize,

    /// Number of open file descriptors for the read end. When this drops to 0,
    /// writes will fail with BrokenPipe.
    reader_count: u32,

    /// Number of open file descriptors for the write end. When this drops to 0,
    /// reads return empty (EOF).
    writer_count: u32,
}

impl Pipe {
    /// Create a new pipe with an empty circular buffer of the given capacity.
    ///
    /// The pipe starts with one reader and one writer -- the two file
    /// descriptors returned by the pipe() system call.
    pub fn new(capacity: usize) -> Self {
        Pipe {
            buffer: vec![0u8; capacity + 1],
            capacity,
            read_pos: 0,
            write_pos: 0,
            reader_count: 1,
            writer_count: 1,
        }
    }

    /// Write data into the pipe.
    ///
    /// Returns `Ok(bytes_written)`. This may be less than `data.len()` if the
    /// buffer fills up (partial write). In a real OS, the process would block
    /// until space is available; here we return the partial count.
    ///
    /// Returns `Err(BrokenPipe)` if no readers remain.
    pub fn write(&mut self, data: &[u8]) -> Result<usize, IpcError> {
        if self.reader_count == 0 {
            return Err(IpcError::BrokenPipe);
        }

        let mut bytes_written = 0;
        for &byte in data {
            if self.is_full() {
                break;
            }
            self.buffer[self.write_pos] = byte;
            self.write_pos = (self.write_pos + 1) % self.buffer.len();
            bytes_written += 1;
        }
        Ok(bytes_written)
    }

    /// Read up to `count` bytes from the pipe.
    ///
    /// Returns a `Vec<u8>`. The vec may be shorter than `count` if fewer bytes
    /// are available. Returns an empty vec if the buffer is empty AND all
    /// writers have closed (EOF).
    pub fn read(&mut self, count: usize) -> Vec<u8> {
        let mut result = Vec::with_capacity(count);
        for _ in 0..count {
            if self.is_empty() {
                break;
            }
            result.push(self.buffer[self.read_pos]);
            self.read_pos = (self.read_pos + 1) % self.buffer.len();
        }
        result
    }

    /// Close the read end (decrement reader count).
    pub fn close_read(&mut self) {
        if self.reader_count > 0 {
            self.reader_count -= 1;
        }
    }

    /// Close the write end (decrement writer count).
    pub fn close_write(&mut self) {
        if self.writer_count > 0 {
            self.writer_count -= 1;
        }
    }

    /// Is the buffer empty? (read_pos == write_pos)
    pub fn is_empty(&self) -> bool {
        self.read_pos == self.write_pos
    }

    /// Is the buffer full? (next write_pos would equal read_pos)
    pub fn is_full(&self) -> bool {
        (self.write_pos + 1) % self.buffer.len() == self.read_pos
    }

    /// Number of bytes available to read.
    pub fn available(&self) -> usize {
        (self.write_pos + self.buffer.len() - self.read_pos) % self.buffer.len()
    }

    /// Number of bytes that can be written before the buffer is full.
    pub fn space(&self) -> usize {
        self.capacity - self.available()
    }

    /// Is the pipe at EOF? (all writers closed AND buffer empty)
    pub fn is_eof(&self) -> bool {
        self.writer_count == 0 && self.is_empty()
    }

    /// Get the usable capacity.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get the reader count.
    pub fn reader_count(&self) -> u32 {
        self.reader_count
    }

    /// Get the writer count.
    pub fn writer_count(&self) -> u32 {
        self.writer_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- Basic write/read --

    #[test]
    fn test_write_and_read_bytes() {
        let mut pipe = Pipe::new(4096);
        let data = vec![72, 101, 108, 108, 111]; // "Hello"

        let written = pipe.write(&data).unwrap();
        assert_eq!(written, 5);

        let result = pipe.read(5);
        assert_eq!(result, data);
    }

    #[test]
    fn test_fifo_ordering() {
        let mut pipe = Pipe::new(4096);
        pipe.write(&[97, 98, 99]).unwrap();     // "abc"
        pipe.write(&[100, 101, 102]).unwrap();   // "def"

        let result = pipe.read(6);
        assert_eq!(result, vec![97, 98, 99, 100, 101, 102]);
    }

    // -- Circular wrapping --

    #[test]
    fn test_circular_buffer_wrapping() {
        let mut pipe = Pipe::new(8);

        // Write 6 bytes.
        pipe.write(&[1, 2, 3, 4, 5, 6]).unwrap();

        // Read 4 to free space at the front.
        let result = pipe.read(4);
        assert_eq!(result, vec![1, 2, 3, 4]);

        // Write 5 more -- wraps around the end.
        pipe.write(&[7, 8, 9, 10, 11]).unwrap();

        // Read everything back.
        let result = pipe.read(7);
        assert_eq!(result, vec![5, 6, 7, 8, 9, 10, 11]);
    }

    // -- Partial reads/writes --

    #[test]
    fn test_partial_read_returns_available() {
        let mut pipe = Pipe::new(4096);
        pipe.write(&[1, 2, 3]).unwrap();

        let result = pipe.read(10);
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_read_from_empty_returns_empty() {
        let mut pipe = Pipe::new(4096);
        let result = pipe.read(5);
        assert!(result.is_empty());
    }

    #[test]
    fn test_partial_write_when_nearly_full() {
        let mut pipe = Pipe::new(4);

        pipe.write(&[1, 2, 3]).unwrap();

        // Only 1 slot left.
        let written = pipe.write(&[4, 5, 6]).unwrap();
        assert_eq!(written, 1);

        let result = pipe.read(4);
        assert_eq!(result, vec![1, 2, 3, 4]);
    }

    // -- EOF --

    #[test]
    fn test_eof_when_writers_closed_and_empty() {
        let mut pipe = Pipe::new(4096);
        pipe.close_write();
        assert!(pipe.is_eof());
        assert!(pipe.read(5).is_empty());
    }

    #[test]
    fn test_not_eof_when_writers_closed_but_data_remains() {
        let mut pipe = Pipe::new(4096);
        pipe.write(&[1, 2, 3]).unwrap();
        pipe.close_write();

        assert!(!pipe.is_eof());
        let result = pipe.read(3);
        assert_eq!(result, vec![1, 2, 3]);
        assert!(pipe.is_eof());
    }

    #[test]
    fn test_not_eof_when_writers_open() {
        let pipe = Pipe::new(4096);
        assert!(!pipe.is_eof());
    }

    // -- Broken pipe --

    #[test]
    fn test_broken_pipe_when_no_readers() {
        let mut pipe = Pipe::new(4096);
        pipe.close_read();

        let result = pipe.write(&[1, 2, 3]);
        assert_eq!(result, Err(IpcError::BrokenPipe));
    }

    // -- Capacity tracking --

    #[test]
    fn test_available_and_space() {
        let mut pipe = Pipe::new(10);

        assert_eq!(pipe.available(), 0);
        assert_eq!(pipe.space(), 10);

        pipe.write(&[1, 2, 3]).unwrap();
        assert_eq!(pipe.available(), 3);
        assert_eq!(pipe.space(), 7);

        pipe.read(2);
        assert_eq!(pipe.available(), 1);
        assert_eq!(pipe.space(), 9);
    }

    #[test]
    fn test_empty_and_full() {
        let mut pipe = Pipe::new(3);

        assert!(pipe.is_empty());
        assert!(!pipe.is_full());

        pipe.write(&[1, 2, 3]).unwrap();
        assert!(!pipe.is_empty());
        assert!(pipe.is_full());
    }

    // -- Reference counting --

    #[test]
    fn test_initial_counts() {
        let pipe = Pipe::new(4096);
        assert_eq!(pipe.reader_count(), 1);
        assert_eq!(pipe.writer_count(), 1);
    }

    #[test]
    fn test_close_read_decrements() {
        let mut pipe = Pipe::new(4096);
        pipe.close_read();
        assert_eq!(pipe.reader_count(), 0);
    }

    #[test]
    fn test_close_write_decrements() {
        let mut pipe = Pipe::new(4096);
        pipe.close_write();
        assert_eq!(pipe.writer_count(), 0);
    }

    #[test]
    fn test_close_read_no_underflow() {
        let mut pipe = Pipe::new(4096);
        pipe.close_read();
        pipe.close_read();
        assert_eq!(pipe.reader_count(), 0);
    }

    #[test]
    fn test_close_write_no_underflow() {
        let mut pipe = Pipe::new(4096);
        pipe.close_write();
        pipe.close_write();
        assert_eq!(pipe.writer_count(), 0);
    }

    #[test]
    fn test_default_capacity() {
        let pipe = Pipe::new(DEFAULT_PIPE_CAPACITY);
        assert_eq!(pipe.capacity(), 4096);
    }

    #[test]
    fn test_custom_capacity() {
        let pipe = Pipe::new(128);
        assert_eq!(pipe.capacity(), 128);
    }
}
