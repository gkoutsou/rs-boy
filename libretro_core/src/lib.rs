use io::keys::Key;
use lib_rs_boy::gameboy::GameBoy;
use lib_rs_boy::io;
use rust_libretro::{
    contexts::*, core::Core, env_version, input_descriptors, proc::*, retro_core, sys::*, types::*,
};
use std::ffi::CString;
use std::slice;

const FRAMERATE: f64 = 60.0; // 59.7275;

const INPUT_DESCRIPTORS: &[retro_input_descriptor] = &input_descriptors!(
    { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_UP, "Up" },
    { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_DOWN, "Down" },
    { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_LEFT, "Left" },
    { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_RIGHT, "Right" },
    { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_A, "Action" },
);

#[derive(CoreOptions)]
#[categories({
    "advanced_settings",
    "Advanced",
    "Options affecting low-level emulation performance and accuracy."
},{
    "not_so_advanced_settings",
    "Not So Advanced",
    "Options not affecting low-level emulation performance and accuracy."
})]
#[options({
    "foo_option_1",
    "Advanced > Speed hack coprocessor X",
    "Speed hack coprocessor X",
    "Setting 'Advanced > Speed hack coprocessor X' to 'true' or 'Turbo' provides increased performance at the expense of reduced accuracy",
    "Setting 'Speed hack coprocessor X' to 'true' or 'Turbo' provides increased performance at the expense of reduced accuracy",
    "advanced_settings",
    {
        { "false" },
        { "true" },
        { "unstable", "Turbo (Unstable)" },
    }
}, {
    "foo_option_2",
    "Simple > Toggle Something",
    "Toggle Something",
    "Setting 'Simple > Toggle Something' to 'true' does something.",
    "Setting 'Toggle Something' to 'true' does something.",
    "not_so_advanced_settings",
    {
        { "false" },
        { "true" },
    }
})]
struct RsBoyCore {
    game_boy: GameBoy,
    option_1: bool,
    option_2: bool,

    timer: i64,
    even: bool,
}

retro_core!(RsBoyCore {
    game_boy: GameBoy::new(),

    option_1: false,
    option_2: true,

    timer: 5_000_001,
    even: true,
});

impl Core for RsBoyCore {
    fn get_info(&self) -> SystemInfo {
        SystemInfo {
            library_name: CString::new("RS-Boy").unwrap(),
            library_version: CString::new(env_version!("CARGO_PKG_VERSION").to_string()).unwrap(),
            valid_extensions: CString::new("gb").unwrap(),

            need_fullpath: false,
            block_extract: false,
        }
    }

    fn on_set_environment(&mut self, initial: bool, ctx: &mut SetEnvironmentContext) {
        if !initial {
            return;
        }

        ctx.set_support_no_game(false);
    }

    fn on_init(&mut self, ctx: &mut InitContext) {
        let gctx: GenericContext = ctx.into();
        gctx.set_input_descriptors(INPUT_DESCRIPTORS);
    }

    fn on_get_av_info(&mut self, _ctx: &mut GetAvInfoContext) -> retro_system_av_info {
        retro_system_av_info {
            geometry: retro_game_geometry {
                base_width: 160,
                base_height: 144,
                max_width: 160,
                max_height: 144,
                aspect_ratio: 0.0,
            },
            timing: retro_system_timing {
                fps: FRAMERATE,
                sample_rate: lib_rs_boy::gameboy::audio::AUDIO_SAMPLE_RATE as f64,
            },
        }
    }

    fn on_load_game(
        &mut self,
        _info: Option<retro_game_info>,
        ctx: &mut LoadGameContext,
    ) -> Result<(), Box<dyn std::error::Error>> {
        ctx.set_pixel_format(PixelFormat::XRGB8888);
        ctx.set_performance_level(0);
        ctx.enable_frame_time_callback((1000000.0f64 / 60.0).round() as retro_usec_t);

        let size = _info.unwrap().size;
        let rom_data: &[u8] =
            unsafe { slice::from_raw_parts(_info.unwrap().data as *const u8, size) };
        self.game_boy.load_rom(rom_data.to_vec());

        let gctx: GenericContext = ctx.into();
        gctx.enable_audio_callback();

        Ok(())
    }

    fn on_options_changed(&mut self, ctx: &mut OptionsChangedContext) {
        match ctx.get_variable("foo_option_1") {
            Some("true") => self.option_1 = true,
            Some("false") => self.option_1 = false,
            _ => (),
        }

        match ctx.get_variable("foo_option_2") {
            Some("true") => self.option_2 = true,
            Some("false") => self.option_2 = false,
            _ => (),
        }
    }

    #[inline]
    fn on_run(&mut self, ctx: &mut RunContext, delta_us: Option<i64>) {
        let gctx: GenericContext = ctx.into();

        self.timer += delta_us.unwrap_or(16_666);

        let input = unsafe { ctx.get_joypad_state(0, 0) };

        if input.contains(JoypadState::START) && input.contains(JoypadState::SELECT) {
            return gctx.shutdown();
        }

        let mut output: Vec<Key> = Vec::new();
        if input.contains(JoypadState::START) {
            output.push(Key::Enter);
        }
        if input.contains(JoypadState::SELECT) {
            output.push(Key::Backspace);
        }
        if input.contains(JoypadState::UP) {
            output.push(Key::Up);
        }
        if input.contains(JoypadState::DOWN) {
            output.push(Key::Down);
        }
        if input.contains(JoypadState::LEFT) {
            output.push(Key::Left);
        }
        if input.contains(JoypadState::RIGHT) {
            output.push(Key::Right);
        }

        if input.contains(JoypadState::A) {
            output.push(Key::X);
        }
        if input.contains(JoypadState::B) {
            output.push(Key::Z);
        }

        loop {
            let render = self.game_boy.step();
            if render {
                self.game_boy.set_pressed_keys(output);
                break;
            }
        }

        self.timer = 0;
        self.even = !self.even;

        unsafe {
            let (prefix, bytes, suffix) = self
                .game_boy
                .display
                .engine
                .screen
                .as_slice()
                .align_to::<u8>();

            ctx.draw_frame(bytes, 160, 144, 160usize * 4);
        }
    }

    fn on_write_audio(&mut self, ctx: &mut AudioContext) {
        let samples = self.game_boy.speaker.get_samples();

        // TODO there is a small mismatch on the number of samples
        if self.game_boy.speaker.is_buffer_full() {
            ctx.batch_audio_samples(samples);
            self.game_boy.speaker.empty_buffer();
        }
    }
}
