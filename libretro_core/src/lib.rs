use io::keys::Key;
use lib_rs_boy::gameboy::GameBoy;
use lib_rs_boy::io;
use rust_libretro::core::CoreOptions;
use rust_libretro::{
    contexts::*, core::Core, env_version, input_descriptors, proc::*, retro_core, sys::*, types::*,
};
use std::ffi::CString;
use std::ptr::null_mut;
use std::slice;

const FRAMERATE: f64 = 60.0; // 59.7275;
const RUMBLE_OFF: u16 = 0;
const RUMBLE_LOW: u16 = 65535 / 3;
const RUMBLE_MEDIUM: u16 = RUMBLE_LOW * 2;
const RUMBLE_HIGH: u16 = 65535;

const INPUT_DESCRIPTORS: &[retro_input_descriptor] = &input_descriptors!(
    { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_UP, "Up" },
    { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_DOWN, "Down" },
    { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_LEFT, "Left" },
    { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_RIGHT, "Right" },
    { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_A, "A" },
    { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_B, "B" },
    { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_START, "Start" },
    { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_SELECT, "Select" },
);

#[derive(CoreOptions)]
#[categories({
    "sound_settings",
    "Sound",
    "Options affecting the audio channels."
},{
    "gamepad_settings",
    "Gamepad",
    "Options affecting the controller."
})]
#[options({
    "disable_channel_1",
    "Audio > Disable Channel 1",
    "Disable audio channel 1",
    "Setting 'Audio > Disable Channel 1' disables the first audio channel",
    "Setting 'Disable Channel 1' disables the first audio channel",
    "sound_settings",
    {
        { "false" },
        { "true" },
    },
    "false"
},{
    "disable_channel_2",
    "Audio > Disable Channel 2",
    "Disable audio channel 2",
    "Setting 'Audio > Disable Channel 2' disables the second audio channel",
    "Setting 'Disable Channel 2' disables the first second channel",
    "sound_settings",
    {
        { "false" },
        { "true" },
    },
    "false"
},{
    "disable_channel_3",
    "Audio > Disable Channel 3",
    "Disable audio channel 3",
    "Setting 'Audio > Disable Channel 3' disables the third audio channel",
    "Setting 'Disable Channel 3' disables the third audio channel",
    "sound_settings",
    {
        { "false" },
        { "true" },
    },
    "false"
},{
    "disable_channel_4",
    "Audio > Disable Channel 4",
    "Disable audio channel 4",
    "Setting 'Audio > Disable Channel 4' disables the fourth audio channel",
    "Setting 'Disable Channel 4' disables the fourth audio channel",
    "sound_settings",
    {
        { "false" },
        { "true" },
    },
    "false"
},
{
    "rumble_intensity",
    "Gamepad > Rumble Intensity",
    "Rumble intensity",
    "Setting 'Gamepad > Rumble Intensity' controls the rumble intensity for supported games",
    "Setting 'Rumble Intensity' controls the rumble intensity for supported games",
    "gamepad_settings",
    {
        { "OFF" },
        { "LOW" },
        { "MEDIUM" },
        { "HIGH" },
    },
    "HIGH"
})]
struct RsBoyCore {
    game_boy: GameBoy,
    options: RsBoyCoreOptions,

    timer: i64,
    even: bool,
}

retro_core!(RsBoyCore {
    game_boy: GameBoy::new(),
    options: RsBoyCoreOptions{
        rumble_intensity_level: 65535,
    },

    timer: 5_000_001,
    even: true,
});

struct RsBoyCoreOptions {
    rumble_intensity_level: u16,
}

impl Core for RsBoyCore {
    fn get_info(&self) -> SystemInfo {
        SystemInfo {
            library_name: CString::new("rs_boy").unwrap(),
            library_version: CString::new(env_version!("CARGO_PKG_VERSION").to_string()).unwrap(),
            valid_extensions: CString::new("gb|gbc").unwrap(),

            need_fullpath: false,
            block_extract: false,
        }
    }

    fn on_set_environment(&mut self, initial: bool, ctx: &mut SetEnvironmentContext) {
        // Strange but if I move this after the !initial check, the settings are not effective
        self.set_core_options(ctx);
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

        let rom_size = _info.unwrap().size;
        let rom_data: &[u8] =
            unsafe { slice::from_raw_parts(_info.unwrap().data.cast(), rom_size) };
        self.game_boy.load_rom(rom_data.to_vec());

        if let Some(_) = self.game_boy.cartridge.get_rumble_state() {
            ctx.enable_rumble_interface()?;
        }

        let gctx: GenericContext = ctx.into();
        gctx.enable_audio_callback();

        Ok(())
    }

    fn on_options_changed(&mut self, ctx: &mut OptionsChangedContext) {
        match ctx.get_variable("disable_channel_1") {
            Some("true") => self.game_boy.speaker.set_channel_state(1, false),
            Some("false") => self.game_boy.speaker.set_channel_state(1, true),
            _ => (),
        }
        match ctx.get_variable("disable_channel_2") {
            Some("true") => self.game_boy.speaker.set_channel_state(2, false),
            Some("false") => self.game_boy.speaker.set_channel_state(2, true),
            _ => (),
        }
        match ctx.get_variable("disable_channel_3") {
            Some("true") => self.game_boy.speaker.set_channel_state(3, false),
            Some("false") => self.game_boy.speaker.set_channel_state(3, true),
            _ => (),
        }
        match ctx.get_variable("disable_channel_4") {
            Some("true") => self.game_boy.speaker.set_channel_state(4, false),
            Some("false") => self.game_boy.speaker.set_channel_state(4, true),
            _ => (),
        }
        match ctx.get_variable("rumble_intensity") {
            Some("OFF") => self.options.rumble_intensity_level = RUMBLE_OFF,
            Some("LOW") => self.options.rumble_intensity_level = RUMBLE_LOW,
            Some("MEDIUM") => self.options.rumble_intensity_level = RUMBLE_MEDIUM,
            Some("HIGH") => self.options.rumble_intensity_level = RUMBLE_HIGH,
            _ => (),
        }
    }

    #[inline]
    fn on_run(&mut self, ctx: &mut RunContext, delta_us: Option<i64>) {
        let gctx: GenericContext = ctx.into();

        self.timer += delta_us.unwrap_or(16_666);

        let input = ctx.get_joypad_state(0, 0);

        if input.contains(JoypadState::START) && input.contains(JoypadState::SELECT) {
            return gctx.shutdown();
        }

        let mut output: Vec<Key> = Vec::new();
        if input.contains(JoypadState::START) {
            output.push(Key::Start);
        }
        if input.contains(JoypadState::SELECT) {
            output.push(Key::Select);
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
            output.push(Key::A);
        }
        if input.contains(JoypadState::B) {
            output.push(Key::B);
        }

        loop {
            let render = self.game_boy.step();
            if render {
                self.game_boy.set_pressed_keys(output);
                if let Some(rumbling) = self.game_boy.cartridge.get_rumble_state() {
                    let strength = (rumbling as u16) * self.options.rumble_intensity_level;
                    gctx.set_rumble_state(0, retro_rumble_effect::RETRO_RUMBLE_STRONG, strength);
                }
                break;
            }
        }

        self.timer = 0;
        self.even = !self.even;

        unsafe {
            let (_prefix, bytes, _suffix) = self
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

    fn get_memory_data(
        &mut self,
        id: std::os::raw::c_uint,
        _ctx: &mut GetMemoryDataContext,
    ) -> *mut std::os::raw::c_void {
        if id == RETRO_MEMORY_SAVE_RAM {
            return self.game_boy.cartridge.get_ram().as_ptr() as *mut std::os::raw::c_void;
        }
        return null_mut();
    }

    fn get_memory_size(
        &mut self,
        id: std::os::raw::c_uint,
        _ctx: &mut GetMemorySizeContext,
    ) -> usize {
        if id == RETRO_MEMORY_SAVE_RAM {
            return self.game_boy.cartridge.get_ram().len();
        }
        return 0;
    }
}
