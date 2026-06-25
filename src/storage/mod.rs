use std::path::{Path, PathBuf};
use rusqlite::{Connection, params};
use chrono::Utc;

use crate::error::SrrResult;
use crate::types::{Symbol, Session};

const SRR_DIR: &str = ".srr";

pub struct StorageManager {
    db: Connection,
}

impl StorageManager {
    pub fn open(project_path: &Path) -> SrrResult<Self> {
        let srr_path = project_path.join(SRR_DIR);
        if !srr_path.exists() {
            std::fs::create_dir_all(&srr_path)?;
        }
        let db_path = srr_path.join("index.db");
        let db = Connection::open(&db_path)?;
        let mut mgr = Self { db };
        mgr.init_schema()?;
        Ok(mgr)
    }

    fn init_schema(&mut self) -> SrrResult<()> {
        self.db.execute_batch(
            "CREATE TABLE IF NOT EXISTS index_meta (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS files (
                id        INTEGER PRIMARY KEY AUTOINCREMENT,
                path      TEXT UNIQUE NOT NULL,
                checksum  TEXT,
                indexed_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS symbols (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                file_id    INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
                name       TEXT NOT NULL,
                kind       TEXT NOT NULL,
                line       INTEGER NOT NULL,
                column     INTEGER NOT NULL,
                signature  TEXT,
                doc_comment TEXT
            );
            CREATE VIRTUAL TABLE IF NOT EXISTS symbols_fts USING fts5(
                name, signature, doc_comment, content='symbols', content_rowid='id'
            );
            CREATE TRIGGER IF NOT EXISTS symbols_ai AFTER INSERT ON symbols BEGIN
                INSERT INTO symbols_fts(rowid, name, signature, doc_comment)
                VALUES (new.id, new.name, new.signature, new.doc_comment);
            END;
            CREATE TRIGGER IF NOT EXISTS symbols_ad AFTER DELETE ON symbols BEGIN
                INSERT INTO symbols_fts(symbols_fts, rowid, name, signature, doc_comment)
                VALUES ('delete', old.id, old.name, old.signature, old.doc_comment);
            END;
            CREATE TRIGGER IF NOT EXISTS symbols_au AFTER UPDATE ON symbols BEGIN
                INSERT INTO symbols_fts(symbols_fts, rowid, name, signature, doc_comment)
                VALUES ('delete', old.id, old.name, old.signature, old.doc_comment);
                INSERT INTO symbols_fts(rowid, name, signature, doc_comment)
                VALUES (new.id, new.name, new.signature, new.doc_comment);
            END;
            CREATE TABLE IF NOT EXISTS sessions (
                id          TEXT PRIMARY KEY,
                project     TEXT NOT NULL,
                task        TEXT,
                messages    TEXT NOT NULL DEFAULT '[]',
                created_at  TEXT NOT NULL,
                updated_at  TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS embeddings (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                symbol_id  INTEGER NOT NULL REFERENCES symbols(id) ON DELETE CASCADE,
                embedding  BLOB NOT NULL,
                model      TEXT NOT NULL
            );"
        )?;
        Ok(())
    }

    pub fn store_symbols(&self, symbols: &[Symbol], checksums: &[(PathBuf, String)]) -> SrrResult<()> {
        let tx = self.db.unchecked_transaction()?;
        for (path, cksum) in checksums {
            let now = Utc::now().to_rfc3339();
            tx.execute(
                "INSERT OR REPLACE INTO files (path, checksum, indexed_at) VALUES (?1, ?2, ?3)",
                params![path.to_string_lossy().to_string(), cksum, now],
            )?;
        }
        for sym in symbols {
            let path_str = sym.file_path.to_string_lossy().to_string();
            if let Ok(file_id) = tx.query_row(
                "SELECT id FROM files WHERE path = ?1", params![path_str], |r| r.get::<_, i64>(0)
            ) {
                tx.execute(
                    "INSERT INTO symbols (file_id, name, kind, line, column, signature, doc_comment)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        file_id, &sym.name, sym.kind.as_str(),
                        sym.line as i64, sym.column as i64,
                        sym.signature, sym.doc_comment,
                    ],
                )?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn search_symbols(&self, query: &str, limit: usize) -> SrrResult<Vec<Symbol>> {
        let mut stmt = self.db.prepare(
            "SELECT s.name, s.kind, f.path, s.line, s.column, s.signature, s.doc_comment
             FROM symbols_fts
             JOIN symbols s ON symbols_fts.rowid = s.id
             JOIN files f ON s.file_id = f.id
             WHERE symbols_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2"
        )?;
        let results = stmt.query_map(params![query, limit as i64], |r| {
            Ok(Symbol {
                name: r.get(0)?,
                kind: parse_symbol_kind(r.get::<_, String>(1)?.as_str()),
                file_path: PathBuf::from(r.get::<_, String>(2)?),
                line: r.get::<_, i64>(3)? as usize,
                column: r.get::<_, i64>(4)? as usize,
                signature: r.get(5)?,
                doc_comment: r.get(6)?,
            })
        })?;
        let mut out = Vec::new();
        for r in results {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn search_files(&self, query: &str, limit: usize) -> SrrResult<Vec<PathBuf>> {
        let mut stmt = self.db.prepare(
            "SELECT DISTINCT f.path
             FROM symbols_fts
             JOIN symbols s ON symbols_fts.rowid = s.id
             JOIN files f ON s.file_id = f.id
             WHERE symbols_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2"
        )?;
        let results = stmt.query_map(params![query, limit as i64], |r| {
            Ok(PathBuf::from(r.get::<_, String>(0)?))
        })?;
        let mut out = Vec::new();
        for r in results {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn symbol_count(&self) -> SrrResult<u64> {
        let count: i64 = self.db.query_row(
            "SELECT COUNT(*) FROM symbols", [], |r| r.get(0)
        )?;
        Ok(count as u64)
    }

    pub fn file_count(&self) -> SrrResult<u64> {
        let count: i64 = self.db.query_row(
            "SELECT COUNT(*) FROM files", [], |r| r.get(0)
        )?;
        Ok(count as u64)
    }

    pub fn get_indexed_checksums(&self) -> SrrResult<Vec<(PathBuf, String)>> {
        let mut stmt = self.db.prepare("SELECT path, checksum FROM files WHERE checksum IS NOT NULL")?;
        let results = stmt.query_map([], |r| {
            Ok((
                PathBuf::from(r.get::<_, String>(0)?),
                r.get::<_, String>(1)?,
            ))
        })?;
        let mut out = Vec::new();
        for r in results {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn save_session(&self, session: &Session) -> SrrResult<()> {
        let messages_json = serde_json::to_string(&session.messages).unwrap_or_default();
        self.db.execute(
            "INSERT OR REPLACE INTO sessions (id, project, task, messages, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                session.id,
                session.project_path.to_string_lossy().to_string(),
                session.task,
                messages_json,
                session.created_at.to_rfc3339(),
                session.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn load_sessions(&self, project_path: &Path) -> SrrResult<Vec<Session>> {
        let path_str = project_path.to_string_lossy().to_string();
        let mut stmt = self.db.prepare(
            "SELECT id, project, task, messages, created_at, updated_at
             FROM sessions WHERE project = ?1 ORDER BY updated_at DESC"
        )?;
        let results = stmt.query_map(params![path_str], |r| {
            let messages_str: String = r.get(3)?;
            let messages: Vec<AgentMessage> = serde_json::from_str(&messages_str).unwrap_or_default();
            Ok(Session {
                id: r.get(0)?,
                project_path: PathBuf::from(r.get::<_, String>(1)?),
                task: r.get(2)?,
                messages,
                created_at: r.get::<_, String>(4)?.parse().unwrap_or_else(|_| Utc::now()),
                updated_at: r.get::<_, String>(5)?.parse().unwrap_or_else(|_| Utc::now()),
            })
        })?;
        let mut out = Vec::new();
        for r in results {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn get_file_checksum(&self, path: &Path) -> SrrResult<Option<String>> {
        let path_str = path.to_string_lossy().to_string();
        let result = self.db.query_row(
            "SELECT checksum FROM files WHERE path = ?1",
            params![path_str],
            |r| r.get::<_, String>(0),
        );
        match result {
            Ok(cksum) => Ok(Some(cksum)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn delete_symbols_for_file(&self, path: &Path) -> SrrResult<()> {
        let path_str = path.to_string_lossy().to_string();
        self.db.execute(
            "DELETE FROM symbols WHERE file_id IN (SELECT id FROM files WHERE path = ?1)",
            params![path_str],
        )?;
        self.db.execute("DELETE FROM files WHERE path = ?1", params![path_str])?;
        Ok(())
    }

    pub fn delete_stale_files(&self, current_paths: &[PathBuf]) -> SrrResult<()> {
        for chunk in current_paths.chunks(999) {
            let placeholders: Vec<String> = chunk.iter().map(|_| "?".to_string()).collect();
            let sql = format!("DELETE FROM files WHERE path NOT IN ({})", placeholders.join(","));
            let mut stmt = self.db.prepare(&sql)?;
            let params: Vec<String> = chunk.iter().map(|p| p.to_string_lossy().to_string()).collect();
            let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
            stmt.execute(param_refs.as_slice())?;
        }
        Ok(())
    }

    pub fn clear_index(&self) -> SrrResult<()> {
        self.db.execute_batch(
            "DELETE FROM symbols_fts; DELETE FROM symbols; DELETE FROM files; DELETE FROM embeddings;"
        )?;
        Ok(())
    }

    pub fn store_embeddings(&self, symbol_ids: &[i64], embeddings: &[Vec<f32>], model: &str) -> SrrResult<()> {
        let tx = self.db.unchecked_transaction()?;
        for (&sym_id, emb) in symbol_ids.iter().zip(embeddings.iter()) {
            let bytes: Vec<u8> = emb.iter().flat_map(|f| f.to_le_bytes()).collect();
            tx.execute(
                "INSERT OR REPLACE INTO embeddings (symbol_id, embedding, model) VALUES (?1, ?2, ?3)",
                params![sym_id, bytes, model],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn get_embeddings(&self) -> SrrResult<Vec<(i64, Vec<f32>, String)>> {
        let mut stmt = self.db.prepare(
            "SELECT e.symbol_id, e.embedding, e.model FROM embeddings e"
        )?;
        let results = stmt.query_map([], |r| {
            let bytes: Vec<u8> = r.get(1)?;
            let dim = bytes.len() / 4;
            let mut emb = Vec::with_capacity(dim);
            for chunk in bytes.chunks(4) {
                if chunk.len() == 4 {
                    emb.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
                }
            }
            Ok((r.get(0)?, emb, r.get(2)?))
        })?;
        let mut out = Vec::new();
        for r in results {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn get_all_symbols_with_ids(&self) -> SrrResult<Vec<(i64, String, String, String)>> {
        let mut stmt = self.db.prepare(
            "SELECT s.id, s.name, s.signature, f.path
             FROM symbols s JOIN files f ON s.file_id = f.id"
        )?;
        let results = stmt.query_map([], |r| {
            Ok((
                r.get(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?.unwrap_or_default(),
                r.get::<_, String>(3)?,
            ))
        })?;
        let mut out = Vec::new();
        for r in results {
            out.push(r?);
        }
        Ok(out)
    }
}

fn parse_symbol_kind(s: &str) -> SymbolKind {
    match s {
        "function" => SymbolKind::Function,
        "method" => SymbolKind::Method,
        "struct" => SymbolKind::Struct,
        "trait" => SymbolKind::Trait,
        "enum" => SymbolKind::Enum,
        "type" => SymbolKind::Type,
        "module" => SymbolKind::Module,
        "class" => SymbolKind::Class,
        "interface" => SymbolKind::Interface,
        "variable" => SymbolKind::Variable,
        "constant" => SymbolKind::Constant,
        "macro" => SymbolKind::Macro,
        "import" => SymbolKind::Import,
        _ => SymbolKind::Type,
    }
}

use crate::types::{SymbolKind, AgentMessage};
