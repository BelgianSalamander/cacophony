pub type Coord = f32;
pub type Sample = f32;
pub type Seed = u32;

pub trait NoiseSource {
    fn sample(&self, x: Coord, y: Coord, seed: Seed) -> Sample;
}

pub struct TestSource;

impl NoiseSource for TestSource {
    fn sample(&self, x: Coord, y: Coord, _seed: Seed) -> Sample {
        x.cos() * 0.5 + y.cos() * 0.5
    }
}