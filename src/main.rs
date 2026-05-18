use std::env;
use std::fs;
use std::io::Write;
use std::process;
use std::process::{Command, Stdio};
use eframe::*;
mod payjar;
use crate::payjar::*;

fn run_payjar(args: Vec<String>) {
    let exe = &args[0];
    let base = exe.rsplit(|c| c == '/' || c == '\\').next().unwrap_or(exe);
    let is_pjrt = base.starts_with("pjrt");
    if is_pjrt {
        if args.len() < 2 { eprintln!("Not enough arguments."); process::exit(1); }
        let mut debug = false; let mut argi = 1usize; let mut skip: &str = "";
        if argi < args.len() && args[argi] == "-d" { debug = true; argi += 1; }
        if argi < args.len() && args[argi] == "-s" { argi += 1; skip = &args[argi]; argi += 1; }
        if argi >= args.len() { eprintln!("Not enough arguments."); process::exit(1); }
        match args[argi].as_str() {
            "run" => { argi += 1; if argi >= args.len() { eprintln!("'run' needs a package name"); process::exit(1); } pjrt_run(debug, &args[argi], skip); }
            "help" => print_usage_pjrt(),
            other => { eprintln!("Command '{}' not found.", other); process::exit(1); }
        }
    } else {
        if args.len() < 2 { eprintln!("Not enough CLI arguments."); process::exit(1); }
        match args[1].as_str() {
            "help"    => print_usage_pjc(),
            "exec"    => interpret(&args[2], false),
            "autorun" => { if args.len() >= 4 && args[2] == "-s" { autorun(false, &args[3]); } else { autorun(false, ""); } }
            "-d" => {
                if args.len() < 3 { eprintln!("-d needs a file"); process::exit(1); }
                if args[2] == "autorun" { if args.len() > 3 && args[3] == "-s" { autorun(true, &args[4]); } else { autorun(true, ""); } }
                else if args[2] == "exec" { interpret(&args[3], true); }
                else { interpret_file(&args[2], true); }
            }
            f if f.contains('.') => interpret_file(f, false),
            other => { eprintln!("Command '{}' not found.", other); process::exit(1); }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum RunMode { PjcNormal, PjcDebug, Pjrt }
impl RunMode {
    fn label(self) -> &'static str {
        match self { RunMode::PjcNormal => "pjc (normal)", RunMode::PjcDebug => "pjc -d (debug)", RunMode::Pjrt => "pjrt (runtime)" }
    }
    fn all() -> &'static [RunMode] { &[RunMode::PjcNormal, RunMode::PjcDebug, RunMode::Pjrt] }
}
impl Default for RunMode { fn default() -> Self { RunMode::PjcNormal } }

struct PayjarIde {
    filename: String, code: String, output: String,
    stdin_buf: String, cursor_pos: usize,
    renaming: bool, rename_buf: String,
    run_mode: RunMode,
}
impl Default for PayjarIde {
    fn default() -> Self {
        Self { filename: String::new(), code: String::new(), output: String::new(),
               stdin_buf: String::new(), cursor_pos: 0,
               renaming: false, rename_buf: String::new(), run_mode: RunMode::PjcNormal }
    }
}

impl PayjarIde {
    fn do_save(&mut self) {
        let path = if self.filename.is_empty() { rfd::FileDialog::new().set_file_name("untitled.pj").save_file() }
                   else { Some(std::path::PathBuf::from(&self.filename)) };
        if let Some(p) = path {
            if let Err(e) = std::fs::write(&p, &self.code) { eprintln!("Save failed: {}", e); }
            else { self.filename = p.display().to_string(); }
        }
    }

    fn do_run(&mut self) {
        let tmp = if self.filename.is_empty() { "tmp_payjar.pj".to_string() } else { self.filename.clone() };
        if self.filename.is_empty() {
            if let Err(e) = std::fs::write(&tmp, &self.code) { self.output = format!("Error writing temp: {}", e); return; }
        }
        let cmd_args: Vec<String> = match self.run_mode {
            RunMode::PjcNormal => vec!["pjc".into(), tmp.clone()],
            RunMode::PjcDebug  => vec!["pjc".into(), "-d".into(), tmp.clone()],
            RunMode::Pjrt => {
                let pkg = std::path::Path::new(&tmp).file_stem()
                    .map(|s| s.to_string_lossy().to_string()).unwrap_or("main".into());
                vec!["pjrt".into(), "run".into(), pkg]
            }
        };
        self.output = run_with_stdin(&cmd_args, &self.stdin_buf);
    }
}

fn run_with_stdin(args: &[String], stdin_text: &str) -> String {
    if args.is_empty() { return "No command.".into(); }
    let mut cmd = Command::new(&args[0]);
    if args.len() > 1 { cmd.args(&args[1..]); }
    cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    match cmd.spawn() {
        Err(e) => format!("Failed to start '{}': {}", args[0], e),
        Ok(mut child) => {
            if let Some(mut s) = child.stdin.take() { let _ = s.write_all(stdin_text.as_bytes()); }
            match child.wait_with_output() {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                    let mut r = stdout;
                    if !stderr.trim().is_empty() { if !r.is_empty() { r.push('\n'); } r.push_str(stderr.trim()); }
                    r
                }
                Err(e) => format!("Wait error: {}", e),
            }
        }
    }
}

impl eframe::App for PayjarIde {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let ctrl = ctx.input(|i| i.modifiers.ctrl);
        if ctrl && ctx.input(|i| i.key_pressed(egui::Key::S)) { self.do_save(); }
        if ctrl && ctx.input(|i| i.key_pressed(egui::Key::R)) { self.do_run(); }

        let toggle_block = ctx.input_mut(|i| i.modifiers.shift && i.modifiers.alt && i.consume_key(egui::Modifiers::NONE, egui::Key::A));
        if toggle_block {
            if let Some(state) = egui::text_edit::TextEditState::load(ctx, egui::Id::new("code_editor")) {
                if let Some(cr) = state.cursor.char_range() {
                    let s = cr.primary.index.min(cr.secondary.index);
                    let e = cr.primary.index.max(cr.secondary.index);
                    if s != e {
                        let sel = &self.code[s..e];
                        let t = sel.trim();
                        let rep = if t.starts_with("/*") && t.ends_with("*/") { t.trim_start_matches("/*").trim_end_matches("*/").to_string() } else { format!("/*{}*/", sel) };
                        self.code.replace_range(s..e, &rep);
                    }
                }
            }
        }
        let toggle_line = ctrl && ctx.input(|i| i.key_pressed(egui::Key::Slash));
        if toggle_line {
            let code = self.code.clone();
            let lines: Vec<&str> = code.split('\n').collect();
            let mut boff = 0usize; let mut cl = 0usize;
            for (i, line) in lines.iter().enumerate() {
                let end = boff + line.len() + 1;
                if self.cursor_pos <= end || i+1 == lines.len() { cl = i; break; }
                boff = end;
            }
            let mut nl: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
            let line = &nl[cl]; let tr = line.trim_start(); let ind = &line[..line.len()-tr.len()];
            nl[cl] = if tr.starts_with("//") { format!("{}{}", ind, tr.strip_prefix("// ").unwrap_or_else(|| tr.strip_prefix("//").unwrap_or(tr))) }
                     else { format!("{}// {}", ind, tr) };
            self.code = nl.join("\n");
        }

        // Styling
        let mut style = (*ctx.style()).clone();
        style.visuals.window_fill = egui::Color32::from_rgb(18,18,24);
        style.visuals.panel_fill  = egui::Color32::from_rgb(18,18,24);
        style.visuals.extreme_bg_color = egui::Color32::from_rgb(12,12,16);
        style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(38,38,52);
        style.visuals.widgets.hovered.bg_fill  = egui::Color32::from_rgb(58,58,80);
        style.visuals.widgets.active.bg_fill   = egui::Color32::from_rgb(80,60,180);
        style.visuals.widgets.inactive.rounding = egui::Rounding::same(6.0);
        style.visuals.widgets.hovered.rounding  = egui::Rounding::same(6.0);
        style.visuals.widgets.active.rounding   = egui::Rounding::same(6.0);
        style.spacing.button_padding = egui::vec2(10.0,5.0);
        style.spacing.item_spacing   = egui::vec2(8.0,6.0);
        ctx.set_style(style);

        // ── Top toolbar ──────────────────────────────────────────────────────
        egui::TopBottomPanel::top("toolbar")
            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(24,24,34)).inner_margin(egui::Margin::symmetric(10.0,7.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Payjar IDE").color(egui::Color32::from_rgb(160,130,255)).size(16.0).strong());
                    ui.separator();

                    // Filename
                    let fname = if self.filename.is_empty() { "untitled.pj".to_string() }
                    else { std::path::Path::new(&self.filename).file_name().map(|n|n.to_string_lossy().to_string()).unwrap_or_else(||self.filename.clone()) };
                    if self.renaming {
                        let re = ui.add(egui::TextEdit::singleline(&mut self.rename_buf).desired_width(180.0).font(egui::TextStyle::Monospace));
                        re.request_focus();
                        let confirmed = ctx.input(|i| i.key_pressed(egui::Key::Enter));
                        let escaped   = ctx.input(|i| i.key_pressed(egui::Key::Escape));
                        if escaped { self.renaming = false; }
                        else if confirmed || re.lost_focus() {
                            let nn = self.rename_buf.trim().to_string();
                            if !nn.is_empty() && nn != fname {
                                let np = if self.filename.is_empty() { std::path::PathBuf::from(&nn) }
                                else { std::path::Path::new(&self.filename).parent().map(|p|p.to_path_buf()).unwrap_or_default().join(&nn) };
                                if !self.filename.is_empty() { let _ = std::fs::rename(&self.filename, &np); }
                                self.filename = np.display().to_string();
                            }
                            self.renaming = false;
                        }
                    } else {
                        let lr = ui.label(egui::RichText::new(&fname).color(egui::Color32::from_rgb(180,180,200)).size(13.0));
                        if lr.double_clicked() { self.rename_buf = fname.clone(); self.renaming = true; }
                        lr.on_hover_text("Double-click to rename");
                        if ui.button(egui::RichText::new("Rename file").size(14.0).color(egui::Color32::from_rgb(140,140,180))).clicked()
                        { self.rename_buf = fname.clone(); self.renaming = true; }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Open
                        if ui.add(egui::Button::new(egui::RichText::new("📂  Open").color(egui::Color32::WHITE).size(13.0)).fill(egui::Color32::from_rgb(60,60,80))).clicked() {
                            if let Some(path) = rfd::FileDialog::new().pick_file() {
                                self.filename = path.display().to_string();
                                if let Ok(c) = std::fs::read_to_string(&self.filename) { self.code = c; }
                            }
                        }
                        // Save
                        if ui.add(egui::Button::new(egui::RichText::new("💾  Save  [Ctrl+S]").color(egui::Color32::WHITE).size(13.0)).fill(egui::Color32::from_rgb(50,80,140))).clicked()
                        { self.do_save(); }

                        ui.separator();

                        // Run
                        if ui.add(egui::Button::new(egui::RichText::new("▶  Run  [Ctrl+R]").color(egui::Color32::WHITE).size(13.0)).fill(egui::Color32::from_rgb(50,130,80))).clicked()
                        { self.do_run(); }

                        ui.separator();

                        // Run-mode dropdown
                        ui.label(egui::RichText::new("Mode:").color(egui::Color32::from_rgb(160,160,190)).size(12.0));
                        let cur = self.run_mode;
                        egui::ComboBox::from_id_source("run_mode")
                            .selected_text(egui::RichText::new(cur.label()).size(12.0).color(egui::Color32::from_rgb(200,200,230)))
                            .width(155.0)
                            .show_ui(ui, |ui| {
                                for &m in RunMode::all() {
                                    if ui.selectable_label(cur == m, egui::RichText::new(m.label()).size(12.0)).clicked()
                                    { self.run_mode = m; }
                                }
                            });
                    });
                });
            });

        // ── Status bar ───────────────────────────────────────────────────────
        egui::TopBottomPanel::bottom("statusbar")
            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(24,24,34)).inner_margin(egui::Margin::symmetric(10.0,4.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("Lines: {}   Chars: {}", self.code.lines().count(), self.code.len()))
                        .color(egui::Color32::from_rgb(120,120,150)).size(11.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new("Ctrl+R Run  •  Ctrl+S Save  •  Ctrl+/ Comment  •  Ctrl+Z Undo")
                            .color(egui::Color32::from_rgb(100,100,130)).size(11.0));
                    });
                });
            });

        // ── Main panel ───────────────────────────────────────────────────────
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(18,18,24)).inner_margin(egui::Margin::same(8.0)))
            .show(ctx, |ui| {
                let total_h = ui.available_height();
                let editor_h = total_h * 0.56;
                let stdin_h  = 70.0;
                let output_h = (total_h - editor_h - stdin_h - 48.0).max(60.0);

                // Code editor
                egui::Frame::none().fill(egui::Color32::from_rgb(14,14,20)).rounding(egui::Rounding::same(8.0))
                    .stroke(egui::Stroke::new(1.0,egui::Color32::from_rgb(50,50,70))).inner_margin(egui::Margin::same(6.0))
                    .show(ui, |ui| {
                        egui::ScrollArea::both().id_source("editor_scroll").max_height(editor_h).auto_shrink([false,false])
                            .show(ui, |ui| {
                                let resp = ui.add(egui::TextEdit::multiline(&mut self.code).id(egui::Id::new("code_editor"))
                                    .desired_width(f32::INFINITY).desired_rows(22).code_editor()
                                    .hint_text("// Type your Payjar code here..."));
                                if resp.has_focus() {
                                    if let Some(st) = egui::text_edit::TextEditState::load(ctx, egui::Id::new("code_editor")) {
                                        if let Some(c) = st.cursor.char_range() { self.cursor_pos = c.primary.index; }
                                    }
                                }
                            });
                    });

                ui.add_space(6.0);

                // Stdin
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("▸ Stdin").color(egui::Color32::from_rgb(140,200,140)).size(12.0));
                    ui.label(egui::RichText::new("— one value per line (fed to readln / readi / readf)")
                        .color(egui::Color32::from_rgb(80,110,80)).size(11.0));
                });
                ui.add_space(2.0);
                egui::Frame::none().fill(egui::Color32::from_rgb(10,16,10)).rounding(egui::Rounding::same(6.0))
                    .stroke(egui::Stroke::new(1.0,egui::Color32::from_rgb(40,80,40))).inner_margin(egui::Margin::same(5.0))
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical().id_source("stdin_scroll").max_height(stdin_h).show(ui, |ui| {
                            ui.add(egui::TextEdit::multiline(&mut self.stdin_buf).desired_width(f32::INFINITY).desired_rows(3)
                                .code_editor().hint_text("e.g.  Alice\n42\n3.14")
                                .text_color(egui::Color32::from_rgb(140,220,140)));
                        });
                    });

                ui.add_space(6.0);

                // Output
                ui.label(egui::RichText::new("▸ Output").color(egui::Color32::from_rgb(140,140,180)).size(12.0));
                ui.add_space(2.0);
                egui::Frame::none().fill(egui::Color32::from_rgb(10,12,10)).rounding(egui::Rounding::same(8.0))
                    .stroke(egui::Stroke::new(1.0,egui::Color32::from_rgb(40,70,40))).inner_margin(egui::Margin::same(6.0))
                    .show(ui, |ui| {
                        egui::ScrollArea::both().id_source("output_scroll").max_height(output_h).auto_shrink([false,false])
                            .show(ui, |ui| {
                                ui.add(egui::TextEdit::multiline(&mut self.output).desired_width(f32::INFINITY)
                                    .code_editor().interactive(true).desired_rows(7)
                                    .text_color(egui::Color32::from_rgb(100,220,100)));
                            });
                    });
            });
    }
}

fn main() -> eframe::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() >= 2 {
        let base = std::path::Path::new(&args[0]).file_name()
            .map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        if base.starts_with("pjc") || base.starts_with("pjrt") {
            run_payjar(args); return Ok(());
        }
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([960.0, 740.0]).with_min_inner_size([600.0,460.0]).with_title("Payjar IDE"),
        ..Default::default()
    };
    eframe::run_native("Payjar IDE", options, Box::new(|_cc| Ok(Box::new(PayjarIde::default()))))
}