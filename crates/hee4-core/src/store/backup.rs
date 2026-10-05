//! The SQL half of the backup door: one consistent online snapshot by `VACUUM INTO`. The
//! non-SQL half (`crate::backup`) copies objects, writes the manifest last and restores.

use std::path::Path;

use super::{Store, StoreError};

impl Store {
    /// Write a consistent snapshot of the committed state (as of statement start) to `dest`
    /// with `VACUUM INTO`: safe under concurrent writers in WAL, never a file copy of a live
    /// WAL ledger. `dest` must not exist yet.
    ///
    /// # Errors
    /// [`StoreError::Io`] for a path that is not UTF-8; SQLite errors (an existing `dest`
    /// among them).
    pub fn snapshot_into(&self, dest: &Path) -> Result<(), StoreError> {
        let target = dest.to_str().ok_or_else(|| StoreError::Io {
            path: dest.display().to_string(),
            source: std::io::Error::new(std::io::ErrorKind::InvalidInput, "path is not UTF-8"),
        })?;
        self.conn.execute("VACUUM INTO ?1", [target])?;
        Ok(())
    }
}
