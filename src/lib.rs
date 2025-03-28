use std::{array, f32::consts::PI};

use chrono::{DateTime, Utc};

pub struct SineWave {
    hz: f32,
    // sample_rate: usize,
    // samples: usize,
    seconds: f32,
    cycles: f32,
}

impl SineWave {
    // fn new_from_sample_count(hz: f32, sample_rate: usize, samples: usize) -> Self {
    //     let seconds = samples as f32 / sample_rate as f32;
    //     let cycles = hz * seconds;
    //     Self {
    //         hz,
    //         sample_rate,
    //         samples,
    //         seconds,
    //         cycles,
    //     }
    // }
    fn new_from_seconds(hz: f32, seconds: f32) -> Self {
        // let samples = sample_rate as f32 * seconds;
        let cycles = hz * seconds;
        Self {
            hz,
            // sample_rate,
            // samples: samples.floor() as usize,
            seconds,
            cycles,
        }
    }
    fn new_from_cycles(hz: f32, cycles: f32) -> Self {
        let seconds = hz * cycles;
        // let samples = sample_rate as f32 * seconds;
        Self {
            hz,
            // sample_rate,
            // samples: samples.floor() as usize,
            seconds,
            cycles,
        }
    }
    fn new_from_cycles_and_seconds(cycles: f32, seconds: f32) -> Self {
        let hz = cycles / seconds;
        Self {
            hz,
            seconds,
            cycles,
        }
    }

    fn generate_samples(&self, sample_rate: usize) -> Vec<f32> {
        let samples = (sample_rate as f32 * self.seconds).floor() as usize;
        (0..samples)
            .map(|sample_index| {
                let way_through_cycle = sample_index as f32 / ((samples - 1) as f32 / self.cycles);
                (way_through_cycle * 2.0 * PI).sin()
            })
            .collect()
    }
}

#[derive(Clone, Copy, Debug)]
enum AfskBit {
    Mark,
    Space,
}

impl From<bool> for AfskBit {
    fn from(bl: bool) -> Self {
        if bl { Self::Mark } else { Self::Space }
    }
}

impl From<AfskBit> for SineWave {
    fn from(bit: AfskBit) -> Self {
        let cycles = match bit {
            AfskBit::Mark => 4.0,
            AfskBit::Space => 3.0,
        };
        Self::new_from_cycles_and_seconds(cycles, 1.92 / 1000.0)
    }
}

#[derive(Clone, Copy, Debug)]
struct AfskByte {
    bits: [AfskBit; 8],
}

impl From<u8> for AfskByte {
    fn from(byte: u8) -> Self {
        AfskByte {
            bits: array::from_fn(|i| ((byte >> i) & 1 == 1).into()),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum OriginatorCode {
    Pep,
    Civ,
    Wxr,
    Eas,
    Ean,
}

impl OriginatorCode {
    fn to_afsk_bytes(self) -> [AfskByte; 3] {
        self.into()
    }
}

impl From<OriginatorCode> for [AfskByte; 3] {
    fn from(org: OriginatorCode) -> Self {
        match org {
            OriginatorCode::Pep => b"PEP",
            OriginatorCode::Civ => b"CIV",
            OriginatorCode::Wxr => b"WXR",
            OriginatorCode::Eas => b"WAS",
            OriginatorCode::Ean => b"EAN",
        }
        .map(|byte| byte.into())
    }
}

fn calibrator() -> [AfskByte; 16] {
    [0xAB.into(); 16]
}

fn header(
    originator_code: OriginatorCode,
    event_code: &[u8; 3],
    location_codes: Vec<[u8; 6]>,
    purge_time: [u8; 4],
    time_of_issue: DateTime<Utc>,
    callsign: [u8; 8],
) -> Vec<AfskByte> {
    let formatted_datetime = time_of_issue.format("%j%H%M").to_string();
    let stripped_callsign = callsign.map(|char| if char == b'-' {b'\\'} else {char});

    let mut header = vec![calibrator().to_vec()];

    header.push(b"ZCZC-".map(|byte| byte.into()).to_vec());
    header.push(originator_code.to_afsk_bytes().to_vec());
    header.push(b"-".map(|byte| byte.into()).to_vec());
    header.push(event_code.map(|byte| byte.into()).to_vec());
    for location_code in location_codes {
        header.push(b"-".map(|byte| byte.into()).to_vec());
        header.push(location_code.map(|byte| byte.into()).to_vec());
    }
    header.push(b"+".map(|byte| byte.into()).to_vec());
    header.push(purge_time.map(|byte| byte.into()).to_vec());
    header.push(b"-".map(|byte| byte.into()).to_vec());
    header.push(
        formatted_datetime
            .as_bytes()
            .iter()
            .cloned()
            .map(|b| b.into())
            .collect(),
    );
    header.push(b"-".map(|byte| byte.into()).to_vec());
    header.push(stripped_callsign.map(|byte| byte.into()).to_vec());
    header.push(b"-".map(|byte| byte.into()).to_vec());

    header.concat()
}


#[cfg(test)]
mod test {
    use crate::SineWave;

    #[test]
    fn simple_sine() {
        let sine = SineWave::new_from_cycles(1.0, 1.0);
        dbg!(sine.generate_samples(50));
    }
}
