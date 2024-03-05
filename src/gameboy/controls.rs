use log::trace;
use minifb::Key;

pub const REGISTER_LOCATION: usize = 0xff00;

pub struct Joypad {
    /// ff00
    ///
    /// 5 Select buttons
    ///
    /// 4 Select d-pad
    ///
    /// 3 Start / Down
    ///
    /// 2 Select / Up
    ///
    /// 1 B / Left
    ///
    /// 0 A / Right
    joypad: u8,

    keys: Vec<minifb::Key>,
}

impl Joypad {
    pub fn get(&self, location: usize) -> u8 {
        match location {
            REGISTER_LOCATION => {
                // If neither buttons nor d-pad is selected ($30 was written), then the low nibble
                // reads $F (all buttons released).
                if self.joypad & 0x30 == 0x30 || self.keys.is_empty() {
                    return self.joypad | 0xf;
                }

                let mut keys = 0xffu8;
                let buttons = self.buttons_selected();
                let dpad = self.dpad_selected();

                if (buttons && self.keys.contains(&Key::Enter))
                    || (dpad && self.keys.contains(&Key::Down))
                {
                    keys &= 0xf7; // 11110111
                }
                if (buttons && self.keys.contains(&Key::Backspace))
                    || (dpad && self.keys.contains(&Key::Up))
                {
                    keys &= 0xfb; // 11111011
                }
                if (buttons && self.keys.contains(&Key::X))
                    || (dpad && self.keys.contains(&Key::Left))
                {
                    keys &= 0xfd; // 11111101
                }
                if (buttons && self.keys.contains(&Key::Z))
                    || (dpad && self.keys.contains(&Key::Right))
                {
                    keys &= 0xfe; // 11111110
                }
                let result = self.joypad & keys;
                trace!(
                    "reading joypad: {:#b} - {},{}",
                    result,
                    self.buttons_selected(),
                    self.dpad_selected()
                );
                result
            }

            _ => panic!("controls location read: {:#x}", location),
        }
    }

    pub fn write(&mut self, location: usize, value: u8) {
        let value = (value & 0x30) | (self.joypad & 0x0f);
        trace!("updating joypad: {:#b}", value);
        match location {
            REGISTER_LOCATION => self.joypad = value,
            _ => {
                panic!("controls location write: {:#x}", location)
            }
        }
    }

    fn buttons_selected(&self) -> bool {
         // flipped semantics
                                                 // println!("== {}", val);
        (self.joypad & (1 << 5)) == 0
    }
    fn dpad_selected(&self) -> bool {
        // println!("joy: {:#b}\nmask: {:#b}", self.joypad, (1 << 4));
        let val = self.joypad & (1 << 4); // flipped semantics
                                            // println!("==> {}", val);
        val == 0
    }

    pub fn key_pressed(&mut self, pressed_keys: Vec<minifb::Key>) {
        self.keys = pressed_keys;
    }

    pub fn default() -> Joypad {
        Joypad {
            joypad: 0xcf,
            keys: Vec::new(),
        }
    }
}
