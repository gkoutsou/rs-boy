use minifb::{Key, Window};

pub const WIDTH: usize = 160;
pub const HEIGHT: usize = 144;

pub(crate) trait DrawingWindow {
    fn refresh_buffer(&mut self, screen: &Vec<u32>);
    fn get_pressed_keys(&self) -> Vec<minifb::Key>;
}

pub(crate) struct Screen {
    window: Window,
}

impl DrawingWindow for Screen {
    fn refresh_buffer(&mut self, screen: &Vec<u32>) {
        if self.window.is_open() && !self.window.is_key_down(Key::Escape) {
            self.window
                .update_with_buffer(&screen, WIDTH, HEIGHT)
                .unwrap();
        } else {
            panic!("window deado")
        }
    }

    fn get_pressed_keys(&self) -> Vec<minifb::Key> {
        self.window.get_keys()
    }
}

impl Screen {
    pub fn new() -> Self {
        let window_opts = minifb::WindowOptions {
            scale: minifb::Scale::X2,
            ..Default::default()
        };

        let mut window = Window::new("Test - ESC to exit", WIDTH, HEIGHT, window_opts)
            .unwrap_or_else(|e| {
                panic!("{}", e);
            });

        // Limit to max ~60 fps update rate
        window.limit_update_rate(Some(std::time::Duration::from_micros(16666)));
        // window.limit_update_rate(None);

        Screen { window: window }
    }
}
