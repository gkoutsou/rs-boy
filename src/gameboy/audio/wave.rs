pub trait Wave {
    fn step(&mut self, step: u32);
    fn sample(&self) -> f32;
    fn is_enabled(&self) -> bool;
}
