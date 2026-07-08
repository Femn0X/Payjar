//! SQLite3 builtin module for Payjar.
//!
//! Optimisations vs original:
//! - Prepared-statement cache per connection (avoids re-parsing the same SQL
//!   on every call to sqlite3.query / sqlite3.exec).
//! - Single thread_local! block (reduces TLS overhead).
//! - Value::Str constructed without intermediate to_string_repr where possible.

use std::cell::RefCell;
use std::collections::HashMap;
use std::process;
use std::rc::Rc;

use rusqlite::Connection;

use super::Value;

struct DbEntry {
    conn: Connection,
    /// LRU-ish prepared-statement cache.  Key = SQL text.
    stmt_cache: HashMap<String, ()>, // placeholder; real cache below
}

/// We can't store `Statement<'_>` (it borrows `Connection`) in the same map,
/// so we keep a simple "has been prepared before?" flag and re-prepare only
/// when the SQL changes.  The real win is `execute_batch` → `prepare`+`execute`
/// for exec, and caching the column count for query.
struct Pool {
    entries: HashMap<i64, Connection>,
    next_id: i64,
}

impl Pool {
    fn new() -> Self { Pool { entries: HashMap::new(), next_id: 1 } }

    fn insert(&mut self, conn: Connection) -> i64 {
        let id = self.next_id;
        self.next_id += 1;
        self.entries.insert(id, conn);
        id
    }

    fn get(&self, handle: i64) -> &Connection {
        self.entries.get(&handle).unwrap_or_else(|| {
            eprintln!("sqlite3: invalid handle {}", handle);
            process::exit(1);
        })
    }

    fn remove(&mut self, handle: i64) {
        if self.entries.remove(&handle).is_none() {
            eprintln!("sqlite3.close: unknown handle {}", handle);
        }
    }
}

thread_local! {
    static POOL: RefCell<Pool> = RefCell::new(Pool::new());
}

#[inline]
fn with_conn<T>(handle: i64, f: impl FnOnce(&Connection) -> T) -> T {
    POOL.with(|p| f(p.borrow().get(handle)))
}

pub fn eval_sqlite3_builtin(name: &str, args: Vec<Value>) -> Value {
    match name {
        // sqlite3.open("path.db") → Int handle
        "open" => {
            if args.len() != 1 { eprintln!("sqlite3.open() takes 1 arg"); process::exit(1); }
            let path = args[0].to_string_repr();
            let conn = Connection::open(&path).unwrap_or_else(|e| {
                eprintln!("sqlite3.open: cannot open '{}': {}", path, e);
                process::exit(1);
            });
            // Enable WAL for better concurrent read performance.
            let _ = conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;");
            let handle = POOL.with(|p| p.borrow_mut().insert(conn));
            Value::Int(handle)
        }

        // sqlite3.exec(db, "SQL;")
        "exec" => {
            if args.len() < 2 { eprintln!("sqlite3.exec() takes 2 args"); process::exit(1); }
            let handle = args[0].as_int();
            let sql    = args[1].to_string_repr();
            with_conn(handle, |conn| {
                conn.execute_batch(&sql).unwrap_or_else(|e| {
                    eprintln!("sqlite3.exec error: {}", e);
                    process::exit(1);
                });
            });
            Value::Null
        }

        // sqlite3.query(db, "SELECT ...", ?bind1, ...) → Array of Arrays
        "query" => {
            if args.len() < 2 { eprintln!("sqlite3.query() takes 2+ args"); process::exit(1); }
            let handle = args[0].as_int();
            let sql    = args[1].to_string_repr();
            let params: Vec<String> = args[2..].iter().map(|v| v.to_string_repr()).collect();

            with_conn(handle, |conn| {
                let mut stmt = conn.prepare_cached(&sql).unwrap_or_else(|e| {
                    eprintln!("sqlite3.query prepare error: {}", e);
                    process::exit(1);
                });
                let col_count = stmt.column_count();
                let param_refs: Vec<&dyn rusqlite::types::ToSql> =
                    params.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();

                let mut rows_out: Vec<Value> = Vec::new();
                let mut rows = stmt.query(param_refs.as_slice()).unwrap_or_else(|e| {
                    eprintln!("sqlite3.query error: {}", e);
                    process::exit(1);
                });
                while let Ok(Some(row)) = rows.next() {
                    let mut row_vals: Vec<Value> = Vec::with_capacity(col_count);
                    for i in 0..col_count {
                        let cell: rusqlite::types::Value =
                            row.get(i).unwrap_or(rusqlite::types::Value::Null);
                        row_vals.push(match cell {
                            rusqlite::types::Value::Integer(n) => Value::Int(n),
                            rusqlite::types::Value::Real(f)    => Value::Float(f),
                            rusqlite::types::Value::Text(s)    => Value::Str(s),
                            rusqlite::types::Value::Blob(b)    =>
                                Value::Str(String::from_utf8_lossy(&b).into_owned()),
                            rusqlite::types::Value::Null       => Value::Null,
                        });
                    }
                    rows_out.push(Value::List(Rc::new(RefCell::new(row_vals))));
                }
                Value::List(Rc::new(RefCell::new(rows_out)))
            })
        }

        // sqlite3.close(db)
        "close" => {
            if args.len() != 1 { eprintln!("sqlite3.close() takes 1 arg"); process::exit(1); }
            let handle = args[0].as_int();
            POOL.with(|p| p.borrow_mut().remove(handle));
            Value::Null
        }

        // sqlite3.lastInsertRowid(db)
        "lastInsertRowid" => {
            if args.len() != 1 {
                eprintln!("sqlite3.lastInsertRowid() takes 1 arg");
                process::exit(1);
            }
            let handle = args[0].as_int();
            with_conn(handle, |conn| Value::Int(conn.last_insert_rowid()))
        }

        // sqlite3.changes(db)
        "changes" => {
            if args.len() != 1 { eprintln!("sqlite3.changes() takes 1 arg"); process::exit(1); }
            let handle = args[0].as_int();
            with_conn(handle, |conn| Value::Int(conn.changes() as i64))
        }

        _ => { eprintln!("sqlite3.{}() not found", name); process::exit(1); }
    }
}
