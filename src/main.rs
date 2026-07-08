mod payjar;

use std::env;
use std::io::{self, Write};

fn cli() {
    println!("Payjar 1.3.0 cli");
    let mut multiline = false;
    let mut block = String::new();

    loop {
        if multiline { print!("... "); } else { print!(">>> "); }
        io::stdout().flush().unwrap();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            println!();
            break;
        }

        let line = input.trim();

        if multiline {
            if line.is_empty() {
                payjar::interpret(
                    &format!("public class main(@self){{ {} }}", block),
                    false,
                );
                block.clear();
                multiline = false;
            } else {
                block.push_str(line);
                block.push('\n');
            }
            continue;
        }

        if line == "exit()" || line == "quit()" {
            println!("Bye.");
            break;
        }

        // Multiline starters — buffer rather than execute immediately
        if line.starts_with("func ")
            || line.starts_with("class ")
            || line.starts_with("if ")
            || line.starts_with("for ")
            || line.starts_with("while ")
        {
            multiline = true;
            block.push_str(line);
            block.push('\n');
            continue;
        }

        if !line.is_empty() {
            payjar::interpret(
                &format!("public class main(@self){{ {} }}", line),
                false,
            );
        }
    }
}

fn main() {
    // Initialise the GUI channel BEFORE spawning any threads.
    payjar::gui_channel::init();

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        payjar::print_usage_pjc();
        return;
    }

    match args[1].as_str() {
        "cli" => { cli(); }
        "version" => { println!("1.3.0"); }
        "help" => { payjar::print_usage_pjc(); }
        "autorun" => {
            let debug = args.iter().any(|a| a == "-d");
            let skip = args.windows(2)
                .find(|w| w[0] == "-s")
                .map(|w| w[1].clone())
                .unwrap_or_default();
            let handle = std::thread::spawn(move || {
                payjar::autorun(debug, &skip);
            });
            run_gui_loop(handle);
        }
        "-d" => {
            if let Some(path) = args.get(2) {
                let path = path.clone();
                let handle = std::thread::spawn(move || {
                    payjar::interpret_file(&path, true);
                });
                run_gui_loop(handle);
            } else {
                payjar::print_usage_pjc();
            }
        }
        path => {
            let path = path.to_string();
            let handle = std::thread::spawn(move || {
                payjar::interpret_file(&path, false);
            });
            run_gui_loop(handle);
        }
    }
}

/// Pumps GUI requests on the main thread until the interpreter thread finishes.
/// Uses `drain_one`'s internal condvar sleep to avoid spinning at 100 % CPU.
fn run_gui_loop(handle: std::thread::JoinHandle<()>) {
    loop {
        payjar::gui_channel::drain_one();

        if handle.is_finished() {
            // Drain any final requests queued just before the thread exited.
            while payjar::gui_channel::drain_one() {}
            handle.join().ok();
            return;
        }
    }
}
