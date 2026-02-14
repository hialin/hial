use std::{
    collections::HashMap,
    fmt,
    fs::File,
    io::{self, Read},
    path::Path,
};

use crate::warning;

pub fn detect_indentation(data: &str) -> String {
    let mut reader = IndentationReader::new(data.as_bytes());
    if io::copy(&mut reader, &mut io::sink()).is_err() {
        return String::new();
    }
    reader.detected_indentation()
}

pub fn detect_file_indentation(path: &Path) -> String {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(err) => {
            warning!("cannot open file for indentation detection {:?}: {}", path, err);
            return String::new();
        }
    };
    let mut reader = IndentationReader::new(file);
    if let Err(err) = io::copy(&mut reader, &mut io::sink()) {
        warning!(
            "cannot read file for indentation detection {:?}: {}",
            path, err
        );
    }
    reader.detected_indentation()
}

pub struct IndentationReader<R: Read> {
    inner: R,
    state: LineState,
    space_count: HashMap<usize, usize>,
    tab_count: usize,
    finished: bool,
}

impl<R: Read> IndentationReader<R> {
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            state: LineState::Start,
            space_count: HashMap::new(),
            tab_count: 0,
            finished: false,
        }
    }

    pub fn detected_indentation(&self) -> String {
        let mut max_spaces = 0;
        let mut max_count = 0;
        for (spaces, count) in &self.space_count {
            if *count > max_count {
                max_spaces = *spaces;
                max_count = *count;
            }
        }
        if self.tab_count > max_count {
            "\t".to_string()
        } else if max_spaces > 0 {
            " ".repeat(max_spaces)
        } else {
            String::new()
        }
    }

    fn process_bytes(&mut self, buf: &[u8]) {
        for byte in buf {
            self.state = match (self.state, *byte) {
                (LineState::Start, b' ') => LineState::Spaces(1),
                (LineState::Start, b'\t') => {
                    self.tab_count += 1;
                    LineState::Done
                }
                (LineState::Start, b'\n') => LineState::Start,
                (LineState::Start, _) => LineState::Done,
                (LineState::Spaces(spaces), b' ') => LineState::Spaces(spaces + 1),
                (LineState::Spaces(spaces), b'\n') => {
                    *self.space_count.entry(spaces).or_insert(0) += 1;
                    LineState::Start
                }
                (LineState::Spaces(spaces), _) => {
                    *self.space_count.entry(spaces).or_insert(0) += 1;
                    LineState::Done
                }
                (LineState::Done, b'\n') => LineState::Start,
                (LineState::Done, _) => LineState::Done,
            };
        }
    }
}

impl<R: Read> Read for IndentationReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let bytes_read = self.inner.read(buf)?;
        if bytes_read == 0 {
            if !self.finished {
                if let LineState::Spaces(spaces) = self.state {
                    *self.space_count.entry(spaces).or_insert(0) += 1;
                    self.state = LineState::Done;
                }
                self.finished = true;
            }
            return Ok(0);
        }
        self.process_bytes(&buf[..bytes_read]);
        Ok(bytes_read)
    }
}

impl<R: Read> fmt::Debug for IndentationReader<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IndentationReader")
            .field("state", &self.state)
            .field("space_count", &self.space_count)
            .field("tab_count", &self.tab_count)
            .field("finished", &self.finished)
            .finish()
    }
}

#[derive(Clone, Copy, Debug)]
enum LineState {
    Start,
    Spaces(usize),
    Done,
}
