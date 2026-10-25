use std::process;
use std::io::{self, BufRead, Write,BufReader,Read};
use std::fs;
use std::rc::Rc;
use std::cell::RefCell;
use std::net::TcpStream;
use std::time::Duration;
use std::process::Command;
use super::{Value, PJObject, Env, AstNode, call_func_with_vals, Complex};
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
        "abs"   => { need!(args,1); match &args[0]{ Value::Int(i)=>Value::Int(i.abs()), Value::Comp(c)=>Value::Float((c.real*c.real+c.imag*c.imag).sqrt()), _=>Value::Float(args[0].as_float().abs()) } }
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
    fn mk(r: f64, i: f64) -> Value { Value::Comp(Complex::new(r, i)) }
    fn unpack(v: &Value) -> (f64, f64) { let c = v.as_complex(); (c.real, c.imag) }
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
        "sqrt"    => { need!(args,1); let (r,i)=unpack(&args[0]); let s = Complex::new(r,i).sqrt(); mk(s.real,s.imag) }
        "exp"     => { need!(args,1); let (r,i)=unpack(&args[0]); let er=r.exp(); mk(er*i.cos(),er*i.sin()) }
        "ln"      => { need!(args,1); let (r,i)=unpack(&args[0]); let m=(r*r+i*i).sqrt(); if m==0.0{eprintln!("cmath.ln: zero");process::exit(1);} mk(m.ln(),i.atan2(r)) }
        "pow"     => { need!(args,2); let (ar,ai)=unpack(&args[0]);let (br,bi)=unpack(&args[1]); let r = Complex::new(ar,ai).powc(Complex::new(br,bi)); mk(r.real,r.imag) }
        "sin"     => { need!(args,1); let (r,i)=unpack(&args[0]); mk(r.sin()*i.cosh(), r.cos()*i.sinh()) }
        "cos"     => { need!(args,1); let (r,i)=unpack(&args[0]); mk(r.cos()*i.cosh(),-r.sin()*i.sinh()) }
        "polar"   => { need!(args,2); let r=args[0].as_float();let theta=args[1].as_float(); mk(r*theta.cos(),r*theta.sin()) }
        "toString"=> { need!(args,1); let (r,i)=unpack(&args[0]); Value::Str(Complex::new(r,i).to_display()) }
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
            Value::Map(m) => {
                let mut map = serde_json::Map::new();
                for (k, v) in m.borrow().iter() {
                    map.insert(k.clone(), val_to_json(v));
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
                use rustc_hash::FxHashMap;
                let mut map: FxHashMap<String, Value> = FxHashMap::default();
                for (k, v) in o.iter() { map.insert(k.clone(), json_to_val(v)); }
                Value::Map(std::rc::Rc::new(std::cell::RefCell::new(map)))
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
    match name {
        "getcwd"  => Value::Str(std::env::current_dir().map(|p| p.display().to_string()).unwrap_or_default()),
        "chdir"   => { need!(args,1); std::env::set_current_dir(args[0].to_string_repr()).ok(); Value::Null }
        "getenv"  => { need!(args,1); Value::Str(std::env::var(args[0].to_string_repr()).unwrap_or_default()) }
        "setenv"  => { need!(args,2); unsafe { std::env::set_var(args[0].to_string_repr(), args[1].to_string_repr()); } Value::Null }
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
        "version"  => Value::Str("Payjar 1.4.0-beta".to_string()),
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
    use std::time::{SystemTime, UNIX_EPOCH};
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
            let _s = args[0].to_string_repr();
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
    fn rng() -> u64 {
        // OS-provided CSPRNG (was: clock-seeded LCG, predictable)
        let mut b = [0u8; 8];
        getrandom::getrandom(&mut b).expect("OS RNG unavailable");
        u64::from_le_bytes(b)
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
        use super::gui_channel::{GuiRequest, gui_send};
        let (done_tx, done_rx) = mpsc::channel::<()>();
        gui_send(GuiRequest {
            title: title.to_string(),
            size,
            app: Box::new(app),
            done_tx,
        });
        // Block the interpreter thread until main thread closes the window.
        let _ = done_rx.recv();
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
            let final_input = result.lock().unwrap().clone();
            Value::Str(final_input)
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
        // Python-inspired dir(): list visible names, or fields/methods of an object.
        "dir" => {
            if args.len() > 1 {
                eprintln!("dir() takes 0 or 1 argument");
                process::exit(1);
            }
            let mut names: Vec<String> = if args.is_empty() {
                let mut visible = std::collections::BTreeSet::new();
                visible.extend(super::Scope::visible_names(&env.scope));
                visible.extend(env.funcs.keys().cloned());
                visible.extend(env.classes.keys().cloned());
                visible.extend(env.enums.keys().cloned());
                // Builtins are dispatched directly rather than stored in Env::funcs.
                visible.extend([
                    "abs", "appendFile", "ceil", "charAt", "clamp", "cos", "dir",
                    "exit", "floor", "indexOf", "len", "listLen", "max", "min",
                    "pow", "print", "printerr", "println", "range", "readFile",
                    "readf", "readi", "readln", "round", "sin", "sqrt", "strContains",
                    "strEndsWith", "strLen", "strSlice", "strSplit", "strStartsWith",
                    "strTrim", "toFloat", "toInt", "toStr", "typeOf", "writeFile",
                ].into_iter().map(str::to_string));
                visible.into_iter().collect()
            } else {
                match &args[0] {
                    Value::Object(object) => {
                        let object = object.borrow();
                        let mut names = std::collections::BTreeSet::new();
                        names.extend(object.fields.iter().map(|field| field.name.clone()));
                        names.extend(object.class_def.methods.iter().filter_map(|method| method.name.clone()));
                        names.into_iter().collect()
                    }
                    Value::Map(map) => {
                        let map = map.borrow();
                        map.keys().cloned().collect::<std::collections::BTreeSet<_>>().into_iter().collect()
                    }
                    // Primitive and list values have no per-instance named attributes in PayJar.
                    _ => Vec::new(),
                }
            };
            names.sort();
            Value::List(Rc::new(RefCell::new(names.into_iter().map(Value::Str).collect())))
        }
        "toStr"   => { need!(args,1); Value::Str(args[0].to_string_repr()) }
        "toInt"   => { need!(args,1); match &args[0]{ Value::Str(s)=>Value::Int(s.trim().parse().unwrap_or(0)), v=>Value::Int(v.as_int()) } }
        "toFloat" => { need!(args,1); Value::Float(args[0].as_float()) }
        "toComplex" => { need!(args,2); Value::Comp(Complex::new(args[0].as_float(), args[1].as_float())) }
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
        "typeOf"        => { need!(args,1); Value::Str(match &args[0]{ Value::Int(_)=>"int", Value::Float(_)=>"float", Value::Str(_)=>"str", Value::Null=>"null", Value::Object(_)=>"object", Value::List(_)=>"list", Value::Comp(_)=>"complex", Value::Map(_)=>"map", Value::Task(_)=>"task", Value::Enum(_,_)=>"enum" }.to_string()) }
        "readi"         => { need!(args,1); print!("{}",args[0].to_string_repr()); io::stdout().flush().ok(); let mut l=String::new(); io::stdin().lock().read_line(&mut l).ok(); Value::Int(l.trim().parse().unwrap_or(0)) }
        "readf"         => { need!(args,1); print!("{}",args[0].to_string_repr()); io::stdout().flush().ok(); let mut l=String::new(); io::stdin().lock().read_line(&mut l).ok(); Value::Float(l.trim().parse().unwrap_or(0.0)) }
        "readln"        => { let p=if args.len()==1{args[0].to_string_repr()}else{String::new()}; print!("{}",p); io::stdout().flush().ok(); let mut l=String::new(); io::stdin().lock().read_line(&mut l).ok(); if l.ends_with('\n'){l.pop();} if l.ends_with('\r'){l.pop();} Value::Str(l) }
        "range"         => { let(start,end,step)=match args.len(){1=>(0,args[0].as_int(),1),2=>(args[0].as_int(),args[1].as_int(),1),3=>(args[0].as_int(),args[1].as_int(),args[2].as_int()),_=>{eprintln!("range() 1-3 args");process::exit(1);}}; if step==0{eprintln!("range: step!=0");process::exit(1);} let mut items=Vec::new();let mut i=start; while if step>0{i<end}else{i>end}{items.push(Value::Int(i));i+=step;} Value::List(Rc::new(RefCell::new(items))) }
        _ => {
            if matches!(name,"abs"|"sqrt"|"cbrt"|"floor"|"ceil"|"round"|"sum"|"min"|"max"|"ln"|"log"|"log2"|"log10"|"sin"|"cos"|"tan"|"asin"|"acos"|"atan"|"atan2"|"sinh"|"cosh"|"tanh"|"exp"|"hypot"|"sign"|"clamp"|"lerp"|"isNan"|"isInf"|"toRad"|"toDeg"|"trunc"|"fract"|"gcd"|"lcm"){ return eval_math_builtin(name, args); }
            if matches!(name,"randint"|"randFloat"|"choice"|"shuffle"|"seed"){ return eval_random_builtin(name, args); }
            let func = env.funcs.get(name).cloned();
            if let Some(f) = func {
                if f.is_async() { return super::make_pending_task(f, args, None); }
                return call_func_with_vals(env, &f, args, None);
            }
            if let Some(ref so) = self_obj {
                let method = so.borrow().find_method(name).cloned();
                if let Some(m) = method {
                    if m.is_async() { return super::make_pending_task(m, args, Some(Rc::clone(so))); }
                    return call_func_with_vals(env, &m, args, Some(Rc::clone(so)));
                }
            }
            eprintln!("Runtime Error: Undefined function '{}'", name);
            process::exit(1);
        }
    }
}


// ============================================================
// NET — HTTP + raw TCP networking
// ============================================================
pub fn eval_net_builtin(name: &str, args: Vec<Value>) -> Value {
    match name {
        // ── HTTP (blocking, stdlib only — no extra dep) ─────────────────────
        // net.get(url) -> Str (response body) or Null on error
        "get" => {
            if args.is_empty() { eprintln!("net.get() takes 1 arg"); std::process::exit(1); }
            let url = args[0].to_string_repr();
            match get_req(&url) {
                Ok(body) => Value::Str(body),
                Err(e)   => { eprintln!("net.get error: {}", e); Value::Null }
            }
        }
        // net.getStatus(url) -> Int (HTTP status code) or -1 on error
        "getStatus" => {
            if args.is_empty() { eprintln!("net.getStatus() takes 1 arg"); std::process::exit(1); }
            let url = args[0].to_string_repr();
            match get_status(&url) {
                Ok(code) => Value::Int(code),
                Err(_)   => Value::Int(-1),
            }
        }
        // net.post(url, body, content_type?) -> Str
        "post" => {
            if args.len() < 2 { eprintln!("net.post() takes 2-4 args"); std::process::exit(1); }
            let url  = args[0].to_string_repr();
            let body = args[1].to_string_repr();
            let ct   = if args.len() >= 3 { args[2].to_string_repr() }
                       else { "application/x-www-form-urlencoded".to_string() };
            match post_req(&url, &body, &ct) {
                Ok(resp) => Value::Str(resp),
                Err(e)   => { eprintln!("net.post error: {}", e); Value::Null }
            }
        }
        // ── Raw TCP ─────────────────────────────────────────────────────────
        // net.tcpSend(host, port, data) -> Str (response, up to 64 KB)
        "tcpSend" => {
            if args.len() < 3 { eprintln!("net.tcpSend() takes 3 args"); std::process::exit(1); }
            let host = args[0].to_string_repr();
            let port = args[1].as_int() as u16;
            let data = args[2].to_string_repr();
            let addr = format!("{}:{}", host, port);
            match TcpStream::connect(&addr) {
                Err(e) => { eprintln!("net.tcpSend connect error: {}", e); Value::Null }
                Ok(mut stream) => {
                    stream.set_read_timeout(Some(Duration::from_secs(10))).ok();
                    stream.write_all(data.as_bytes()).ok();
                    let mut buf = Vec::new();
                    stream.read_to_end(&mut buf).ok();
                    Value::Str(String::from_utf8_lossy(&buf).into_owned())
                }
            }
        }
        // net.urlEncode(s) -> Str
        "urlEncode" => {
            if args.is_empty() { eprintln!("net.urlEncode() takes 1 arg"); std::process::exit(1); }
            let s = args[0].to_string_repr();
            let encoded: String = s.bytes().flat_map(|b| {
                if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' {
                    vec![b as char]
                } else {
                    format!("%{:02X}", b).chars().collect()
                }
            }).collect();
            Value::Str(encoded)
        }
        // net.urlDecode(s) -> Str
        "urlDecode" => {
            if args.is_empty() { eprintln!("net.urlDecode() takes 1 arg"); std::process::exit(1); }
            let s = args[0].to_string_repr();
            let mut out = Vec::new();
            let bytes = s.as_bytes();
            let mut i = 0;
            while i < bytes.len() {
                if bytes[i] == b'%' && i + 2 < bytes.len() {
                    if let Ok(h) = u8::from_str_radix(unsafe { std::str::from_utf8_unchecked(&bytes[i+1..i+3]) }, 16) {
                        out.push(h); i += 3; continue;
                    }
                } else if bytes[i] == b'+' {
                    out.push(b' '); i += 1; continue;
                }
                out.push(bytes[i]); i += 1;
            }
            Value::Str(String::from_utf8_lossy(&out).into_owned())
        }
        // net.parseUrl(url) -> List of [scheme, host, port, path, query]
        "parseUrl" => {
            if args.is_empty() { eprintln!("net.parseUrl() takes 1 arg"); std::process::exit(1); }
            let url = args[0].to_string_repr();
            let parts = parse_url_parts(&url);
            Value::List(std::rc::Rc::new(std::cell::RefCell::new(
                parts.into_iter().map(Value::Str).collect()
            )))
        }
        _ => { eprintln!("net.{}() not found", name); std::process::exit(1); }
    }
}
// ── Helpers (HTTP/1.1 via TcpStream + TLS via system process fallback) ──────

fn parse_url_parts(url: &str) -> Vec<String> {
    let (scheme, rest) = if let Some(s) = url.strip_prefix("https://") { ("https".to_string(), s) }
                          else if let Some(s) = url.strip_prefix("http://") { ("http".to_string(), s) }
                          else { ("".to_string(), url) };
    let (hostport, path_query) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None    => (rest, "/"),
    };
    let (host, port) = match hostport.rsplit_once(':') {
        Some((h, p)) => (h.to_string(), p.to_string()),
        None => (hostport.to_string(), if scheme == "https" { "443".to_string() } else { "80".to_string() }),
    };
    let (path, query) = match path_query.find('?') {
        Some(i) => (&path_query[..i], &path_query[i+1..]),
        None    => (path_query, ""),
    };
    vec![scheme, host, port, path.to_string(), query.to_string()]
}

fn get_req(url: &str) -> Result<String, String> {
    let (body, _) = raw_http(url, "GET", None, None)?;
    Ok(body)
}

fn get_status(url: &str) -> Result<i64, String> {
    let (_, status) = raw_http(url, "GET", None, None)?;
    Ok(status)
}

fn post_req(url: &str, body: &str, content_type: &str) -> Result<String, String> {
    let (resp, _) = raw_http(url, "POST", Some(body), Some(content_type))?;
    Ok(resp)
}

/// Fallback helper to execute secure HTTPS requests via platform-native utilities (`curl` or `powershell`).
fn https_fallback(url: &str, method: &str, body: Option<&str>, ct: Option<&str>) -> Result<(String, i64), String> {
    validate_url(url)?;
    if cfg!(target_os = "windows") {
        // Values are passed via environment variables, never interpolated
        // into the script, so quotes in url/body cannot inject commands.
        let mut cmd = Command::new("powershell");
        cmd.arg("-NoProfile").arg("-Command");
        cmd.env("PJ_URL", url)
           .env("PJ_BODY", body.unwrap_or(""))
           .env("PJ_CT", ct.unwrap_or("application/x-www-form-urlencoded"));
        let script = if method == "POST" {
            "$r = Invoke-WebRequest -Uri $env:PJ_URL -Method Post -Body $env:PJ_BODY -ContentType $env:PJ_CT -UseBasicParsing; Write-Output \"$($r.StatusCode)`n$($r.Content)\""
        } else {
            "$r = Invoke-WebRequest -Uri $env:PJ_URL -Method Get -UseBasicParsing; Write-Output \"$($r.StatusCode)`n$($r.Content)\""
        };
        let out = cmd.arg(script).output().map_err(|e| e.to_string())?;
        let resp = String::from_utf8_lossy(&out.stdout);
        let mut lines = resp.lines();
        let code = lines.next().and_then(|s| s.trim().parse::<i64>().ok()).unwrap_or(200);
        let body_out: Vec<&str> = lines.collect();
        Ok((body_out.join("\n"), code))
    } else {
        let mut cmd = Command::new("curl");
        // restrict protocols (no file://, ftp://, ...) and add a timeout
        cmd.args(["-s", "-i", "--proto", "=http,https", "--max-time", "15", "-X", method]);
        if method == "POST" {
            if let Some(c) = ct { cmd.arg("-H").arg(format!("Content-Type: {}", c)); }
            if let Some(b) = body { cmd.arg("-d").arg(b); }
        }
        // `--` ends option parsing so a URL can never be read as a flag
        cmd.arg("--").arg(url);
        let out = cmd.output().map_err(|e| e.to_string())?;
        let resp = String::from_utf8_lossy(&out.stdout).to_string();

        let mut parts = resp.splitn(2, "\r\n\r\n");
        let headers = parts.next().unwrap_or("");
        let body_out = parts.next().unwrap_or("").to_string();

        let status_code = headers
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|code| code.parse().ok())
            .unwrap_or(200);

        Ok((body_out, status_code))
    }
}

/// Reject anything that is not a plain http(s) URL. Blocks other schemes,
/// quote characters (shell/PowerShell injection) and control characters
/// (CRLF header injection in the raw TCP path).
fn validate_url(url: &str) -> Result<(), String> {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("only http:// and https:// URLs are allowed".into());
    }
    if url.chars().any(|c| c.is_control() || c == ' ' || c == '\'' || c == '"' || c == '`') {
        return Err("invalid characters in URL".into());
    }
    Ok(())
}

fn raw_http(url: &str, method: &str, body: Option<&str>, ct: Option<&str>) -> Result<(String, i64), String> {
    validate_url(url)?;
    let parts = parse_url_parts(url);
    let (scheme, host, port_str, path, query) = (&parts[0], &parts[1], &parts[2], &parts[3], &parts[4]);

    // Use platform HTTPS executor if scheme is HTTPS
    if scheme == "https" {
        return https_fallback(url, method, body, ct);
    }

    let port: u16 = port_str.parse().unwrap_or(80);
    let full_path = if query.is_empty() { path.clone() } else { format!("{}?{}", path, query) };
    let addr = format!("{}:{}", host, port);

    let mut stream = TcpStream::connect(&addr)
        .map_err(|e| format!("connect {addr}: {e}"))?;
    stream.set_read_timeout(Some(Duration::from_secs(15))).ok();
    stream.set_write_timeout(Some(Duration::from_secs(10))).ok();

    let body_str = body.unwrap_or("");
    let ct_str   = ct.unwrap_or("application/x-www-form-urlencoded");
    let request  = if method == "POST" {
        format!("{} {} HTTP/1.1\r\nHost: {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            method, full_path, host, ct_str, body_str.len(), body_str)
    } else {
        format!("{} {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
            method, full_path, host)
    };

    stream.write_all(request.as_bytes()).map_err(|e| e.to_string())?;

    let mut reader = BufReader::new(stream);

    // Parse status line
    let mut status_line = String::new();
    reader.read_line(&mut status_line).map_err(|e| e.to_string())?;
    let status_code: i64 = status_line.split_whitespace().nth(1)
        .and_then(|s| s.parse().ok()).unwrap_or(0);

    // Skip headers
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).map_err(|e| e.to_string())?;
        if line == "\r\n" || line.is_empty() { break; }
    }

    // Read body
    let mut resp_body = String::new();
    reader.read_to_string(&mut resp_body).map_err(|e| e.to_string())?;

    Ok((resp_body, status_code))
}

fn eval_toml_builtin(name:&str,args:Vec<Value>)->Value{
    match name{
        ""=>Value::Null,
        _=>{eprintln!("toml.{}() not found", name); std::process::exit(1);}
    }
}
fn eval_yaml_builtin(name:&str,args:Vec<Value>)->Value{
    match name{
        ""=>Value::Null,
        _=>{eprintln!("yaml.{}() not found", name); std::process::exit(1);}
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
        "math"               => Some(eval_math_builtin(name, args)),
        "cmath"              => Some(eval_cmath_builtin(name, args)),
        "json"               => Some(eval_json_builtin(name, args)),
        "os"                 => Some(eval_os_builtin(name, args)),
        "system"             => Some(eval_sys_builtin(name, args)),
        "time"               => Some(eval_time_builtin(name, args)),
        "base64"             => Some(eval_base64_builtin(name, args)),
        "hash"               => Some(eval_hash_builtin(name, args)),
        "random"             => Some(eval_random_builtin(name, args)),
        "regex"              => Some(eval_regex_builtin(name, args)),
        "gui"                => Some(eval_gui_builtin(name, args)),
        "sqlite3" | "sqllite3"=> Some(eval_sqlite3_builtin(name, args)),
        "net"                => Some(eval_net_builtin(name, args)),
        "toml"               => Some(eval_toml_builtin(name,args)),
        "yaml"|"yml"         => Some(eval_yaml_builtin(name,args)),
        _ => None,
    }
}
