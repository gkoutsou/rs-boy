use std::env;

mod gameboy;

use env_logger::Env;
use gameboy::GameBoy;

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .target(env_logger::Target::Stdout)
        .init();

    let args: Vec<String> = env::args().collect();
    if args.len() <= 1 {
        panic!("Please provide a rom");
    }

    let path = args[1].as_str();

    let mut gb = GameBoy::new(path);
    gb.start();
}
