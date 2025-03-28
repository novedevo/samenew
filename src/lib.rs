use std::{array, f32::consts::PI};

use chrono::{DateTime, Utc};

struct SineWave {
    seconds: f32,
    cycles: f32,
}

impl SineWave {
    fn new_from_cycles_and_seconds(cycles: f32, seconds: f32) -> Self {
        Self { seconds, cycles }
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

struct MultiSineWave {
    seconds: f32,
    frequencies: Vec<f32>,
}

impl MultiSineWave {
    fn generate_samples(&self, sample_rate: usize) -> Vec<f32> {
        let samples = (sample_rate as f32 * self.seconds).floor() as usize;
        (0..samples)
            .map(|sample_index| {
                self.frequencies
                    .iter()
                    .map(|frequency| {
                        let cycles = frequency * self.seconds;
                        let way_through_cycle =
                            sample_index as f32 / ((samples - 1) as f32 / cycles);
                        (way_through_cycle * 2.0 * PI).sin()
                    })
                    .collect::<Vec<_>>()
            })
            .map(|samples| samples.iter().sum::<f32>() / samples.len() as f32)
            .collect()
    }
}

#[derive(Clone, Copy, Debug, Default)]
enum AfskBit {
    Mark,
    #[default]
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

#[derive(Clone, Copy, Debug, Default)]
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
pub enum OriginatorCode {
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

fn preamble() -> [AfskByte; 16] {
    [0xAB.into(); 16]
}

#[derive(bon::Builder)]
pub struct Header {
    originator_code: OriginatorCode,
    event_code: [u8; 3],
    location_codes: Vec<[u8; 6]>,
    purge_time: [u8; 4],
    time_of_issue: DateTime<Utc>,
    callsign: [u8; 8],
}

impl Header {
    fn render(&self) -> Vec<AfskByte> {
        let formatted_datetime = self.time_of_issue.format("%j%H%M").to_string();
        let stripped_callsign = self
            .callsign
            .map(|char| if char == b'-' { b'\\' } else { char });

        let mut header = vec![preamble().to_vec()];

        header.push(b"ZCZC-".map(|byte| byte.into()).to_vec());
        header.push(self.originator_code.to_afsk_bytes().to_vec());
        header.push(b"-".map(|byte| byte.into()).to_vec());
        header.push(self.event_code.map(|byte| byte.into()).to_vec());
        for location_code in &self.location_codes {
            header.push(b"-".map(|byte| byte.into()).to_vec());
            header.push(location_code.map(|byte| byte.into()).to_vec());
        }
        header.push(b"+".map(|byte| byte.into()).to_vec());
        header.push(self.purge_time.map(|byte| byte.into()).to_vec());
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
}

fn tail() -> [AfskByte; 20] {
    let mut tail = [AfskByte::default(); 20];
    tail[0..16].copy_from_slice(&preamble());
    tail[16..].copy_from_slice(&b"NNNN".map(|byte| byte.into()));
    tail
}

#[derive(Copy, Clone, Debug)]
enum AttentionSignal {
    SingleTone(f32),
    CombinedTone(f32),
}

impl AttentionSignal {
    pub fn single(seconds: f32) -> Option<AttentionSignal> {
        if seconds < 8.0 {
            None
        } else {
            Some(Self::SingleTone(seconds))
        }
    }
    pub fn combined(seconds: f32) -> Option<AttentionSignal> {
        if seconds < 8.0 {
            None
        } else {
            Some(Self::CombinedTone(seconds))
        }
    }
}

impl From<AttentionSignal> for MultiSineWave {
    fn from(attsig: AttentionSignal) -> Self {
        let (seconds, frequencies) = match attsig {
            AttentionSignal::SingleTone(secs) => (secs, vec![1050.0]),
            AttentionSignal::CombinedTone(secs) => (secs, vec![853.0, 960.0]),
        };
        Self {
            seconds,
            frequencies,
        }
    }
}

enum Section {
    AfskBytes(Vec<AfskByte>),
    Silence(f32),
    Tone(MultiSineWave),
    Audio(Vec<f32>),
}

impl Section {
    fn render(&self, sample_rate: usize) -> Vec<f32> {
        match self {
            Self::Silence(seconds) => vec![0.0; (sample_rate as f32 * seconds).floor() as usize],
            Self::Audio(audio) => audio.clone(),
            Self::Tone(msw) => msw.generate_samples(sample_rate),
            Self::AfskBytes(afsk_bytes) => afsk_bytes
                .iter()
                .flat_map(|afsk_byte| afsk_byte.bits)
                .flat_map(|afsk_bit| {
                    let sinewave: SineWave = afsk_bit.into();
                    sinewave.generate_samples(sample_rate)
                })
                .collect(),
        }
    }
}

pub struct EasWarning {
    header: Header,
    attention_signal: AttentionSignal,
}

impl EasWarning {
    ///`attsig_secs` must be at least `8.0`, `attsig_combined` is true for a combined tone and false for a single tone
    pub fn new(header: Header, attsig_secs: f32, attsig_combined: bool) -> Option<Self> {
        let attention_signal = if !attsig_combined {
            AttentionSignal::single(attsig_secs)
        } else {
            AttentionSignal::combined(attsig_secs)
        }?;

        Some(Self {
            header,
            attention_signal,
        })
    }
    pub fn construct(&self, sample_rate: usize, message: Vec<f32>) -> Vec<f32> {
        use Section::*;
        let mut sections = vec![];

        let header = self.header.render();
        let eom = tail().to_vec();

        sections.push(AfskBytes(header.clone()));
        sections.push(Silence(1.0));
        sections.push(AfskBytes(header.clone()));
        sections.push(Silence(1.0));
        sections.push(AfskBytes(header));
        sections.push(Silence(1.0));

        sections.push(Tone(self.attention_signal.into()));
        sections.push(Silence(1.0));

        sections.push(Audio(message));
        sections.push(Silence(1.0));

        sections.push(AfskBytes(eom.clone()));
        sections.push(Silence(1.0));
        sections.push(AfskBytes(eom.clone()));
        sections.push(Silence(1.0));
        sections.push(AfskBytes(eom));
        sections.push(Silence(1.0));

        Self::render(&sections, sample_rate)
    }

    fn render(sections: &[Section], sample_rate: usize) -> Vec<f32> {
        sections
            .iter()
            .flat_map(|section| section.render(sample_rate))
            .collect()
    }
}

#[cfg(test)]
mod test {
    use crate::SineWave;

    #[test]
    fn simple_sine() {
        let sine = SineWave::new_from_cycles_and_seconds(1.0, 1.0);
        dbg!(sine.generate_samples(50));
    }
}
