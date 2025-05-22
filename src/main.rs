use env_logger::Env;
use rs_boy::gameboy::GameBoy;
use std::env;

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .target(env_logger::Target::Stdout)
        .init();

    let mut use_speakers: bool = true;
    let mut rom_path: String = "".to_owned();

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-na" | "--no-audio" => use_speakers = false,
            _ => {
                if rom_path.len() > 0 {
                    panic!("can only pass one rom file");
                }
                rom_path = arg;
            }
        }
    }

    if rom_path.len() == 0 {
        panic!("Please provide a rom");
    }

    let mut gb = GameBoy::new(&rom_path);
    gb.start(use_speakers);
}
