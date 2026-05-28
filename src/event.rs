use std::time::Duration;

use crossterm::event::{Event as CrosstermEvent};
use tokio::sync::mpsc;
use tokio::time::interval;

use crate::model::AppEvent;

pub struct EventHandler {
    /// Receiver half — handed to the caller so the main loop can drain it.
    rx: mpsc::UnboundedReceiver<AppEvent>,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::unbounded_channel::<AppEvent>();
        let tx_tick = tx.clone();
        tokio::spawn(async move{
            let mut ticker = interval(tick_rate);
            loop {
                ticker.tick().await;
                if tx_tick.send(AppEvent::Tick).is_err() {
                    break ;
                }
            }
        });

        let tx_input = tx;
        tokio::spawn(async move {
            loop {
                // crossterm::event::read is a plain fn pointer — no closure
                // needed, which avoids a move-capture issue.
                let result = tokio::task::spawn_blocking(crossterm::event::read).await;

                let keep_going = match result {
                    Ok(Ok(CrosstermEvent::Key(key))) => {
                        tx_input.send(AppEvent::Key(key)).is_ok()
                    }
                    Ok(Ok(CrosstermEvent::Resize(w, h))) => {
                        tx_input.send(AppEvent::Resize(w, h)).is_ok()
                    }
                    Ok(Ok(_)) => true, // mouse, focus, paste — ignore for now
                    Ok(Err(_)) => false, // crossterm read error — exit
                    Err(_) => false,    // spawn_blocking join error — exit
                };

                if !keep_going {
                    break;
                }
            }
        });

        Self { rx }
    }

    pub async fn next(&mut self) -> Option<AppEvent> {
        self.rx.recv().await
    }
}