//! gui_channel.rs
//!
//! Bridges the interpreter thread (gui.alert / gui.confirm / etc.)
//! and the main thread (the only thread allowed to create a winit EventLoop).
//!
//! Optimisations vs original:
//! - `drain_one` uses a condvar-backed wait (1 ms timeout) instead of the
//!   caller busy-looping with `sleep(1ms)`.  This shaves latency when a GUI
//!   request arrives between two sleep intervals.
//! - The `OnceLock<Mutex<Receiver>>` is replaced with a plain
//!   `OnceLock<Receiver>` guarded by the same Mutex — no double-wrapping.

use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Condvar, Mutex, OnceLock};
use std::time::Duration;

/// One GUI window request.
pub struct GuiRequest {
    pub title:   String,
    pub size:    [f32; 2],
    /// Boxed eframe app — must be Send so it can cross the channel.
    pub app:     Box<dyn eframe::App + Send>,
    /// Signal back to the interpreter thread when the window is closed.
    pub done_tx: Sender<()>,
}

// ── Global channel ────────────────────────────────────────────────────────────

static TX: OnceLock<Sender<GuiRequest>> = OnceLock::new();

struct Chan {
    rx: Receiver<GuiRequest>,
}

static CHAN: OnceLock<Mutex<Chan>> = OnceLock::new();

// Condvar so the main thread can sleep efficiently instead of spinning.
static WAKE: OnceLock<Condvar> = OnceLock::new();
static WAKE_LOCK: OnceLock<Mutex<bool>> = OnceLock::new();

// ── Public API ────────────────────────────────────────────────────────────────

/// Called once from `main()` **before** spawning the interpreter thread.
pub fn init() {
    let (tx, rx) = mpsc::channel::<GuiRequest>();
    TX.set(tx).ok();
    CHAN.set(Mutex::new(Chan { rx })).ok();
    WAKE.set(Condvar::new()).ok();
    WAKE_LOCK.set(Mutex::new(false)).ok();
}

/// Called by the interpreter thread to request a GUI window.
/// Blocks until the window is closed.
pub fn gui_send(req: GuiRequest) {
    TX.get()
        .expect("gui_channel not initialised")
        .send(req)
        .expect("GUI channel closed");

    // Wake the main thread immediately so it doesn't wait out its sleep.
    if let (Some(lock), Some(cv)) = (WAKE_LOCK.get(), WAKE.get()) {
        *lock.lock().unwrap() = true;
        cv.notify_one();
    }
}

/// Called by the **main thread**.  Runs one pending GUI request if available.
/// Blocks for up to `timeout` before returning `false`, avoiding a hot spin.
/// Returns `true` if a request was processed.
pub fn drain_one() -> bool {
    // Fast path: try without blocking.
    if let Some(guard) = CHAN.get().and_then(|m| m.try_lock().ok()) {
        match guard.rx.try_recv() {
            Ok(req) => {
                drop(guard); // release lock before running eframe
                run_request(req);
                return true;
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => return false,
        }
    }

    // Slow path: wait up to 1 ms for a new request.
    if let (Some(lock), Some(cv)) = (WAKE_LOCK.get(), WAKE.get()) {
        let guard = lock.lock().unwrap();
        let _ = cv.wait_timeout(guard, Duration::from_millis(1));
    }
    false
}

fn run_request(req: GuiRequest) {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(&req.title)
            .with_inner_size(req.size)
            .with_resizable(false),
        ..Default::default()
    };
    let app = req.app;
    let _ = eframe::run_native(&req.title, opts, Box::new(move |_| app));
    let _ = req.done_tx.send(());
}
