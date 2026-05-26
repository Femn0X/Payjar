mod payjar;

use std::env;

fn main() {
    // Initialise the GUI channel BEFORE spawning any threads.
    payjar::gui_channel::init();

    let args: Vec<String> = env::args().collect();
    let bin = args.get(0).map(String::as_str).unwrap_or("payjar");

    if bin.ends_with("pjrt") {
        // ---- pjrt mode ----
        let mut debug = false;
        let mut skip = String::new();
        let mut pkg = String::new();
        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "help"  => { payjar::print_usage_pjrt(); return; }
                "-d"    => { debug = true; }
                "-s"    => { i += 1; if i < args.len() { skip = args[i].clone(); } }
                "run"   => { i += 1; if i < args.len() { pkg = args[i].clone(); } }
                _       => {}
            }
            i += 1;
        }
        if pkg.is_empty() { payjar::print_usage_pjrt(); return; }

        let handle = std::thread::spawn(move || {
            payjar::pjrt_run(debug, &pkg, &skip);
        });
        run_gui_loop(handle);

    } else {
        // ---- pjc mode ----
        if args.len() < 2 { payjar::print_usage_pjc(); return; }
        match args[1].as_str() {
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
}

/// Pumps pending GUI requests on the main thread until the interpreter
/// thread finishes, then waits for it to join cleanly.
fn run_gui_loop(handle: std::thread::JoinHandle<()>) {
    loop {
        // Drain any pending GUI request (runs eframe::run_native here,
        // on the main thread, satisfying winit's requirement).
        payjar::gui_channel::drain_one();

        // Check if the interpreter thread is done.
        if handle.is_finished() {
            // Drain any final requests it may have queued before exiting.
            while payjar::gui_channel::drain_one() {}
            handle.join().ok();
            return;
        }

        // Small sleep to avoid spinning the main thread at 100% CPU when
        // there are no GUI requests pending.
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}
