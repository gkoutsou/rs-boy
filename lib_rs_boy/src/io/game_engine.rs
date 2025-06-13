pub trait DrawingWindow {
    fn refresh_buffer(&mut self, screen: &Vec<u32>);
    fn get_pressed_keys(&self) -> Vec<Key>;
}

#[derive(PartialEq)]
pub enum Key {
    X,
    Z,

    Down,
    Left,
    Right,
    Up,

    Enter,
    Backspace,
}
