use crate::io::game_engine::Key;

pub const WIDTH: usize = 160;
pub const HEIGHT: usize = 144;

pub struct FakeScreen {}
impl super::super::super::io::game_engine::DrawingWindow for FakeScreen {
    fn refresh_buffer(&mut self, _screen: &Vec<u32>) {}

    fn get_pressed_keys(&self) -> Vec<Key> {
        return vec![];
    }
}
