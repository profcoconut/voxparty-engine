pub mod platform;
pub mod core;
pub mod game;
pub mod assets;

use platform::{Platform, TouchHandler};

pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log::info!("VoxParty starting");

    let mut plat = Platform::new("VoxParty", 1280, 720);
    let mut touch = TouchHandler::new(1280, 720);

    loop {
        for event in plat.event_pump.poll_iter() {
            touch.handle_event(&event);
            match event {
                sdl2::event::Event::Quit { .. } => return,
                _ => {}
            }
        }

        plat.clear(32, 32, 32, 255);
        plat.present();
        plat.delay(16);
    }
}
