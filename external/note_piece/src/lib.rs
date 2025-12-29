#![cfg_attr(not(test), no_std)]

/// Maximum number of notes stored in the piece.
pub const MAX_NOTES: usize = 8;
/// Maximum bytes per note.
pub const NOTE_LEN: usize = 64;

/// Errors returned by the note piece.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteError {
    Full,
    TooLong,
    Empty,
    NotFound,
    InvalidIndex,
    InvalidUtf8,
}

/// Fixed-size note payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Note {
    len: usize,
    buf: [u8; NOTE_LEN],
}

impl Note {
    /// Builds a note from text, enforcing length limits.
    pub fn from_str(text: &str) -> Result<Self, NoteError> {
        if text.is_empty() {
            return Err(NoteError::Empty);
        }
        let bytes = text.as_bytes();
        if bytes.len() > NOTE_LEN {
            return Err(NoteError::TooLong);
        }
        let mut buf = [0u8; NOTE_LEN];
        for (idx, byte) in bytes.iter().enumerate() {
            buf[idx] = *byte;
        }
        Ok(Self {
            len: bytes.len(),
            buf,
        })
    }

    /// Returns the stored text as UTF-8.
    pub fn as_str(&self) -> Result<&str, NoteError> {
        core::str::from_utf8(&self.buf[..self.len]).map_err(|_| NoteError::InvalidUtf8)
    }

    /// Returns the stored length.
    pub fn len(&self) -> usize {
        self.len
    }
}

/// A tiny note store that fits inside a puzzle piece.
#[derive(Debug, Clone)]
pub struct NoteStore {
    notes: [Option<Note>; MAX_NOTES],
    count: usize,
}

impl NoteStore {
    /// Creates an empty note store.
    pub const fn new() -> Self {
        Self {
            notes: [None; MAX_NOTES],
            count: 0,
        }
    }

    /// Returns the number of stored notes.
    pub fn count(&self) -> usize {
        self.count
    }

    /// Returns true when the store is full.
    pub fn is_full(&self) -> bool {
        self.count >= MAX_NOTES
    }

    /// Adds a note and returns the slot index.
    pub fn add(&mut self, text: &str) -> Result<usize, NoteError> {
        if self.is_full() {
            return Err(NoteError::Full);
        }
        let note = Note::from_str(text)?;
        let Some((index, slot)) = self
            .notes
            .iter_mut()
            .enumerate()
            .find(|(_, slot)| slot.is_none())
        else {
            return Err(NoteError::Full);
        };
        *slot = Some(note);
        self.count += 1;
        Ok(index)
    }

    /// Returns the note at the given slot.
    pub fn get(&self, index: usize) -> Result<&Note, NoteError> {
        if index >= MAX_NOTES {
            return Err(NoteError::InvalidIndex);
        }
        self.notes[index].as_ref().ok_or(NoteError::NotFound)
    }

    /// Returns the note at the given slot, or None if empty.
    pub fn slot(&self, index: usize) -> Result<Option<&Note>, NoteError> {
        if index >= MAX_NOTES {
            return Err(NoteError::InvalidIndex);
        }
        Ok(self.notes[index].as_ref())
    }

    /// Removes a note from the given slot.
    pub fn remove(&mut self, index: usize) -> Result<(), NoteError> {
        if index >= MAX_NOTES {
            return Err(NoteError::InvalidIndex);
        }
        if self.notes[index].is_none() {
            return Err(NoteError::NotFound);
        }
        self.notes[index] = None;
        self.count = self.count.saturating_sub(1);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_starts_empty() {
        let store = NoteStore::new();
        assert_eq!(store.count(), 0);
        assert!(!store.is_full());
        assert_eq!(store.slot(0).unwrap(), None);
    }

    #[test]
    fn add_and_get_note() {
        let mut store = NoteStore::new();
        let index = store.add("hello").expect("add should succeed");
        assert_eq!(index, 0);
        assert_eq!(store.count(), 1);
        let note = store.get(index).expect("note should exist");
        assert_eq!(note.as_str().unwrap(), "hello");
        assert_eq!(note.len(), 5);
    }

    #[test]
    fn add_rejects_empty_note() {
        let mut store = NoteStore::new();
        assert_eq!(store.add(""), Err(NoteError::Empty));
    }

    #[test]
    fn add_rejects_too_long_note() {
        let mut store = NoteStore::new();
        let text = "a".repeat(NOTE_LEN + 1);
        assert_eq!(store.add(&text), Err(NoteError::TooLong));
    }

    #[test]
    fn remove_clears_slot() {
        let mut store = NoteStore::new();
        let index = store.add("one").unwrap();
        assert_eq!(store.remove(index), Ok(()));
        assert_eq!(store.count(), 0);
        assert_eq!(store.slot(index).unwrap(), None);
    }

    #[test]
    fn remove_rejects_missing_slot() {
        let mut store = NoteStore::new();
        assert_eq!(store.remove(0), Err(NoteError::NotFound));
    }

    #[test]
    fn reject_invalid_index() {
        let mut store = NoteStore::new();
        assert_eq!(store.get(MAX_NOTES), Err(NoteError::InvalidIndex));
        assert_eq!(store.remove(MAX_NOTES), Err(NoteError::InvalidIndex));
        assert_eq!(store.slot(MAX_NOTES), Err(NoteError::InvalidIndex));
    }

    #[test]
    fn store_reports_full() {
        let mut store = NoteStore::new();
        for i in 0..MAX_NOTES {
            let label = format!("n{}", i);
            store.add(&label).expect("should insert");
        }
        assert!(store.is_full());
        assert_eq!(store.add("extra"), Err(NoteError::Full));
    }

    #[test]
    fn add_reports_full_when_slots_taken() {
        let note = Note::from_str("x").unwrap();
        let mut store = NoteStore {
            notes: [Some(note); MAX_NOTES],
            count: 0,
        };
        assert!(!store.is_full());
        assert_eq!(store.add("y"), Err(NoteError::Full));
    }

    #[test]
    fn note_rejects_invalid_utf8() {
        let mut buf = [0u8; NOTE_LEN];
        buf[0] = 0xFF;
        let note = Note { len: 1, buf };
        assert_eq!(note.as_str(), Err(NoteError::InvalidUtf8));
    }
}
