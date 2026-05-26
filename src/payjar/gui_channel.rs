//! gui_channel.rs
//!
//! Bridges the interpreter thread (which calls gui.alert / gui.confirm / etc.)
//! and the main thread (the only thread allowed to create a winit EventLoop).
//!
//! Flow:
//!   interpreter thread  →  gui_send(GuiRequest)  →  blocks on done_rx
//!   main thread         →  drain_gui_requests()  →  runs eframe, signals done_tx

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::OnceLock;

/// One GUI window request.
pub struct GuiRequest {
    pub title:   String,
    pub size:    [f32; 2],
    /// Boxed eframe app — must be Send so it can cross the channel.
    pub app:     Box<dyn eframe::App + Send>,
    /// Signal back to the interpreter thread when the window is closed.
    pub done_tx: Sender<()>,
}

// The global channel.  Initialised once; lives for the whole process.
static CHANNEL: OnceLock<Sender<GuiRequest>> = OnceLock::new();
static RECEIVER: OnceLock<std::sync::Mutex<Receiver<GuiRequest>>> = OnceLock::new();

/// Called once from `main()` before the interpreter thread is spawned.
pub fn init() {
    let (tx, rx) = mpsc::channel::<GuiRequest>();
    CHANNEL.set(tx).ok();
    RECEIVER.set(std::sync::Mutex::new(rx)).ok();
}

/// Called by the interpreter thread (via `spawn_wait` in builtins.rs).
pub fn gui_send(req: GuiRequest) {
    CHANNEL
        .get()
        .expect("gui_channel not initialised — call gui_channel::init() in main()")
        .send(req)
        .expect("GUI channel closed");
}

/// Called by the **main thread** in a loop while the interpreter is running.
/// Returns `true` if a request was processed, `false` if the channel was empty.
pub fn drain_one() -> bool {
    let lock = RECEIVER
        .get()
        .expect("gui_channel not initialised");
    let rx = lock.lock().unwrap();

    // Non-blocking try_recv so the main thread can also check if the
    // interpreter thread has finished.
    match rx.try_recv() {
        Ok(req) => {
            let opts = eframe::NativeOptions {
                viewport: egui::ViewportBuilder::default()
                    .with_title(&req.title)
                    .with_inner_size(req.size)
                    .with_resizable(false),
                ..Default::default()
            };
            let app = req.app;
            // run_native blocks until the window is closed.
            let _ = eframe::run_native(
                &req.title,
                opts,
                Box::new(move |_| app),
            );
            // Tell the interpreter thread it can continue.
            let _ = req.done_tx.send(());
            true
        }
        Err(_) => false,
    }
}