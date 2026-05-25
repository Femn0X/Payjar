//! SQLite3 builtin module for Payjar.
//! Uses thread_local storage so Connection (non-Send) stays on the main thread.

use std::cell::RefCell;
use std::collections::HashMap;
use std::process;
use std::rc::Rc;
use std::cell::RefCell as RC;

use rusqlite::Connection;

use super::Value;

thread_local! {
    static DB_POOL: RefCell<HashMap<i64, Connection>> = RefCell::new(HashMap::new());
    static DB_NEXT: RefCell<i64> = RefCell::new(1);
}

pub fn eval_sqlite3_builtin(name: &str, args: Vec<Value>) -> Value {
    match name {
        // sqlite3.open("path.db")  →  Int handle
        "open" => {
            if args.len() != 1 { eprintln!("sqlite3.open() takes 1 arg"); process::exit(1); }
            let path = args[0].to_string_repr();
            let conn = Connection::open(&path).unwrap_or_else(|e| {
                eprintln!("sqlite3.open: cannot open '{}': {}", path, e); process::exit(1);
            });
            DB_POOL.with(|pool| {
                DB_NEXT.with(|next| {
                    let handle = *next.borrow();
                    *next.borrow_mut() += 1;
                    pool.borrow_mut().insert(handle, conn);
                    Value::Int(handle)
                })
            })
        }

        // sqlite3.exec(db, "SQL;")
        "exec" => {
            if args.len() < 2 { eprintln!("sqlite3.exec() takes 2 args"); process::exit(1); }
            let handle = args[0].as_int();
            let sql    = args[1].to_string_repr();
            DB_POOL.with(|pool| {
                let pool = pool.borrow();
                let conn = pool.get(&handle).unwrap_or_else(|| {
                    eprintln!("sqlite3.exec: invalid handle {}", handle); process::exit(1);
                });
                conn.execute_batch(&sql).unwrap_or_else(|e| {
                    eprintln!("sqlite3.exec error: {}", e); process::exit(1);
                });
                Value::Null
            })
        }

        // sqlite3.query(db, "SELECT ...", ?bind1, ?bind2, ...)  →  Array* of Array*
        "query" => {
            if args.len() < 2 { eprintln!("sqlite3.query() takes 2+ args"); process::exit(1); }
            let handle = args[0].as_int();
            let sql    = args[1].to_string_repr();
            let params: Vec<String> = args[2..].iter().map(|v| v.to_string_repr()).collect();

            DB_POOL.with(|pool| {
                let pool = pool.borrow();
                let conn = pool.get(&handle).unwrap_or_else(|| {
                    eprintln!("sqlite3.query: invalid handle {}", handle); process::exit(1);
                });
                let mut stmt = conn.prepare(&sql).unwrap_or_else(|e| {
                    eprintln!("sqlite3.query prepare error: {}", e); process::exit(1);
                });
                let col_count = stmt.column_count();
                let param_refs: Vec<&dyn rusqlite::types::ToSql> =
                    params.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();

                let mut rows_out: Vec<Value> = Vec::new();
                let mut rows = stmt.query(param_refs.as_slice()).unwrap_or_else(|e| {
                    eprintln!("sqlite3.query error: {}", e); process::exit(1);
                });
                while let Ok(Some(row)) = rows.next() {
                    let mut row_vals: Vec<Value> = Vec::new();
                    for i in 0..col_count {
                        let cell: rusqlite::types::Value = row.get(i)
                            .unwrap_or(rusqlite::types::Value::Null);
                        row_vals.push(match cell {
                            rusqlite::types::Value::Integer(n) => Value::Int(n),
                            rusqlite::types::Value::Real(f)    => Value::Float(f),
                            rusqlite::types::Value::Text(s)    => Value::Str(s),
                            rusqlite::types::Value::Blob(b)    => Value::Str(String::from_utf8_lossy(&b).to_string()),
                            rusqlite::types::Value::Null       => Value::Null,
                        });
                    }
                    rows_out.push(Value::List(Rc::new(RC::new(row_vals))));
                }
                Value::List(Rc::new(RC::new(rows_out)))
            })
        }

        // sqlite3.close(db)
        "close" => {
            if args.len() != 1 { eprintln!("sqlite3.close() takes 1 arg"); process::exit(1); }
            let handle = args[0].as_int();
            DB_POOL.with(|pool| {
                if pool.borrow_mut().remove(&handle).is_none() {
                    eprintln!("sqlite3.close: unknown handle {}", handle);
                }
                Value::Null
            })
        }

        // sqlite3.lastInsertRowid(db)
        "lastInsertRowid" => {
            if args.len() != 1 { eprintln!("sqlite3.lastInsertRowid() takes 1 arg"); process::exit(1); }
            let handle = args[0].as_int();
            DB_POOL.with(|pool| {
                let pool = pool.borrow();
                let conn = pool.get(&handle).unwrap_or_else(|| {
                    eprintln!("sqlite3.lastInsertRowid: invalid handle {}", handle); process::exit(1);
                });
                Value::Int(conn.last_insert_rowid())
            })
        }

        // sqlite3.changes(db)
        "changes" => {
            if args.len() != 1 { eprintln!("sqlite3.changes() takes 1 arg"); process::exit(1); }
            let handle = args[0].as_int();
            DB_POOL.with(|pool| {
                let pool = pool.borrow();
                let conn = pool.get(&handle).unwrap_or_else(|| {
                    eprintln!("sqlite3.changes: invalid handle {}", handle); process::exit(1);
                });
                Value::Int(conn.changes() as i64)
            })
        }

        _ => { eprintln!("sqlite3.{}() not found", name); process::exit(1); }
    }
}
