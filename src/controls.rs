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
                println!(
                    "reading joypad: {:#b} - {},{}",
                    self.joypad,
                    self.buttons_selected(),
                    self.dpad_selected()
                );
                // If neither buttons nor d-pad is selected ($30 was written), then the low nibble
                // reads $F (all buttons released).
                if self.joypad & 0x30 == 0x30 {
                    self.joypad | 0xf
                } else {
                    self.joypad
                }
            }
            _ => panic!("controls location read: {:#x}", location),
        }
    }

    pub fn write(&mut self, location: usize, value: u8) {
        let value = (value & 0x30) | (self.joypad & 0x0f);
        println!("updating joypad: {:#b}", value);
        match location {
            REGISTER_LOCATION => self.joypad = value,
            _ => {
                panic!("controls location write: {:#x}", location)
            }
        }
    }

    fn buttons_selected(&self) -> bool {
        let val = (self.joypad & (1 << 5)) == 0; // flipped semantics
                                                 // println!("== {}", val);
        val
    }
    fn dpad_selected(&self) -> bool {
        // println!("joy: {:#b}\nmask: {:#b}", self.joypad, (1 << 4));
        let val = (self.joypad & (1 << 4)); // flipped semantics
                                            // println!("==> {}", val);
        val == 0
    }

    pub fn key_pressed(&mut self, pressed_keys: Vec<minifb::Key>) {
        self.keys = pressed_keys;
        // if pressed_keys.len() == 0 {
        //     self.joypad |= 0b00001111;
        //     return;
        // }
        // let buttons = self.buttons_selected();
        // let dpad = self.dpad_selected();
        // // println!("buttons: {} dpad: {}", buttons, dpad);

        // pressed_keys.iter().for_each(|key| match key {
        //     Key::Right if dpad => {
        //         println!("pressed right");
        //         self.joypad &= !(1u8 << 0)
        //     }
        //     Key::Z if buttons => {
        //         println!("pressed Z (a)");
        //         self.joypad &= !(1u8 << 0)
        //     } // A

        //     Key::Left if dpad => {
        //         println!("pressed left");
        //         self.joypad &= !(1u8 << 1)
        //     }
        //     Key::X if buttons => {
        //         println!("pressed X (b)");
        //         self.joypad &= !(1u8 << 1)
        //     } // B

        //     Key::Up if dpad => {
        //         println!("pressed up");
        //         self.joypad &= !(1u8 << 2)
        //     }
        //     Key::Backspace if buttons => {
        //         println!("pressed backspace (select)");
        //         self.joypad &= !(1u8 << 2)
        //     } // Select

        //     Key::Down if dpad => {
        //         println!("pressed down");
        //         self.joypad &= !(1u8 << 3)
        //     }
        //     Key::Enter if buttons => {
        //         println!("pressed enter (start)");
        //         self.joypad &= !(1u8 << 3)
        //     } // Start
        //     _a => {
        //         (panic!(
        //             "{:#?} - {} {}",
        //             _a,
        //             self.buttons_selected(),
        //             self.dpad_selected()
        //         ))
        //     } // _ => (),
        // });
    }

    pub fn default() -> Joypad {
        Joypad {
            joypad: 0x3f,
            keys: Vec::new(),
        }
    }
}
