pub mod platform;
pub mod core;
pub mod game;
pub mod assets;

pub fn run() {
    loop {
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
}
