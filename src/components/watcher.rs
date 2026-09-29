use std::{path::Path, sync::mpsc, time::Duration};

use notify_debouncer_mini::{DebounceEventResult, new_debouncer, notify::RecursiveMode};

use crate::components::{config::Config, voice::Voice};

pub struct Watcher {
    monitored_path: String,
    voice: Voice,
}

impl Watcher {
    pub fn new() -> Watcher {
        let config = Config::new();
        let monitored_path = config.monitored_path.clone();
        let voice = Voice::new(config);

        Watcher {
            monitored_path,
            voice,
        }
    }

    pub fn watch(mut self) {
        let (tx, rx) = mpsc::channel::<DebounceEventResult>();
        let mut debouncer = new_debouncer(Duration::from_secs(1), tx).unwrap();
        debouncer
            .watcher()
            .watch(Path::new(&self.monitored_path), RecursiveMode::Recursive)
            .unwrap();

        for res in rx {
            match res {
                Ok(event) => {
                    for e in event {
                        self.voice.append_from_event(e);
                    }
                }
                Err(e) => println!("watch error: {:?}", e),
            }

            self.voice.wait();
        }
    }
}
