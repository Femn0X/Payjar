use std::process;
use std::io::{self, BufRead, Write};
use std::fs;
use std::rc::Rc;
use std::cell::RefCell;

use super::{Value, PJObject, Env, AstNode, call_func_with_vals};
pub use super::sqlite3::eval_sqlite3_builtin;

macro_rules! need {
    ($args:expr, $n:expr) => {
        if $args.len() != $n {
            eprintln!("Expected {} arg(s), got {}", $n, $args.len());
            process::exit(1);
        }
    };
}

// ============================================================
// MATH
// ============================================================
pub fn eval_math_builtin(name: &str, args: Vec<Value>) -> Value {
    match name {
        "pi"  => return Value::Float(std::f64::consts::PI),
        "e"   => return Value::Float(std::f64::consts::E),
        "tau" => return Value::Float(std::f64::consts::TAU),
        "inf" => return Value::Float(f64::INFINITY),
        "nan" => return Value::Float(f64::NAN),
        _ => {}
    }
    match name {
        "abs"   => { need!(args,1); match &args[0]{ Value::Int(i)=>Value::Int(i.abs()), _=>Value::Float(args[0].as_float().abs()) } }
        "sqrt"  => { need!(args,1); let x=args[0].as_float(); if x<0.0{eprintln!("math.sqrt: arg>=0");process::exit(1);} Value::Float(x.sqrt()) }
        "cbrt"  => { need!(args,1); Value::Float(args[0].as_float().cbrt()) }
        "ceil"  => { need!(args,1); Value::Float(args[0].as_float().ceil()) }
        "round" => { need!(args,1); Value::Float(args[0].as_float().round()) }
        "trunc" => { need!(args,1); Value::Float(args[0].as_float().trunc()) }
        "fract" => { need!(args,1); Value::Float(args[0].as_float().fract()) }
        "sign"  => { need!(args,1); let x=args[0].as_float(); if x>0.0{Value::Int(1)}else if x<0.0{Value::Int(-1)}else{Value::Int(0)} }
        "floor" => match args.len() {
            1 => Value::Float(args[0].as_float().floor()),
            2 => { let a=args[0].as_int(); let b=args[1].as_int(); if b==0{eprintln!("math.floor: div by zero");process::exit(1);} Value::Int(a.div_euclid(b)) }
            _ => { eprintln!("math.floor() takes 1 or 2 args"); process::exit(1); }
        },
        "pow"   => { need!(args,2); if let(Value::Int(b),Value::Int(e))=(&args[0],&args[1]){if *e>=0{return Value::Int(b.pow(*e as u32));}} Value::Float(args[0].as_float().powf(args[1].as_float())) }
        "exp"   => { need!(args,1); Value::Float(args[0].as_float().exp()) }
        "ln"    => { need!(args,1); let x=args[0].as_float(); if x<=0.0{eprintln!("math.ln: arg>0");process::exit(1);} Value::Float(x.ln()) }
        "log"   => { need!(args,2); let x=args[0].as_float(); let b=args[1].as_float(); if x<=0.0||b<=0.0||b==1.0{eprintln!("math.log: invalid");process::exit(1);} Value::Float(x.log(b)) }
        "log2"  => { need!(args,1); let x=args[0].as_float(); if x<=0.0{eprintln!("math.log2: arg>0");process::exit(1);} Value::Float(x.log2()) }
        "log10" => { need!(args,1); let x=args[0].as_float(); if x<=0.0{eprintln!("math.log10: arg>0");process::exit(1);} Value::Float(x.log10()) }
        "sin"   => { need!(args,1); Value::Float(args[0].as_float().sin()) }
        "cos"   => { need!(args,1); Value::Float(args[0].as_float().cos()) }
        "tan"   => { need!(args,1); Value::Float(args[0].as_float().tan()) }
        "asin"  => { need!(args,1); Value::Float(args[0].as_float().asin()) }
        "acos"  => { need!(args,1); Value::Float(args[0].as_float().acos()) }
        "atan"  => { need!(args,1); Value::Float(args[0].as_float().atan()) }
        "atan2" => { need!(args,2); Value::Float(args[0].as_float().atan2(args[1].as_float())) }
        "sinh"  => { need!(args,1); Value::Float(args[0].as_float().sinh()) }
        "cosh"  => { need!(args,1); Value::Float(args[0].as_float().cosh()) }
        "tanh"  => { need!(args,1); Value::Float(args[0].as_float().tanh()) }
        "toRad" => { need!(args,1); Value::Float(args[0].as_float().to_radians()) }
        "toDeg" => { need!(args,1); Value::Float(args[0].as_float().to_degrees()) }
        "hypot" => { need!(args,2); Value::Float(args[0].as_float().hypot(args[1].as_float())) }
        "isNan" => { need!(args,1); Value::Int(if args[0].as_float().is_nan(){1}else{0}) }
        "isInf" => { need!(args,1); Value::Int(if args[0].as_float().is_infinite(){1}else{0}) }
        "min"   => { if args.is_empty(){eprintln!("math.min: need args");process::exit(1);} let vals:Vec<f64>=if args.len()==1{if let Value::List(l)=&args[0]{l.borrow().iter().map(|v|v.as_float()).collect()}else{vec![args[0].as_float()]}}else{args.iter().map(|v|v.as_float()).collect()}; Value::Float(vals.into_iter().fold(f64::INFINITY,f64::min)) }
        "max"   => { if args.is_empty(){eprintln!("math.max: need args");process::exit(1);} let vals:Vec<f64>=if args.len()==1{if let Value::List(l)=&args[0]{l.borrow().iter().map(|v|v.as_float()).collect()}else{vec![args[0].as_float()]}}else{args.iter().map(|v|v.as_float()).collect()}; Value::Float(vals.into_iter().fold(f64::NEG_INFINITY,f64::max)) }
        "sum"   => { if args.is_empty(){eprintln!("math.sum: need args");process::exit(1);} let vals:Vec<f64>=if args.len()==1{if let Value::List(l)=&args[0]{l.borrow().iter().map(|v|v.as_float()).collect()}else{vec![args[0].as_float()]}}else{args.iter().map(|v|v.as_float()).collect()}; let t:f64=vals.iter().sum(); if vals.iter().all(|x|x.fract()==0.0){Value::Int(t as i64)}else{Value::Float(t)} }
        "clamp" => { need!(args,3); Value::Float(args[0].as_float().clamp(args[1].as_float(),args[2].as_float())) }
        "lerp"  => { need!(args,3); let a=args[0].as_float();let b=args[1].as_float();let t=args[2].as_float(); Value::Float(a+(b-a)*t) }
        "gcd"   => { need!(args,2); let mut a=args[0].as_int().unsigned_abs();let mut b=args[1].as_int().unsigned_abs(); while b!=0{let t=b;b=a%b;a=t;} Value::Int(a as i64) }
        "lcm"   => { need!(args,2); let a=args[0].as_int().unsigned_abs();let b=args[1].as_int().unsigned_abs(); if a==0||b==0{return Value::Int(0);} let mut ga=a;let mut gb=b; while gb!=0{let t=gb;gb=ga%gb;ga=t;} Value::Int((a/ga*b) as i64) }
        _ => { eprintln!("math.{}() not found", name); process::exit(1); }
    }
}

// ============================================================
// CMATH  (complex number math)
// ============================================================
pub fn eval_cmath_builtin(name: &str, args: Vec<Value>) -> Value {
    // Represent complex as [real, imag] list for simplicity
    fn mk(r: f64, i: f64) -> Value {
        Value::List(Rc::new(RefCell::new(vec![Value::Float(r), Value::Float(i)])))
    }
    fn unpack(v: &Value) -> (f64, f64) {
        match v {
            Value::List(l) => {
                let b = l.borrow();
                (b.get(0).map(|x| x.as_float()).unwrap_or(0.0),
                 b.get(1).map(|x| x.as_float()).unwrap_or(0.0))
            }
            other => (other.as_float(), 0.0),
        }
    }
    match name {
        "complex" => { need!(args,2); mk(args[0].as_float(), args[1].as_float()) }
        "real"    => { need!(args,1); let (r,_)=unpack(&args[0]); Value::Float(r) }
        "imag"    => { need!(args,1); let (_,i)=unpack(&args[0]); Value::Float(i) }
        "abs"     => { need!(args,1); let (r,i)=unpack(&args[0]); Value::Float((r*r+i*i).sqrt()) }
        "phase"   => { need!(args,1); let (r,i)=unpack(&args[0]); Value::Float(i.atan2(r)) }
        "conj"    => { need!(args,1); let (r,i)=unpack(&args[0]); mk(r,-i) }
        "add"     => { need!(args,2); let (ar,ai)=unpack(&args[0]);let (br,bi)=unpack(&args[1]); mk(ar+br,ai+bi) }
        "sub"     => { need!(args,2); let (ar,ai)=unpack(&args[0]);let (br,bi)=unpack(&args[1]); mk(ar-br,ai-bi) }
        "mul"     => { need!(args,2); let (ar,ai)=unpack(&args[0]);let (br,bi)=unpack(&args[1]); mk(ar*br-ai*bi, ar*bi+ai*br) }
        "div"     => { need!(args,2); let (ar,ai)=unpack(&args[0]);let (br,bi)=unpack(&args[1]); let d=br*br+bi*bi; if d==0.0{eprintln!("cmath.div: division by zero");process::exit(1);} mk((ar*br+ai*bi)/d,(ai*br-ar*bi)/d) }
        "sqrt"    => { need!(args,1); let (r,i)=unpack(&args[0]); let m=(r*r+i*i).sqrt(); let sr=((m+r)/2.0).sqrt(); let si=if i<0.0{-((m-r)/2.0).sqrt()}else{((m-r)/2.0).sqrt()}; mk(sr,si) }
        "exp"     => { need!(args,1); let (r,i)=unpack(&args[0]); let er=r.exp(); mk(er*i.cos(),er*i.sin()) }
        "ln"      => { need!(args,1); let (r,i)=unpack(&args[0]); let m=(r*r+i*i).sqrt(); if m==0.0{eprintln!("cmath.ln: zero");process::exit(1);} mk(m.ln(),i.atan2(r)) }
        "pow"     => { need!(args,2); let (ar,ai)=unpack(&args[0]);let (br,bi)=unpack(&args[1]); let lnr=(ar*ar+ai*ai).sqrt().ln(); let th=ai.atan2(ar); let new_r=lnr*br-th*bi; let new_i=lnr*bi+th*br; let er=new_r.exp(); mk(er*new_i.cos(),er*new_i.sin()) }
        "sin"     => { need!(args,1); let (r,i)=unpack(&args[0]); mk(r.sin()*i.cosh(), r.cos()*i.sinh()) }
        "cos"     => { need!(args,1); let (r,i)=unpack(&args[0]); mk(r.cos()*i.cosh(),-r.sin()*i.sinh()) }
        "polar"   => { need!(args,2); let r=args[0].as_float();let theta=args[1].as_float(); mk(r*theta.cos(),r*theta.sin()) }
        "toString"=> { need!(args,1); let (r,i)=unpack(&args[0]); let sign=if i<0.0{"-"}else{"+"}; Value::Str(format!("{}{}{}i",r,sign,i.abs())) }
        _ => { eprintln!("cmath.{}() not found", name); process::exit(1); }
    }
}

// ============================================================
// JSON
// ============================================================
pub fn eval_json_builtin(name: &str, args: Vec<Value>) -> Value {
    fn val_to_json(v: &Value) -> serde_json::Value {
        match v {
            Value::Int(i)   => serde_json::Value::Number(serde_json::Number::from(*i)),
            Value::Float(f) => serde_json::Number::from_f64(*f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            Value::Str(s)   => serde_json::Value::String(s.clone()),
            Value::Null     => serde_json::Value::Null,
            Value::List(l)  => serde_json::Value::Array(l.borrow().iter().map(val_to_json).collect()),
            Value::Object(o)=> {
                let mut map = serde_json::Map::new();
                for f in &o.borrow().fields {
                    map.insert(f.name.clone(), val_to_json(&f.val));
                }
                serde_json::Value::Object(map)
            }
            _ => serde_json::Value::Null,
        }
    }
    fn json_to_val(j: &serde_json::Value) -> Value {
        match j {
            serde_json::Value::Null      => Value::Null,
            serde_json::Value::Bool(b)   => Value::Int(if *b {1} else {0}),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() { Value::Int(i) }
                else { Value::Float(n.as_f64().unwrap_or(0.0)) }
            }
            serde_json::Value::String(s) => Value::Str(s.clone()),
            serde_json::Value::Array(a)  => Value::List(Rc::new(RefCell::new(a.iter().map(json_to_val).collect()))),
            serde_json::Value::Object(o) => {
                // Return as list of [key, value] pairs (no class system needed)
                let pairs: Vec<Value> = o.iter().map(|(k, v)| {
                    Value::List(Rc::new(RefCell::new(vec![Value::Str(k.clone()), json_to_val(v)])))
                }).collect();
                Value::List(Rc::new(RefCell::new(pairs)))
            }
        }
    }
    match name {
        "stringify" => {
            need!(args,1);
            let pretty = args.len() >= 2 && args[1].as_int() != 0;
            let j = val_to_json(&args[0]);
            let s = if pretty {
                serde_json::to_string_pretty(&j).unwrap_or_default()
            } else {
                serde_json::to_string(&j).unwrap_or_default()
            };
            Value::Str(s)
        }
        "parse" => {
            need!(args,1);
            let s = args[0].to_string_repr();
            match serde_json::from_str::<serde_json::Value>(&s) {
                Ok(j)  => json_to_val(&j),
                Err(e) => { eprintln!("json.parse error: {}", e); process::exit(1); }
            }
        }
        "readFile" => {
            need!(args,1);
            let path = args[0].to_string_repr();
            let s = fs::read_to_string(&path).unwrap_or_else(|e| {
                eprintln!("json.readFile: cannot read '{}': {}", path, e); process::exit(1);
            });
            match serde_json::from_str::<serde_json::Value>(&s) {
                Ok(j)  => json_to_val(&j),
                Err(e) => { eprintln!("json.readFile parse error: {}", e); process::exit(1); }
            }
        }
        "writeFile" => {
            if args.len() < 2 { eprintln!("json.writeFile takes 2 args"); process::exit(1); }
            let path = args[0].to_string_repr();
            let pretty = args.len() >= 3 && args[2].as_int() != 0;
            let j = val_to_json(&args[1]);
            let s = if pretty { serde_json::to_string_pretty(&j) } else { serde_json::to_string(&j) }
                .unwrap_or_default();
            fs::write(&path, s).unwrap_or_else(|e| {
                eprintln!("json.writeFile: cannot write '{}': {}", path, e); process::exit(1);
            });
            Value::Null
        }
        _ => { eprintln!("json.{}() not found", name); process::exit(1); }
    }
}

// ============================================================
// OS
// ============================================================
pub fn eval_os_builtin(name: &str, args: Vec<Value>) -> Value {
    use std::process::Command;
    match name {
        "getcwd"  => Value::Str(std::env::current_dir().map(|p| p.display().to_string()).unwrap_or_default()),
        "chdir"   => { need!(args,1); std::env::set_current_dir(args[0].to_string_repr()).ok(); Value::Null }
        "getenv"  => { need!(args,1); Value::Str(std::env::var(args[0].to_string_repr()).unwrap_or_default()) }
        "setenv"  => { need!(args,2);unsafe{ std::env::set_var(args[0].to_string_repr(), args[1].to_string_repr());} Value::Null }
        "listdir" => {
            let path = if args.is_empty() { ".".to_string() } else { args[0].to_string_repr() };
            let entries: Vec<Value> = fs::read_dir(&path).unwrap_or_else(|e| {
                eprintln!("os.listdir: {}", e); process::exit(1);
            }).flatten()
            .map(|e| Value::Str(e.file_name().to_string_lossy().to_string()))
            .collect();
            Value::List(Rc::new(RefCell::new(entries)))
        }
        "exists"   => { need!(args,1); Value::Int(if std::path::Path::new(&args[0].to_string_repr()).exists(){1}else{0}) }
        "isFile"   => { need!(args,1); Value::Int(if std::path::Path::new(&args[0].to_string_repr()).is_file(){1}else{0}) }
        "isDir"    => { need!(args,1); Value::Int(if std::path::Path::new(&args[0].to_string_repr()).is_dir(){1}else{0}) }
        "mkdir"    => { need!(args,1); fs::create_dir_all(args[0].to_string_repr()).ok(); Value::Null }
        "remove"   => { need!(args,1); fs::remove_file(args[0].to_string_repr()).ok(); Value::Null }
        "rmdir"    => { need!(args,1); fs::remove_dir_all(args[0].to_string_repr()).ok(); Value::Null }
        "rename"   => { need!(args,2); fs::rename(args[0].to_string_repr(), args[1].to_string_repr()).ok(); Value::Null }
        "copy"     => { need!(args,2); fs::copy(args[0].to_string_repr(), args[1].to_string_repr()).ok(); Value::Null }
        "system"   => { need!(args,1); let r = Command::new("sh").arg("-c").arg(args[0].to_string_repr()).status().map(|s| s.code().unwrap_or(-1)).unwrap_or(-1); Value::Int(r as i64) }
        "popen"    => { need!(args,1); let out = Command::new("sh").arg("-c").arg(args[0].to_string_repr()).output().unwrap_or_else(|e|{eprintln!("os.popen: {}",e);process::exit(1);}); Value::Str(String::from_utf8_lossy(&out.stdout).trim_end().to_string()) }
        "exit"     => { let code=if !args.is_empty(){args[0].as_int() as i32}else{0}; process::exit(code); }
        "args"     => { let a: Vec<Value>=std::env::args().map(Value::Str).collect(); Value::List(Rc::new(RefCell::new(a))) }
        "pathJoin" => { if args.is_empty(){return Value::Str(String::new());} let mut p=std::path::PathBuf::from(args[0].to_string_repr()); for a in &args[1..]{p.push(a.to_string_repr());} Value::Str(p.display().to_string()) }
        "basename" => { need!(args,1); Value::Str(std::path::Path::new(&args[0].to_string_repr()).file_name().map(|n|n.to_string_lossy().to_string()).unwrap_or_default()) }
        "dirname"  => { need!(args,1); Value::Str(std::path::Path::new(&args[0].to_string_repr()).parent().map(|p|p.display().to_string()).unwrap_or_default()) }
        "absPath"  => { need!(args,1); Value::Str(fs::canonicalize(args[0].to_string_repr()).map(|p|p.display().to_string()).unwrap_or_else(|_|args[0].to_string_repr())) }
        _ => { eprintln!("os.{}() not found", name); process::exit(1); }
    }
}

// ============================================================
// SYS
// ============================================================
pub fn eval_sys_builtin(name: &str, args: Vec<Value>) -> Value {
    match name {
        "platform" => Value::Str(std::env::consts::OS.to_string()),
        "arch"     => Value::Str(std::env::consts::ARCH.to_string()),
        "argv"     => { let a: Vec<Value>=std::env::args().map(Value::Str).collect(); Value::List(Rc::new(RefCell::new(a))) }
        "exit"     => { let code=if !args.is_empty(){args[0].as_int() as i32}else{0}; process::exit(code); }
        "version"  => Value::Str("Payjar 0.1.0".to_string()),
        "stdin"    => { let mut l=String::new(); io::stdin().lock().read_line(&mut l).ok(); if l.ends_with('\n'){l.pop();} Value::Str(l) }
        "stdout"   => { need!(args,1); print!("{}",args[0].to_string_repr()); io::stdout().flush().ok(); Value::Null }
        "stderr"   => { need!(args,1); eprint!("{}",args[0].to_string_repr()); Value::Null }
        _ => { eprintln!("sys.{}() not found", name); process::exit(1); }
    }
}

// ============================================================
// TIME
// ============================================================
pub fn eval_time_builtin(name: &str, args: Vec<Value>) -> Value {
    use std::time::{SystemTime, UNIX_EPOCH, Duration};
    match name {
        "now"   => Value::Float(SystemTime::now().duration_since(UNIX_EPOCH).map(|d|d.as_secs_f64()).unwrap_or(0.0)),
        "nowMs" => Value::Int(SystemTime::now().duration_since(UNIX_EPOCH).map(|d|d.as_millis() as i64).unwrap_or(0)),
        "sleep" => { need!(args,1); let ms=args[0].as_float(); std::thread::sleep(Duration::from_millis((ms*1000.0) as u64)); Value::Null }
        "sleepMs"=>{ need!(args,1); std::thread::sleep(Duration::from_millis(args[0].as_int() as u64)); Value::Null }
        "format"=> {
            // time.format(timestamp_secs, "format_string") — basic formatting
            need!(args,1);
            let secs = args[0].as_int() as u64;
            // Manual UTC breakdown (no chrono dep)
            let s = secs % 60; let m = (secs/60)%60; let h=(secs/3600)%24;
            let days = secs/86400;
            let y400=days/146097; let r=days%146097;
            let y100=r/36524; let r=r%36524;
            let y4=r/1461; let r=r%1461;
            let y1=r/365; let r=r%365;
            let year=y400*400+y100*100+y4*4+y1+1970;
            let leap=(year%4==0&&year%100!=0)||(year%400==0);
            let months=[31u64,if leap{29}else{28},31,30,31,30,31,31,30,31,30,31];
            let mut month=1u64; let mut day=r+1;
            for mlen in &months{ if day>*mlen{day-=mlen;month+=1;}else{break;} }
            Value::Str(format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}",year,month,day,h,m,s))
        }
        "timestamp" => {
            // Parse "YYYY-MM-DD HH:MM:SS" → unix seconds (very basic)
            need!(args,1);
            let s = args[0].to_string_repr();
            // Just return 0 for now — full parsing needs chrono
            eprintln!("time.timestamp: use time.now() for current timestamp; parsing not yet supported");
            Value::Int(0)
        }
        _ => { eprintln!("time.{}() not found", name); process::exit(1); }
    }
}

// ============================================================
// BASE64
// ============================================================
pub fn eval_base64_builtin(name: &str, args: Vec<Value>) -> Value {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    match name {
        "encode" => {
            need!(args,1);
            let bytes = args[0].to_string_repr().into_bytes();
            Value::Str(STANDARD.encode(&bytes))
        }
        "decode" => {
            need!(args,1);
            let s = args[0].to_string_repr();
            match STANDARD.decode(s.trim()) {
                Ok(b)  => Value::Str(String::from_utf8_lossy(&b).to_string()),
                Err(e) => { eprintln!("base64.decode error: {}", e); process::exit(1); }
            }
        }
        "encodeBytes" => {
            need!(args,1);
            if let Value::List(l) = &args[0] {
                let bytes: Vec<u8> = l.borrow().iter().map(|v| v.as_int() as u8).collect();
                Value::Str(STANDARD.encode(&bytes))
            } else { eprintln!("base64.encodeBytes: need list"); process::exit(1); }
        }
        _ => { eprintln!("base64.{}() not found", name); process::exit(1); }
    }
}

// ============================================================
// HASH
// ============================================================
pub fn eval_hash_builtin(name: &str, args: Vec<Value>) -> Value {
    use sha2::{Sha256, Sha512, Digest};
    use md5::Md5;
    match name {
        "sha256" => {
            need!(args,1);
            let mut h = Sha256::new(); h.update(args[0].to_string_repr().as_bytes());
            Value::Str(format!("{:x}", h.finalize()))
        }
        "sha512" => {
            need!(args,1);
            let mut h = Sha512::new(); h.update(args[0].to_string_repr().as_bytes());
            Value::Str(format!("{:x}", h.finalize()))
        }
        "md5" => {
            need!(args,1);
            let mut h = Md5::new(); h.update(args[0].to_string_repr().as_bytes());
            Value::Str(format!("{:x}", h.finalize()))
        }
        "sha256File" => {
            need!(args,1);
            let path = args[0].to_string_repr();
            let data = fs::read(&path).unwrap_or_else(|e| { eprintln!("hash.sha256File: {}", e); process::exit(1); });
            let mut h = Sha256::new(); h.update(&data);
            Value::Str(format!("{:x}", h.finalize()))
        }
        _ => { eprintln!("hash.{}() not found", name); process::exit(1); }
    }
}

// ============================================================
// REGEX  (real `regex` crate)
// ============================================================
pub fn eval_regex_builtin(name: &str, args: Vec<Value>) -> Value {
    use regex::Regex;
    match name {
        "test" => {
            need!(args,2);
            let pat = args[0].to_string_repr(); let txt = args[1].to_string_repr();
            let re = Regex::new(&pat).unwrap_or_else(|e|{eprintln!("regex.test: invalid pattern '{}': {}",pat,e);process::exit(1);});
            Value::Int(if re.is_match(&txt){1}else{0})
        }
        "match" => {
            need!(args,2);
            let pat = args[0].to_string_repr(); let txt = args[1].to_string_repr();
            let re = Regex::new(&pat).unwrap_or_else(|e|{eprintln!("regex.match: {}",e);process::exit(1);});
            match re.find(&txt) { Some(m) => Value::Str(m.as_str().to_string()), None => Value::Null }
        }
        "matchGroup" => {
            // regex.matchGroup(pattern, text) → list of capture groups
            need!(args,2);
            let pat = args[0].to_string_repr(); let txt = args[1].to_string_repr();
            let re = Regex::new(&pat).unwrap_or_else(|e|{eprintln!("regex.matchGroup: {}",e);process::exit(1);});
            match re.captures(&txt) {
                None => Value::Null,
                Some(caps) => {
                    let groups: Vec<Value> = caps.iter().map(|m| match m {
                        Some(s) => Value::Str(s.as_str().to_string()),
                        None    => Value::Null,
                    }).collect();
                    Value::List(Rc::new(RefCell::new(groups)))
                }
            }
        }
        "findAll" => {
            need!(args,2);
            let pat = args[0].to_string_repr(); let txt = args[1].to_string_repr();
            let re = Regex::new(&pat).unwrap_or_else(|e|{eprintln!("regex.findAll: {}",e);process::exit(1);});
            let items: Vec<Value> = re.find_iter(&txt).map(|m| Value::Str(m.as_str().to_string())).collect();
            Value::List(Rc::new(RefCell::new(items)))
        }
        "findAllGroups" => {
            need!(args,2);
            let pat = args[0].to_string_repr(); let txt = args[1].to_string_repr();
            let re = Regex::new(&pat).unwrap_or_else(|e|{eprintln!("regex.findAllGroups: {}",e);process::exit(1);});
            let items: Vec<Value> = re.captures_iter(&txt).map(|caps| {
                let groups: Vec<Value> = caps.iter().map(|m| match m {
                    Some(s) => Value::Str(s.as_str().to_string()), None => Value::Null,
                }).collect();
                Value::List(Rc::new(RefCell::new(groups)))
            }).collect();
            Value::List(Rc::new(RefCell::new(items)))
        }
        "replace" => {
            need!(args,3);
            let pat = args[0].to_string_repr(); let rep = args[1].to_string_repr(); let txt = args[2].to_string_repr();
            let re = Regex::new(&pat).unwrap_or_else(|e|{eprintln!("regex.replace: {}",e);process::exit(1);});
            Value::Str(re.replace(&txt, rep.as_str()).to_string())
        }
        "replaceAll" => {
            need!(args,3);
            let pat = args[0].to_string_repr(); let rep = args[1].to_string_repr(); let txt = args[2].to_string_repr();
            let re = Regex::new(&pat).unwrap_or_else(|e|{eprintln!("regex.replaceAll: {}",e);process::exit(1);});
            Value::Str(re.replace_all(&txt, rep.as_str()).to_string())
        }
        "split" => {
            need!(args,2);
            let pat = args[0].to_string_repr(); let txt = args[1].to_string_repr();
            let re = Regex::new(&pat).unwrap_or_else(|e|{eprintln!("regex.split: {}",e);process::exit(1);});
            let parts: Vec<Value> = re.split(&txt).map(|s| Value::Str(s.to_string())).collect();
            Value::List(Rc::new(RefCell::new(parts)))
        }
        "valid" => {
            need!(args,1);
            Value::Int(if Regex::new(&args[0].to_string_repr()).is_ok(){1}else{0})
        }
        _ => { eprintln!("regex.{}() not found", name); process::exit(1); }
    }
}

// ============================================================
// RANDOM
// ============================================================
pub fn eval_random_builtin(name: &str, args: Vec<Value>) -> Value {
    use std::time::{SystemTime, UNIX_EPOCH};
    fn rng() -> u64 {
        let n = SystemTime::now().duration_since(UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as u64 ^ d.as_secs().wrapping_mul(2654435761))
            .unwrap_or(99991);
        n.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407)
    }
    match name {
        "randint"   => { need!(args,2); let lo=args[0].as_int();let hi=args[1].as_int(); if lo>hi{eprintln!("random.randint: lo>hi");process::exit(1);} Value::Int(lo+(rng()%(hi-lo+1) as u64) as i64) }
        "random"    => Value::Float(rng() as f64 / u64::MAX as f64),
        "randFloat" => { need!(args,2); let lo=args[0].as_float();let hi=args[1].as_float(); Value::Float(lo+(rng() as f64/u64::MAX as f64)*(hi-lo)) }
        "choice"    => { need!(args,1); if let Value::List(ref l)=args[0]{ let b=l.borrow(); if b.is_empty(){eprintln!("random.choice: empty list");process::exit(1);} b[(rng() as usize)%b.len()].clone() }else{eprintln!("random.choice: need list");process::exit(1);} }
        "shuffle"   => { need!(args,1); if let Value::List(ref l)=args[0]{ let mut b=l.borrow_mut(); for i in (1..b.len()).rev(){let j=(rng() as usize)%(i+1);b.swap(i,j);} Value::Null }else{eprintln!("random.shuffle: need list");process::exit(1);} }
        "seed"      => Value::Null,
        _ => { eprintln!("random.{}() not found", name); process::exit(1); }
    }
}

// ============================================================
// GUI  — real egui windows (each call blocks until closed)
// ============================================================
pub fn eval_gui_builtin(name: &str, args: Vec<Value>) -> Value {
    use std::sync::{Arc, Mutex, mpsc};

    struct AlertApp  { title: String, msg: String, done: bool }
    struct ConfirmApp{ msg: String, result: Option<bool> }
    struct PromptApp { msg: String, input: String, submitted: bool }
    struct NotifyApp { title: String, msg: String, start: std::time::Instant }

    impl eframe::App for AlertApp {
        fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.add_space(14.0); ui.vertical_centered(|ui| {
                    ui.heading(&self.title); ui.add_space(8.0); ui.label(&self.msg); ui.add_space(14.0);
                    if ui.button("  OK  ").clicked() { self.done = true; }
                });
            });
            if self.done { ctx.send_viewport_cmd(egui::ViewportCommand::Close); }
        }
    }
    impl eframe::App for ConfirmApp {
        fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.add_space(14.0); ui.vertical_centered(|ui| {
                    ui.label(&self.msg); ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if ui.button("  Cancel  ").clicked() { self.result = Some(false); }
                        if ui.button("    OK    ").clicked() { self.result = Some(true);  }
                    });
                });
            });
            if self.result.is_some() { ctx.send_viewport_cmd(egui::ViewportCommand::Close); }
        }
    }
    impl eframe::App for PromptApp {
        fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.add_space(12.0); ui.vertical_centered(|ui| {
                    ui.label(&self.msg); ui.add_space(8.0);
                    ui.text_edit_singleline(&mut self.input).request_focus();
                    ui.add_space(10.0);
                    if ui.button("  OK  ").clicked() || ctx.input(|i| i.key_pressed(egui::Key::Enter))
                    { self.submitted = true; }
                });
            });
            if self.submitted { ctx.send_viewport_cmd(egui::ViewportCommand::Close); }
        }
    }
    impl eframe::App for NotifyApp {
        fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(10.0); ui.heading(&self.title); ui.add_space(6.0); ui.label(&self.msg);
                    let s = (2.0 - self.start.elapsed().as_secs_f32()).max(0.0);
                    ui.add_space(8.0); ui.label(egui::RichText::new(format!("Closing in {:.0}s…", s)).weak());
                });
            });
            if self.start.elapsed().as_secs_f32() >= 2.0 { ctx.send_viewport_cmd(egui::ViewportCommand::Close); }
        }
    }

    fn spawn_wait(
    title: &str,
    size: [f32; 2],
    app: impl eframe::App + Send + 'static,
) {
    let t = title.to_string();
    let (tx, rx) = mpsc::channel::<()>();

    std::thread::spawn(move || {
        let opts = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_title(&t)
                .with_inner_size(size)
                .with_resizable(false),
            ..Default::default()
        };

        let _ = eframe::run_native(
            &t,
            opts,
            Box::new(|_| Box::new(app) as Box<dyn eframe::App>),
        );

        let _ = tx.send(());
    });

    let _ = rx.recv();
}

    match name {
        "alert" | "msgbox" => {
            let (title, msg) = match args.len() {
                1 => ("Payjar".to_string(), args[0].to_string_repr()),
                2 => (args[0].to_string_repr(), args[1].to_string_repr()),
                _ => { eprintln!("gui.{}() takes 1 or 2 args", name); process::exit(1); }
            };
            let t2=title.clone(); let m2=msg.clone();
            spawn_wait(&title, [360.0,180.0], AlertApp { title: t2, msg: m2, done: false });
            Value::Null
        }
        "confirm" => {
            need!(args,1);
            let msg = args[0].to_string_repr();
            let result: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
            let r2 = Arc::clone(&result);
            struct CW { inner: ConfirmApp, store: Arc<Mutex<bool>> }
            impl eframe::App for CW {
                fn update(&mut self, ctx: &egui::Context, f: &mut eframe::Frame) {
                    self.inner.update(ctx, f);
                    if let Some(r) = self.inner.result { *self.store.lock().unwrap() = r; }
                }
            }
            spawn_wait("Confirm", [360.0,160.0], CW { inner: ConfirmApp { msg, result: None }, store: r2 });
            Value::Int(if *result.lock().unwrap(){1}else{0})
        }
        "prompt" => {
            need!(args,1);
            let msg = args[0].to_string_repr();
            let result: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
            let r2 = Arc::clone(&result);
            struct PW { inner: PromptApp, store: Arc<Mutex<String>> }
            impl eframe::App for PW {
                fn update(&mut self, ctx: &egui::Context, f: &mut eframe::Frame) {
                    self.inner.update(ctx, f);
                    if self.inner.submitted { *self.store.lock().unwrap() = self.inner.input.clone(); }
                }
            }
            spawn_wait("Input", [380.0,160.0], PW { inner: PromptApp { msg, input: String::new(), submitted: false }, store: r2 });
            Value::Str(result.lock().unwrap().clone())
        }
        "notify" => {
            let (title, msg) = match args.len() {
                1 => ("Payjar".to_string(), args[0].to_string_repr()),
                2 => (args[0].to_string_repr(), args[1].to_string_repr()),
                _ => { eprintln!("gui.notify() takes 1 or 2 args"); process::exit(1); }
            };
            let t2=title.clone(); let m2=msg.clone();
            spawn_wait(&title, [360.0,140.0], NotifyApp { title: t2, msg: m2, start: std::time::Instant::now() });
            Value::Null
        }
        _ => { eprintln!("gui.{}() not found", name); process::exit(1); }
    }
}

// ============================================================
// GENERAL BUILTINS (toStr, len, readln, etc.)
// ============================================================
pub fn eval_builtin_or_func(
    env: &mut Env, name: &str, args: Vec<Value>,
    self_obj: Option<Rc<RefCell<PJObject>>>,
) -> Value {
    match name {
        "toStr"   => { need!(args,1); Value::Str(args[0].to_string_repr()) }
        "toInt"   => { need!(args,1); match &args[0]{ Value::Str(s)=>Value::Int(s.trim().parse().unwrap_or(0)), v=>Value::Int(v.as_int()) } }
        "toFloat" => { need!(args,1); Value::Float(args[0].as_float()) }
        "pow"     => { need!(args,2); if let(Value::Int(b),Value::Int(e))=(&args[0],&args[1]){if *e>=0{return Value::Int(b.pow(*e as u32));}} Value::Float(args[0].as_float().powf(args[1].as_float())) }
        "print"   => { if args.len()>1{eprintln!("print() 0-1 arg");process::exit(1);} if let Some(v)=args.get(0){v.print();} Value::Null }
        "println" => { if args.len()>1{eprintln!("println() 0-1 arg");process::exit(1);} if let Some(v)=args.get(0){v.print();} println!(); Value::Null }
        "len"     => { need!(args,1); match &args[0]{ Value::Str(s)=>Value::Int(s.chars().count() as i64), Value::List(l)=>Value::Int(l.borrow().len() as i64), _=>Value::Int(0) } }
        "strLen"  => { need!(args,1); if let Value::Str(s)=&args[0]{Value::Int(s.chars().count() as i64)}else{eprintln!("strLen: string");process::exit(1);} }
        "charAt"  => { need!(args,2); if let Value::Str(s)=&args[0]{ let ch:Vec<char>=s.chars().collect(); let mut i=args[1].as_int(); if i<0{i=ch.len() as i64+i;} if i<0||i>=ch.len() as i64{eprintln!("charAt: out of range");process::exit(1);} Value::Str(ch[i as usize].to_string()) }else{eprintln!("charAt: string");process::exit(1);} }
        "strSlice"      => { if args.len()<2{eprintln!("strSlice: 2-3 args");process::exit(1);} if let Value::Str(s)=&args[0]{ let ch:Vec<char>=s.chars().collect();let n=ch.len() as i64; let mut a=args[1].as_int();let mut b=if args.len()>=3{args[2].as_int()}else{n}; if a<0{a=n+a;} if b<0{b=n+b;} a=a.clamp(0,n);b=b.clamp(0,n);if a>b{a=b;} Value::Str(ch[a as usize..b as usize].iter().collect()) }else{eprintln!("strSlice: string");process::exit(1);} }
        "strContains"   => { need!(args,2); if let(Value::Str(s),Value::Str(p))=(&args[0],&args[1]){Value::Int(if s.contains(p.as_str()){1}else{0})}else{eprintln!("strContains: strings");process::exit(1);} }
        "strReplace"    => { need!(args,3); if let(Value::Str(s),Value::Str(f),Value::Str(t))=(&args[0],&args[1],&args[2]){Value::Str(s.replace(f.as_str(),t.as_str()))}else{eprintln!("strReplace: strings");process::exit(1);} }
        "strSplit"      => { need!(args,2); if let(Value::Str(s),Value::Str(d))=(&args[0],&args[1]){ let items:Vec<Value>=if d.is_empty(){s.chars().map(|c|Value::Str(c.to_string())).collect()}else{s.split(d.as_str()).map(|p|Value::Str(p.to_string())).collect()}; Value::List(Rc::new(RefCell::new(items))) }else{eprintln!("strSplit: strings");process::exit(1);} }
        "strTrim"       => { need!(args,1); if let Value::Str(s)=&args[0]{Value::Str(s.trim().to_string())}else{eprintln!("strTrim: string");process::exit(1);} }
        "strUpper"      => { need!(args,1); if let Value::Str(s)=&args[0]{Value::Str(s.to_uppercase())}else{eprintln!("strUpper: string");process::exit(1);} }
        "strLower"      => { need!(args,1); if let Value::Str(s)=&args[0]{Value::Str(s.to_lowercase())}else{eprintln!("strLower: string");process::exit(1);} }
        "strStartsWith" => { need!(args,2); if let(Value::Str(s),Value::Str(p))=(&args[0],&args[1]){Value::Int(if s.starts_with(p.as_str()){1}else{0})}else{eprintln!("strStartsWith: strings");process::exit(1);} }
        "strEndsWith"   => { need!(args,2); if let(Value::Str(s),Value::Str(p))=(&args[0],&args[1]){Value::Int(if s.ends_with(p.as_str()){1}else{0})}else{eprintln!("strEndsWith: strings");process::exit(1);} }
        "readFile"      => { need!(args,1); if let Value::Str(p)=&args[0]{Value::Str(fs::read_to_string(p).unwrap_or_default())}else{eprintln!("readFile: string");process::exit(1);} }
        "writeFile"     => { need!(args,2); if let Value::Str(p)=&args[0]{ fs::write(p,args[1].to_string_repr()).unwrap_or_else(|_|{eprintln!("writeFile: failed");process::exit(1);}); Value::Null }else{eprintln!("writeFile: string");process::exit(1);} }
        "appendFile"    => { need!(args,2); if let Value::Str(p)=&args[0]{ use std::io::Write as _; let mut f=std::fs::OpenOptions::new().append(true).create(true).open(p).unwrap_or_else(|_|{eprintln!("appendFile: failed");process::exit(1);}); f.write_all(args[1].to_string_repr().as_bytes()).ok(); Value::Null }else{eprintln!("appendFile: string");process::exit(1);} }
        "exit"          => { process::exit(if !args.is_empty(){args[0].as_int() as i32}else{0}); }
        "listLen"       => { need!(args,1); match &args[0]{ Value::List(l)=>Value::Int(l.borrow().len() as i64), _=>Value::Int(0) } }
        "typeOf"        => { need!(args,1); Value::Str(match &args[0]{ Value::Int(_)=>"int", Value::Float(_)=>"float", Value::Str(_)=>"str", Value::Null=>"null", Value::Object(_)=>"object", Value::List(_)=>"list", Value::Comp(_)=>"complex" }.to_string()) }
        "readi"         => { need!(args,1); print!("{}",args[0].to_string_repr()); io::stdout().flush().ok(); let mut l=String::new(); io::stdin().lock().read_line(&mut l).ok(); Value::Int(l.trim().parse().unwrap_or(0)) }
        "readf"         => { need!(args,1); print!("{}",args[0].to_string_repr()); io::stdout().flush().ok(); let mut l=String::new(); io::stdin().lock().read_line(&mut l).ok(); Value::Float(l.trim().parse().unwrap_or(0.0)) }
        "readln"        => { let p=if args.len()==1{args[0].to_string_repr()}else{String::new()}; print!("{}",p); io::stdout().flush().ok(); let mut l=String::new(); io::stdin().lock().read_line(&mut l).ok(); if l.ends_with('\n'){l.pop();} if l.ends_with('\r'){l.pop();} Value::Str(l) }
        "range"         => { let(start,end,step)=match args.len(){1=>(0,args[0].as_int(),1),2=>(args[0].as_int(),args[1].as_int(),1),3=>(args[0].as_int(),args[1].as_int(),args[2].as_int()),_=>{eprintln!("range() 1-3 args");process::exit(1);}}; if step==0{eprintln!("range: step!=0");process::exit(1);} let mut items=Vec::new();let mut i=start; while if step>0{i<end}else{i>end}{items.push(Value::Int(i));i+=step;} Value::List(Rc::new(RefCell::new(items))) }
        _ => {
            if matches!(name,"abs"|"sqrt"|"cbrt"|"floor"|"ceil"|"round"|"sum"|"min"|"max"|"ln"|"log"|"log2"|"log10"|"sin"|"cos"|"tan"|"asin"|"acos"|"atan"|"atan2"|"sinh"|"cosh"|"tanh"|"exp"|"hypot"|"sign"|"clamp"|"lerp"|"isNan"|"isInf"|"toRad"|"toDeg"|"trunc"|"fract"|"gcd"|"lcm"){ return eval_math_builtin(name, args); }
            if matches!(name,"randint"|"randFloat"|"choice"|"shuffle"|"seed"){ return eval_random_builtin(name, args); }
            let func = env.funcs.get(name).cloned();
            if let Some(f) = func { return call_func_with_vals(env, &f, args, None); }
            if let Some(ref so) = self_obj {
                let method = so.borrow().find_method(name).cloned();
                if let Some(m) = method { return call_func_with_vals(env, &m, args, Some(Rc::clone(so))); }
            }
            eprintln!("Runtime Error: Undefined function '{}'", name);
            process::exit(1);
        }
    }
}

// ============================================================
// MODULE METHOD DISPATCH
// ============================================================
pub fn eval_builtin_module_method(
    module: &str, name: &str, arg_nodes: &[AstNode],
    env: &mut Env, self_obj: Option<Rc<RefCell<PJObject>>>,
) -> Option<Value> {
    let args: Vec<Value> = arg_nodes.iter().map(|a| super::eval(env, a, self_obj.clone())).collect();
    match module {
        "math" => match name {
            "abs"|"sqrt"|"cbrt"|"floor"|"ceil"|"round"|"sum"|"min"|"max"|"ln"|"log"|"log2"|"log10"|
            "sin"|"cos"|"tan"|"asin"|"acos"|"atan"|"atan2"|"sinh"|"cosh"|"tanh"|
            "pow"|"exp"|"hypot"|"sign"|"clamp"|"lerp"|"pi"|"e"|"tau"|"inf"|"nan"|
            "isNan"|"isInf"|"toRad"|"toDeg"|"trunc"|"fract"|"gcd"|"lcm"
                => Some(eval_math_builtin(name, args)),
            _ => None,
        },
        "cmath"           => Some(eval_cmath_builtin(name, args)),
        "json"            => Some(eval_json_builtin(name, args)),
        "os"              => Some(eval_os_builtin(name, args)),
        "sys"             => Some(eval_sys_builtin(name, args)),
        "time"            => Some(eval_time_builtin(name, args)),
        "base64"          => Some(eval_base64_builtin(name, args)),
        "hash"            => Some(eval_hash_builtin(name, args)),
        "io" => match name {
            "print"|"println"|"readln"|"readi"|"readf"
                => Some(eval_builtin_or_func(env, name, args, self_obj)),
            _ => None,
        },
        "random"             => Some(eval_random_builtin(name, args)),
        "regex"              => Some(eval_regex_builtin(name, args)),
        "gui"                => Some(eval_gui_builtin(name, args)),
        "sqlite3"|"sqllite3" => Some(eval_sqlite3_builtin(name, args)),
        _ => None,
    }
}
