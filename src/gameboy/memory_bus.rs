pub trait MemoryAccessor {
    fn get(&self, location: usize) -> u8;
    fn write(&mut self, location: usize, value: u8);
}
