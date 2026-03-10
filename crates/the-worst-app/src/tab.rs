use std::sync::Arc;
use crossbeam_channel::Sender;
use parking_lot::Mutex;
use the_worst_core::grid::TerminalGrid;
use the_worst_core::pty::{self, InputEvent, PtyEvent, PtyHandle};

pub struct Tab {
    pub title: String,
    pub handle: PtyHandle,
    pub exited: bool,
}

impl Tab {
    pub fn new(cols: u16, rows: u16, repaint_tx: Sender<PtyEvent>) -> Self {
        let handle = pty::spawn(cols, rows, repaint_tx);
        Self {
            title: "The-Worst".to_string(),
            handle,
            exited: false,
        }
    }

    pub fn send_input(&self, bytes: Vec<u8>) {
        let _ = self.handle.input_tx.send(InputEvent::Bytes(bytes));
    }

    pub fn send_resize(&self, cols: u16, rows: u16) {
        let _ = self.handle.input_tx.send(InputEvent::Resize(cols, rows));
    }

    pub fn grid(&self) -> Arc<Mutex<TerminalGrid>> {
        Arc::clone(&self.handle.grid)
    }

    /// Drain pending events. Returns true if any redraw occurred.
    pub fn poll(&mut self) -> bool {
        let mut dirty = false;
        while let Ok(ev) = self.handle.event_rx.try_recv() {
            match ev {
                PtyEvent::Redraw => dirty = true,
                PtyEvent::TitleChanged(t) => self.title = t,
                PtyEvent::Exited => {
                    self.exited = true;
                    dirty = true;
                }
            }
        }
        dirty
    }
}
