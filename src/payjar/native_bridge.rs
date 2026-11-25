//! Native ABI v1: a UTF-8 JSON request and callback-owned response.
//! Plugins are trusted executable code, not sandboxed.
use super::{Value, Rc, RefCell};
use serde_json::{Value as Json, json};
use libloading::Library;
use std::path::Path;
type Reply = unsafe extern "C" fn(*mut std::ffi::c_void, *const u8, usize);
type Call = unsafe extern "C" fn(*const u8, usize, Reply, *mut std::ffi::c_void) -> i32;
pub struct NativeModule { _library: Library, call: Call }
unsafe extern "C" fn receive(ctx: *mut std::ffi::c_void, data: *const u8, len: usize) {
    if data.is_null() || len > 16 * 1024 * 1024 { return; }
    let out = unsafe { &mut *ctx.cast::<Option<Vec<u8>>>() };
    *out = Some(unsafe { std::slice::from_raw_parts(data, len) }.to_vec());
}
fn encode(v: &Value, depth: usize) -> Result<Json, String> {
    if depth > 64 { return Err("Native argument nesting limit".into()); }
    Ok(match v {
        Value::Int(v) => json!(v), Value::Float(v) if v.is_finite() => json!(v),
        Value::Str(v) => json!(v), Value::Null => Json::Null,
        Value::List(v) => Json::Array(v.borrow().iter().map(|x| encode(x, depth+1)).collect::<Result<_,_>>()?),
        Value::Map(v) => Json::Object(v.borrow().iter().map(|(k,x)| Ok((k.clone(),encode(x,depth+1)?))).collect::<Result<_,String>>()?),
        _ => return Err("Unsupported native argument".into()),
    })
}
fn decode(v: Json) -> Value {
    match v {
        Json::Null => Value::Null, Json::Bool(v) => Value::Int(v as i64),
        Json::Number(n) => n.as_i64().map(Value::Int).unwrap_or_else(|| Value::Float(n.as_f64().unwrap_or_default())),
        Json::String(s) => Value::Str(s),
        Json::Array(a) => Value::List(Rc::new(RefCell::new(a.into_iter().map(decode).collect()))),
        Json::Object(m) => Value::Map(Rc::new(RefCell::new(m.into_iter().map(|(k,v)| (k,decode(v))).collect()))),
    }
}
impl NativeModule {
    pub fn load(path: &Path) -> Result<Self,String> {
        let library = unsafe { Library::new(path) }.map_err(|e| e.to_string())?;
        let call = unsafe { *library.get::<Call>(b"payjar_call_v1\0").map_err(|e| e.to_string())? };
        Ok(Self { _library: library, call })
    }
    pub fn call(&self, method: &str, args: &[Value]) -> Result<Value,String> {
        let args = args.iter().map(|x| encode(x,0)).collect::<Result<Vec<_>,_>>()?;
        let req = serde_json::to_vec(&json!({"method":method,"args":args})).map_err(|e| e.to_string())?;
        let mut reply: Option<Vec<u8>> = None;
        let status = unsafe { (self.call)(req.as_ptr(),req.len(),receive,(&mut reply as *mut Option<Vec<u8>>).cast()) };
        let result: Json = serde_json::from_slice(&reply.ok_or("Native plugin returned no response")?).map_err(|e| e.to_string())?;
        if status != 0 { return Err(result.as_str().unwrap_or("Native call failed").to_string()); }
        Ok(decode(result))
    }
}
