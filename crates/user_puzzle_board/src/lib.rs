#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Errors returned when modifying the puzzle board.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoardError {
    SlotNotFound,
    SlotAlreadyFilled,
    SlotNotCompatible,
}

/// Describes a slot on the puzzle board.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PuzzleSlot {
    pub name: String,
    pub required: bool,
    pub provider: Option<String>,
}

impl PuzzleSlot {
    /// Creates an empty puzzle slot.
    pub fn new(name: &str, required: bool) -> Self {
        Self {
            name: name.to_string(),
            required,
            provider: None,
        }
    }
}

/// Tracks which modules fill which slots.
#[derive(Debug, Clone, Default)]
pub struct PuzzleBoard {
    slots: BTreeMap<String, PuzzleSlot>,
}

impl PuzzleBoard {
    /// Builds a new board from the provided slots.
    pub fn new(slots: Vec<PuzzleSlot>) -> Self {
        let mut map = BTreeMap::new();
        for slot in slots {
            map.insert(slot.name.clone(), slot);
        }
        Self { slots: map }
    }

    /// Returns the slot list sorted by name.
    pub fn list(&self) -> Vec<PuzzleSlot> {
        self.slots.values().cloned().collect()
    }

    /// Returns true if all required slots are filled.
    pub fn is_complete(&self) -> bool {
        self.slots
            .values()
            .filter(|slot| slot.required)
            .all(|slot| slot.provider.is_some())
    }

    /// Returns required slots that are still empty.
    pub fn missing_required(&self) -> Vec<String> {
        self.slots
            .values()
            .filter(|slot| slot.required && slot.provider.is_none())
            .map(|slot| slot.name.clone())
            .collect()
    }

    /// Returns the provider bound to a slot, if any.
    pub fn provider_for(&self, slot: &str) -> Option<&str> {
        self.slots
            .get(slot)
            .and_then(|entry| entry.provider.as_deref())
    }

    /// Plugs a module into a slot if it declares compatibility.
    pub fn plug(
        &mut self,
        slot: &str,
        module: &str,
        module_slots: &[String],
    ) -> Result<(), BoardError> {
        let entry = self.slots.get_mut(slot).ok_or(BoardError::SlotNotFound)?;
        if entry.provider.is_some() {
            return Err(BoardError::SlotAlreadyFilled);
        }
        if !module_slots.iter().any(|item| item == slot) {
            return Err(BoardError::SlotNotCompatible);
        }
        entry.provider = Some(module.to_string());
        Ok(())
    }

    /// Removes the module from a slot.
    pub fn unplug(&mut self, slot: &str) -> Result<Option<String>, BoardError> {
        let entry = self.slots.get_mut(slot).ok_or(BoardError::SlotNotFound)?;
        Ok(entry.provider.take())
    }

    /// Seeds the board with an already running module.
    pub fn mark_running(&mut self, module: &str, module_slots: &[String]) {
        for slot in module_slots {
            if let Some(entry) = self.slots.get_mut(slot) {
                if entry.provider.is_none() {
                    entry.provider = Some(module.to_string());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn board() -> PuzzleBoard {
        PuzzleBoard::new(vec![
            PuzzleSlot::new("ruzzle.slot.console", true),
            PuzzleSlot::new("ruzzle.slot.shell", true),
            PuzzleSlot::new("ruzzle.slot.net", false),
        ])
    }

    #[test]
    fn board_tracks_required_slots() {
        let mut board = board();
        assert!(!board.is_complete());
        assert_eq!(
            board.missing_required(),
            vec!["ruzzle.slot.console".to_string(), "ruzzle.slot.shell".to_string()]
        );

        board
            .plug(
                "ruzzle.slot.console",
                "console-service",
                &["ruzzle.slot.console".to_string()],
            )
            .unwrap();
        assert!(!board.is_complete());

        board
            .plug(
                "ruzzle.slot.shell",
                "tui-shell",
                &["ruzzle.slot.shell".to_string()],
            )
            .unwrap();
        assert!(board.is_complete());
    }

    #[test]
    fn plug_rejects_missing_slot() {
        let mut board = board();
        let result = board.plug("ruzzle.slot.fs", "fs-service", &[]);
        assert_eq!(result, Err(BoardError::SlotNotFound));
    }

    #[test]
    fn plug_rejects_incompatible_module() {
        let mut board = board();
        let result = board.plug(
            "ruzzle.slot.console",
            "console-service",
            &["ruzzle.slot.shell".to_string()],
        );
        assert_eq!(result, Err(BoardError::SlotNotCompatible));
    }

    #[test]
    fn plug_rejects_filled_slot() {
        let mut board = board();
        board
            .plug(
                "ruzzle.slot.console",
                "console-service",
                &["ruzzle.slot.console".to_string()],
            )
            .unwrap();
        let result = board.plug(
            "ruzzle.slot.console",
            "alt-console",
            &["ruzzle.slot.console".to_string()],
        );
        assert_eq!(result, Err(BoardError::SlotAlreadyFilled));
    }

    #[test]
    fn unplug_clears_provider() {
        let mut board = board();
        board
            .plug(
                "ruzzle.slot.net",
                "net-service",
                &["ruzzle.slot.net".to_string()],
            )
            .unwrap();
        let removed = board.unplug("ruzzle.slot.net").unwrap();
        assert_eq!(removed, Some("net-service".to_string()));
        assert_eq!(board.unplug("ruzzle.slot.net").unwrap(), None);
    }

    #[test]
    fn provider_for_reports_active_provider() {
        let mut board = board();
        assert_eq!(board.provider_for("ruzzle.slot.net"), None);
        board
            .plug(
                "ruzzle.slot.net",
                "net-service",
                &["ruzzle.slot.net".to_string()],
            )
            .unwrap();
        assert_eq!(board.provider_for("ruzzle.slot.net"), Some("net-service"));
    }

    #[test]
    fn unplug_rejects_missing_slot() {
        let mut board = board();
        let result = board.unplug("ruzzle.slot.missing");
        assert_eq!(result, Err(BoardError::SlotNotFound));
    }

    #[test]
    fn mark_running_ignores_missing_slot() {
        let mut board = board();
        board.mark_running(
            "console-service",
            &["ruzzle.slot.unknown".to_string()],
        );
        assert!(board.list().iter().all(|slot| slot.provider.is_none()));
    }

    #[test]
    fn mark_running_does_not_overwrite_provider() {
        let mut board = board();
        board
            .plug(
                "ruzzle.slot.console",
                "console-service",
                &["ruzzle.slot.console".to_string()],
            )
            .unwrap();
        board.mark_running(
            "alt-console",
            &["ruzzle.slot.console".to_string()],
        );
        let slot = board
            .list()
            .into_iter()
            .find(|slot| slot.name == "ruzzle.slot.console")
            .expect("slot should exist");
        assert_eq!(slot.provider, Some("console-service".to_string()));
    }

    #[test]
    fn mark_running_seeds_slots() {
        let mut board = board();
        board.mark_running(
            "console-service",
            &["ruzzle.slot.console".to_string()],
        );
        let slots = board.list();
        let console = slots
            .iter()
            .find(|slot| slot.name == "ruzzle.slot.console")
            .unwrap();
        assert_eq!(console.provider.as_deref(), Some("console-service"));
    }
}
