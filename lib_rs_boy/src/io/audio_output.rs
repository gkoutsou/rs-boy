pub trait AudioTarget {
    fn play(&mut self, left: f32, right: f32);
    fn start(&self);
}
