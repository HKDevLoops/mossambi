//! Undo command handler. The core has a persistent NDJSON action journal
//! (`core::action_journal`) with an `undo_last` reversal, so undo is real.
//! Redo has no core API and is reported N/A (see command help).
//! Pure ASCII by contract.

use anyhow::{bail, Result};
use serde_json::json;

use crate::core::action_journal::undo_last;

pub fn handle_undo(as_json: bool) -> Result<()> {
    match undo_last() {
        Ok(message) => {
            if as_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "status": "success", "operation": "undo", "result": message
                    }))?
                );
            } else {
                println!("Undo: {}", message);
            }
            Ok(())
        }
        Err(e) => bail!("undo failed: {}", e),
    }
}
