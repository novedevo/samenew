use std::f32::consts::PI;

pub struct SineWave {
    hz: f32,
    sample_rate: usize,
    samples: usize,
    seconds: f32,
    cycles: f32,
}

impl SineWave {
    fn new_from_sample_count(hz: f32, sample_rate: usize, samples: usize) -> Self {
        let seconds = samples as f32 / sample_rate as f32;
        let cycles = hz * seconds;
        Self {
            hz,
            sample_rate,
            samples,
            seconds,
            cycles,
        }
    }
    fn new_from_seconds(hz: f32, sample_rate: usize, seconds: f32) -> Self {
        let samples = sample_rate as f32 * seconds;
        let cycles = hz * seconds;
        Self {
            hz,
            sample_rate,
            samples: samples.floor() as usize,
            seconds,
            cycles,
        }
    }
    fn new_from_cycles(hz: f32, sample_rate: usize, cycles: f32) -> Self {
        let seconds = hz * cycles;
        let samples = sample_rate as f32 * seconds;
        Self {
            hz,
            sample_rate,
            samples: samples.floor() as usize,
            seconds,
            cycles,
        }
    }

    fn generate_samples(&self) -> Vec<f32> {
        (0..self.samples).map(|s| s as f32).map(|sample_index| {
            let way_through_cycle = sample_index / ((self.samples - 1) as f32 / self.cycles);
            (way_through_cycle * 2.0 * PI).sin()
        }).collect()
    }
}

#[cfg(test)]
mod test {
    use crate::SineWave;

    #[test]
    fn simple_sine() {
        let sine = SineWave::new_from_cycles(1.0, 50, 1.0);
        dbg!(sine.generate_samples());
    }
}