use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crossterm::event::{self, Event as CtEvent, KeyEvent};

/// Application events delivered to the main loop.
#[derive(Debug)]
pub enum Event {
    /// A key was pressed.
    Key(KeyEvent),
    /// Periodic redraw tick (interval set at construction).
    Tick,
}

/// Spawns a background thread that forwards terminal events and ticks.
pub struct EventHandler {
    rx: mpsc::Receiver<Event>,
}

impl EventHandler {
    /// `tick_rate` — how often `Event::Tick` is emitted when no key arrives.
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            loop {
                if event::poll(tick_rate).unwrap_or(false) {
                    if let Ok(CtEvent::Key(key)) = event::read() {
                        if tx.send(Event::Key(key)).is_err() {
                            return; // receiver dropped — exit thread
                        }
                    }
                } else {
                    if tx.send(Event::Tick).is_err() {
                        return;
                    }
                }
            }
        });
        Self { rx }
    }

    /// Block until the next event arrives.
    pub fn next(&self) -> anyhow::Result<Event> {
        self.rx.recv().map_err(|e| anyhow::anyhow!("event channel closed: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn tick_events_arrive_within_deadline() {
        let handler = EventHandler::new(Duration::from_millis(50));
        let event = handler.next().unwrap();
        assert!(
            matches!(event, Event::Tick),
            "first event within 200ms should be a Tick"
        );
    }
}
