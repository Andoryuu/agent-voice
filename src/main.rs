use crate::components::watcher::Watcher;

mod components;

fn main() {
    Watcher::new().watch();
}
