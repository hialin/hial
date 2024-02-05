use std::{collections::HashMap, fs::File, io::Read, path::Path};

use crate::warning;

pub fn detect_file_indentation(path: impl AsRef<Path>) -> String {
    const BUFFER_LEN: usize = 64 * 1024;
    let mut buffer = [0u8; BUFFER_LEN];
    match File::open(path) {
        Ok(mut file) => {
            let read_count = file.read(&mut buffer);
            detect_indentation(std::str::from_utf8(&buffer).unwrap_or(""))
        }
        Err(e) => {
            warning!("cannot detect file indentation: {}", e);
            String::new()
        }
    }
}

pub fn detect_indentation(data: &str) -> String {
    let mut space_count = HashMap::new();
    let mut tab_count = 0;
    for line in data.lines() {
        let leading_spaces = line.chars().take_while(|c| *c == ' ').count();
        if leading_spaces > 0 {
            *space_count.entry(leading_spaces).or_insert(0) += 1;
        } else if line.starts_with('\t') {
            tab_count += 1;
        }
    }

    let mut max_spaces = 0;
    let mut max_count = 0;
    for (spaces, count) in space_count {
        if count > max_count {
            max_spaces = spaces;
            max_count = count;
        }
    }

    if tab_count > max_count {
        "\t".to_string()
    } else if max_spaces > 0 {
        " ".repeat(max_spaces)
    } else {
        String::new()
    }
}
