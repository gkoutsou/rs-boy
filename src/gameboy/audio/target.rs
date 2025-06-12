pub struct FakeSpeaker {}
impl super::super::super::io::audio_output::AudioTarget for FakeSpeaker {
    fn play(&mut self, _: f32, _: f32) {}

    fn start(&self) {}
}
