#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Errors for text editing operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditError {
    IndexOutOfBounds,
}

/// Simple line-based text buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextBuffer {
    lines: Vec<String>,
}

impl TextBuffer {
    /// Creates an empty buffer.
    pub fn new() -> Self {
        Self { lines: Vec::new() }
    }

    /// Loads a buffer from raw text.
    pub fn from_text(text: &str) -> Self {
        let lines = if text.is_empty() {
            Vec::new()
        } else {
            text.split('\n').map(|line| line.to_string()).collect()
        };
        Self { lines }
    }

    /// Returns the number of lines.
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Returns the current lines (read-only).
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// Inserts a line at the requested index.
    pub fn insert_line(&mut self, index: usize, line: &str) -> Result<(), EditError> {
        if index > self.lines.len() {
            return Err(EditError::IndexOutOfBounds);
        }
        self.lines.insert(index, line.to_string());
        Ok(())
    }

    /// Replaces the line at the requested index.
    pub fn replace_line(&mut self, index: usize, line: &str) -> Result<(), EditError> {
        if index >= self.lines.len() {
            return Err(EditError::IndexOutOfBounds);
        }
        self.lines[index] = line.to_string();
        Ok(())
    }

    /// Removes the line at the requested index.
    pub fn remove_line(&mut self, index: usize) -> Result<(), EditError> {
        if index >= self.lines.len() {
            return Err(EditError::IndexOutOfBounds);
        }
        self.lines.remove(index);
        Ok(())
    }

    /// Renders the buffer back into a single string.
    pub fn to_text(&self) -> String {
        self.lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_roundtrip() {
        let mut buffer = TextBuffer::from_text("one\ntwo");
        buffer.insert_line(2, "three").unwrap();
        assert_eq!(buffer.to_text(), "one\ntwo\nthree");
    }

    #[test]
    fn from_text_empty_is_empty_buffer() {
        let buffer = TextBuffer::from_text("");
        assert_eq!(buffer.line_count(), 0);
        assert_eq!(buffer.to_text(), "");
    }

    #[test]
    fn line_count_matches_lines() {
        let buffer = TextBuffer::from_text("one\ntwo");
        assert_eq!(buffer.line_count(), 2);
    }

    #[test]
    fn lines_returns_current_view() {
        let buffer = TextBuffer::from_text("one\ntwo");
        assert_eq!(buffer.lines(), &["one".to_string(), "two".to_string()]);
    }

    #[test]
    fn insert_line_rejects_out_of_bounds() {
        let mut buffer = TextBuffer::new();
        assert_eq!(buffer.insert_line(1, "oops"), Err(EditError::IndexOutOfBounds));
    }

    #[test]
    fn replace_line() {
        let mut buffer = TextBuffer::from_text("one");
        buffer.replace_line(0, "uno").unwrap();
        assert_eq!(buffer.to_text(), "uno");
    }

    #[test]
    fn replace_line_rejects_out_of_bounds() {
        let mut buffer = TextBuffer::new();
        assert_eq!(buffer.replace_line(0, "oops"), Err(EditError::IndexOutOfBounds));
    }

    #[test]
    fn remove_line() {
        let mut buffer = TextBuffer::from_text("one\ntwo");
        buffer.remove_line(0).unwrap();
        assert_eq!(buffer.to_text(), "two");
    }

    #[test]
    fn remove_line_rejects_out_of_bounds() {
        let mut buffer = TextBuffer::new();
        assert_eq!(buffer.remove_line(0), Err(EditError::IndexOutOfBounds));
    }

    #[test]
    fn empty_buffer_to_text() {
        let buffer = TextBuffer::new();
        assert_eq!(buffer.to_text(), "");
    }
}
