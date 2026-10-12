//! Finished transfers, kept on disk so History survives restarts: `history.json` in the config
//! folder, newest first, at most [`MAX_ENTRIES`].
//!
//! A transfer gets its entry the first time it ends (done, failed, cancelled or declined). A failed
//! one that's tried again and finishes replaces its own entry, so each transfer shows up once.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::types::HistoryEntry;

pub const MAX_ENTRIES: usize = 500;

/// One entry plus what only the engine needs: where received files were saved, so History can open
/// them after a restart.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Record {
    #[serde(flatten)]
    pub entry: HistoryEntry,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub saved: Vec<PathBuf>,
}

#[derive(Debug)]
pub(crate) struct History {
    path: PathBuf,
    /// Newest first.
    records: Vec<Record>,
}

impl History {
    /// Reads `history.json`. A missing or damaged file starts an empty history: losing the list is
    /// better than not starting.
    pub fn open(dir: &Path) -> Self {
        let path = dir.join("history.json");
        let records = match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|e| {
                warn!("history.json is damaged, starting a new one: {e}");
                Vec::new()
            }),
            Err(_) => Vec::new(),
        };
        Self { path, records }
    }

    pub fn entries(&self) -> Vec<HistoryEntry> {
        self.records.iter().map(|r| r.entry.clone()).collect()
    }

    pub fn saved(&self, id: &str) -> Vec<PathBuf> {
        self.records.iter().find(|r| r.entry.id == id).map(|r| r.saved.clone()).unwrap_or_default()
    }

    /// Adds the entry, or replaces the one with the same id (keeping its place if it's newer).
    pub fn put(&mut self, record: Record) {
        self.records.retain(|r| r.entry.id != record.entry.id);
        let at = self
            .records
            .iter()
            .position(|r| r.entry.finished_at <= record.entry.finished_at)
            .unwrap_or(self.records.len());
        self.records.insert(at, record);
        self.records.truncate(MAX_ENTRIES);
        self.save();
    }

    pub fn clear(&mut self) {
        self.records.clear();
        self.save();
    }

    /// Writes atomically (a temp file, then rename). History is a convenience: a failed write is logged.
    fn save(&self) {
        let tmp = self.path.with_extension("json.tmp");
        let result = serde_json::to_vec(&self.records)
            .map_err(std::io::Error::other)
            .and_then(|json| std::fs::write(&tmp, json))
            .and_then(|_| std::fs::rename(&tmp, &self.path));
        if let Err(e) = result {
            warn!("couldn't save history: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        store::tests::tempdir,
        types::{DeviceKind, Route, TransferDirection, TransferStatus},
    };

    fn entry(id: &str, finished_at: u64) -> Record {
        Record {
            entry: HistoryEntry {
                id: id.into(),
                direction: TransferDirection::Receive,
                device_id: "d".into(),
                device_name: "Aisyah's phone".into(),
                device_kind: DeviceKind::Phone,
                files: vec![],
                total_bytes: 10,
                done_bytes: 10,
                route: Route::Nearby,
                status: TransferStatus::Done,
                error: None,
                cancelled_by: None,
                started_at: finished_at - 5,
                finished_at,
                average_speed: 2.0,
            },
            saved: vec![PathBuf::from("/tmp/a.txt")],
        }
    }

    #[test]
    fn keeps_newest_first_replaces_by_id_and_survives_a_restart() {
        let dir = tempdir("history");
        let mut h = History::open(&dir);
        h.put(entry("a", 100));
        h.put(entry("b", 300));
        h.put(entry("c", 200));
        let ids = |h: &History| h.entries().into_iter().map(|e| e.id).collect::<Vec<_>>();
        assert_eq!(ids(&h), ["b", "c", "a"]);

        let mut retried = entry("a", 400);
        retried.entry.status = TransferStatus::Done;
        h.put(retried);
        assert_eq!(ids(&h), ["a", "b", "c"], "a retried transfer moves up and isn't listed twice");

        let reopened = History::open(&dir);
        assert_eq!(ids(&reopened), ["a", "b", "c"]);
        assert_eq!(reopened.saved("a"), [PathBuf::from("/tmp/a.txt")]);

        let mut h = reopened;
        h.clear();
        assert!(History::open(&dir).entries().is_empty());
    }

    #[test]
    fn damaged_file_starts_empty_and_size_is_capped() {
        let dir = tempdir("history-cap");
        std::fs::write(dir.join("history.json"), b"{not json").unwrap();
        let mut h = History::open(&dir);
        assert!(h.entries().is_empty());
        for i in 0..(MAX_ENTRIES as u64 + 20) {
            h.put(entry(&format!("t{i}"), 10 + i));
        }
        assert_eq!(h.entries().len(), MAX_ENTRIES);
        assert_eq!(h.entries()[0].id, format!("t{}", MAX_ENTRIES + 19), "the newest are kept");
    }

    #[test]
    fn saved_paths_stay_out_of_the_contract() {
        let v = serde_json::to_value(&entry("a", 9).entry).unwrap();
        assert!(v.get("saved").is_none());
        assert_eq!(v["deviceName"], "Aisyah's phone");
        assert_eq!(v["averageSpeed"], 2.0);
        assert_eq!(v["finishedAt"], 9);
    }
}
