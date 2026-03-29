#![cfg(not(any(target_os = "ios", target_os = "android")))]

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log::info!("VoxParty starting (desktop)");
    voxparty::run();
}
