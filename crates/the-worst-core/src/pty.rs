use std::io::{Read, Write};
use std::sync::Arc;
use crossbeam_channel::{Receiver, Sender};
use parking_lot::Mutex;
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
#[allow(unused_imports)]
use portable_pty::MasterPty;

use crate::grid::TerminalGrid;
use crate::performer::Performer;

pub enum PtyEvent {
    Redraw,
    TitleChanged(String),
    Exited,
}

pub enum InputEvent {
    Bytes(Vec<u8>),
    Resize(u16, u16),
}

pub struct PtyHandle {
    pub grid: Arc<Mutex<TerminalGrid>>,
    pub event_rx: Receiver<PtyEvent>,
    pub input_tx: Sender<InputEvent>,
}

pub fn spawn(
    cols: u16,
    rows: u16,
    repaint_tx: Sender<PtyEvent>,
) -> PtyHandle {
    let grid = Arc::new(Mutex::new(TerminalGrid::new(cols as usize, rows as usize)));
    let (event_tx, event_rx) = crossbeam_channel::unbounded::<PtyEvent>();
    let (input_tx, input_rx) = crossbeam_channel::unbounded::<InputEvent>();

    let grid_clone = Arc::clone(&grid);
    let event_tx_clone = event_tx.clone();

    std::thread::Builder::new()
        .name("pty-main".into())
        .spawn(move || {
            run_pty(
                cols,
                rows,
                grid_clone,
                event_tx_clone,
                input_rx,
                repaint_tx,
            );
        })
        .expect("spawn pty thread");

    PtyHandle { grid, event_rx, input_tx }
}

fn run_pty(
    cols: u16,
    rows: u16,
    grid: Arc<Mutex<TerminalGrid>>,
    event_tx: Sender<PtyEvent>,
    input_rx: Receiver<InputEvent>,
    repaint_tx: Sender<PtyEvent>,
) {
    let pty_system = NativePtySystem::default();
    let pair = pty_system
        .openpty(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
        .expect("openpty");

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
    let mut cmd = CommandBuilder::new(&shell);
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");

    let mut child = pair.slave.spawn_command(cmd).expect("spawn shell");
    drop(pair.slave);

    let master = pair.master;
    let mut reader = master.try_clone_reader().expect("clone reader");

    // Writer thread — takes ownership of master for resize + writer
    std::thread::Builder::new()
        .name("pty-writer".into())
        .spawn(move || {
            let mut writer = master.take_writer().expect("take_writer");
            for event in input_rx {
                match event {
                    InputEvent::Bytes(bytes) => {
                        let _ = writer.write_all(&bytes);
                    }
                    InputEvent::Resize(c, r) => {
                        let _ = master.resize(PtySize {
                            rows: r,
                            cols: c,
                            pixel_width: 0,
                            pixel_height: 0,
                        });
                    }
                }
            }
        })
        .expect("spawn writer thread");

    // Reader thread
    let mut parser = vte::Parser::new();
    let mut buf = [0u8; 8192];

    loop {
        match reader.read(&mut buf) {
            Ok(0) | Err(_) => {
                let _ = event_tx.send(PtyEvent::Exited);
                let _ = repaint_tx.send(PtyEvent::Exited);
                break;
            }
            Ok(n) => {
                let mut title_buf = String::new();
                let mut title_changed = false;

                {
                    let mut g = grid.lock();
                    let mut perf = Performer {
                        grid: &mut g,
                        title_buf: &mut title_buf,
                        title_changed: &mut title_changed,
                    };
                    for &byte in &buf[..n] {
                        parser.advance(&mut perf, byte);
                    }
                }

                if title_changed {
                    let _ = event_tx.send(PtyEvent::TitleChanged(title_buf));
                }
                let _ = event_tx.send(PtyEvent::Redraw);
                // Also signal the repaint channel (which holds egui ctx wakeup)
                let _ = repaint_tx.try_send(PtyEvent::Redraw);
            }
        }
    }

    let _ = child.wait();
}
