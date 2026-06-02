//! Historia transkrypcji: SQLite z FTS5 do pełnotekstowego wyszukiwania.

use std::path::Path;

use parking_lot::Mutex;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: i64,
    pub text: String,
    pub created_at: String,
    pub audio_path: Option<String>,
    pub language: Option<String>,
    pub duration_ms: i64,
}

pub struct HistoryStore {
    conn: Mutex<Connection>,
}

/// Zamienia polskie litery diakrytyczne na ich odpowiedniki bez ogonków,
/// żeby FTS5 (który nie ma wbudowanego "polskiego" stemmera) mógł matchować
/// `łódź` z `lodz` itp. Nie zmienia oryginału w tabeli `history` -
/// tylko tekst wprowadzany do FTS.
fn normalize_for_fts(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        let replacement: char = match c {
            'ą' => 'a',
            'ć' => 'c',
            'ę' => 'e',
            'ł' => 'l',
            'ń' => 'n',
            'ó' => 'o',
            'ś' => 's',
            'ź' => 'z',
            'ż' => 'z',
            'Ą' => 'A',
            'Ć' => 'C',
            'Ę' => 'E',
            'Ł' => 'L',
            'Ń' => 'N',
            'Ó' => 'O',
            'Ś' => 'S',
            'Ź' => 'Z',
            'Ż' => 'Z',
            _ => c,
        };
        out.push(replacement);
    }
    out
}

impl HistoryStore {
    pub fn open(dir: &Path) -> Result<Self, rusqlite::Error> {
        let path = dir.join("history.sqlite");
        let conn = Connection::open(&path)?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                text TEXT NOT NULL,
                created_at TEXT NOT NULL,
                audio_path TEXT,
                language TEXT,
                duration_ms INTEGER NOT NULL DEFAULT 0
            );

            -- FTS jest niezależną tabelą (bez content='history')
            -- bo SQLite zabrania triggerom INSERT do 'content' FTS5.
            -- Zamiast tego: wstawiamy znormalizowany tekst z aplikacji (Rust).
            CREATE VIRTUAL TABLE IF NOT EXISTS history_fts USING fts5(
                text,
                tokenize='unicode61'
            );
            "#,
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn insert(
        &self,
        text: &str,
        audio_path: Option<&str>,
        language: Option<&str>,
        duration_ms: i64,
    ) -> Result<i64, rusqlite::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO history (text, created_at, audio_path, language, duration_ms) VALUES (?, ?, ?, ?, ?)",
            params![text, now, audio_path, language, duration_ms],
        )?;
        let id = conn.last_insert_rowid();
        // Mirror do FTS z normalizacją polskich znaków
        let normalized = normalize_for_fts(text);
        conn.execute(
            "INSERT INTO history_fts(rowid, text) VALUES (?, ?)",
            params![id, normalized],
        )?;
        Ok(id)
    }

    pub fn list(&self, query: Option<&str>, limit: i64) -> Result<Vec<HistoryEntry>, rusqlite::Error> {
        let conn = self.conn.lock();
        if let Some(q) = query.filter(|s| !s.trim().is_empty()) {
            // Sanityzacja FTS5 - escapujemy cudzysłowy i normalizujemy query
            let safe = normalize_for_fts(q).replace('"', "");
            let mut stmt = conn.prepare(
                "SELECT h.id, h.text, h.created_at, h.audio_path, h.language, h.duration_ms
                 FROM history_fts f
                 JOIN history h ON h.id = f.rowid
                 WHERE history_fts MATCH ?1
                 ORDER BY rank
                 LIMIT ?2",
            )?;
            let rows = stmt.query_map(params![safe, limit], Self::row_to_entry)?;
            rows.collect()
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, text, created_at, audio_path, language, duration_ms
                 FROM history
                 ORDER BY id DESC
                 LIMIT ?1",
            )?;
            let rows = stmt.query_map(params![limit], Self::row_to_entry)?;
            rows.collect()
        }
    }

    fn row_to_entry(row: &rusqlite::Row) -> rusqlite::Result<HistoryEntry> {
        Ok(HistoryEntry {
            id: row.get(0)?,
            text: row.get(1)?,
            created_at: row.get(2)?,
            audio_path: row.get(3)?,
            language: row.get(4)?,
            duration_ms: row.get(5)?,
        })
    }

    pub fn delete(&self, id: i64) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM history_fts WHERE rowid = ?", params![id])?;
        conn.execute("DELETE FROM history WHERE id = ?", params![id])?;
        Ok(())
    }

    pub fn audio_path(&self, id: i64) -> Result<Option<String>, rusqlite::Error> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT audio_path FROM history WHERE id = ?")?;
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            Ok(row.get(0)?)
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_removes_polish_chars() {
        assert_eq!(normalize_for_fts("Łódź"), "Lodz");
        assert_eq!(normalize_for_fts("Kraków"), "Krakow");
        assert_eq!(normalize_for_fts("Gdańsk"), "Gdansk");
        assert_eq!(normalize_for_fts("Ściana"), "Sciana");
        assert_eq!(normalize_for_fts("Żółw"), "Zolw");
        assert_eq!(normalize_for_fts("Białystok"), "Bialystok");
        assert_eq!(normalize_for_fts("hello world"), "hello world");
    }
}
