use std::{array, f32::consts::PI};

use chrono::{DateTime, Utc};

pub struct EasWarning {
    header: Header,
    attention_signal: AttentionSignal,
}

impl EasWarning {
    /// `attsig_combined` is true for a combined tone and false for a single tone
    pub fn new(header: Header, attsig_combined: bool) -> Self {
        let attention_signal = if !attsig_combined {
            AttentionSignal::single(9.0)
        } else {
            AttentionSignal::combined(9.0)
        }
        .unwrap();

        Self {
            header,
            attention_signal,
        }
    }
    pub fn construct(
        &self,
        sample_rate: usize,
        message: Option<Vec<f32>>,
        critical: bool,
    ) -> Vec<f32> {
        use Section::*;
        let mut sections = vec![];

        let header = self.header.render();
        let eom = tail().to_vec();

        sections.push(AfskBytes(header.clone()));
        sections.push(Silence(1.0));
        sections.push(AfskBytes(header.clone()));
        sections.push(Silence(1.0));
        sections.push(AfskBytes(header));

        if let Some(message) = message {
            if critical {
                sections.push(Silence(2.0));
                sections.push(Tone(self.attention_signal.into()));
            }
            sections.push(Silence(4.0));
            sections.push(Audio(message));
        }

        sections.push(Silence(2.0));
        sections.push(AfskBytes(eom.clone()));
        sections.push(Silence(1.0));
        sections.push(AfskBytes(eom.clone()));
        sections.push(Silence(1.0));
        sections.push(AfskBytes(eom));

        Self::render(&sections, sample_rate)
    }

    fn render(sections: &[Section], sample_rate: usize) -> Vec<f32> {
        sections
            .iter()
            .flat_map(|section| section.render(sample_rate))
            .collect()
    }
}

#[derive(bon::Builder)]
pub struct Header {
    originator_code: OriginatorCode,
    event_code: [u8; 3],
    /// In Canada, these are Canadian Location Codes (CLC). In the US, a specific format is followed (PSSCCC)
    ///
    /// Maximum of 31 codes per message.
    #[builder(with = |codes: Vec<[u8; 6]>| -> Result<_, ()> {
        if codes.len() <= 31 {
            Ok(codes)
        } else {
            Err(())
        }
    })]
    location_codes: Vec<[u8; 6]>,
    purge_time: [u8; 4],
    time_of_issue: DateTime<Utc>,
    /// Must be 8 characters long.
    /// If your callsign (e.g. `WDAF/FM`) is shorter than 8 characters, add spaces or slashes to the end.
    /// So, `WDAF/FM ` or `WDAF/FM/`
    ///
    // note: I chose to request padding instead of using a variable-length
    // field because all the real-world examples I decoded do this.
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

#[derive(Clone, Copy, Debug)]
pub enum OriginatorCode {
    /// National Public Warning System (f.k.a. Primary Entry Point System).
    ///
    /// Authorized national officials such as the President or Prime Minister
    Pep,
    /// Civil Authorities
    ///
    /// State / provincial governments, municipal police / fire
    Civ,
    /// National Weather Service / Environment Canada
    ///
    /// General weather use
    Wxr,
    /// EAS Participant
    ///
    /// Broadcasters, usually test messages
    Eas,
    /// Emergency Action Notification Network
    #[deprecated = "No longer used since 2010. Use Pep instead."]
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
            #[allow(
                deprecated,
                reason = "Deprecated location code still needs to be implemented."
            )]
            OriginatorCode::Ean => b"EAN",
        }
        .map(|byte| byte.into())
    }
}

#[derive(Copy, Clone, Debug)]
enum AttentionSignal {
    /// used by NOAA Weather Radio, and Canadian weather radio only with event codes RMT, SVR, and TOR.
    SingleTone(f32),
    /// used for broadcast radio / TV; all others
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
                    let sinewave: MultiSineWave = afsk_bit.into();
                    sinewave.generate_samples(sample_rate)
                })
                .collect(),
        }
    }
}

fn preamble() -> [AfskByte; 16] {
    [0xAB.into(); 16]
}

fn tail() -> [AfskByte; 20] {
    let mut tail = [AfskByte::default(); 20];
    tail[0..16].copy_from_slice(&preamble());
    tail[16..].copy_from_slice(&b"NNNN".map(|byte| byte.into()));
    tail
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

impl From<AfskBit> for MultiSineWave {
    fn from(bit: AfskBit) -> Self {
        let cycles = match bit {
            AfskBit::Mark => 4.0,
            AfskBit::Space => 3.0,
        };
        Self::single_from_cycles_and_seconds(cycles, 1.92 / 1000.0)
    }
}
struct MultiSineWave {
    seconds: f32,
    frequencies: Vec<f32>,
}

impl MultiSineWave {
    fn single_from_cycles_and_seconds(cycles: f32, seconds: f32) -> Self {
        let frequency = cycles / seconds;
        Self {
            seconds,
            frequencies: vec![frequency],
        }
    }
    fn generate_samples(&self, sample_rate: usize) -> Vec<f32> {
        let samples = (sample_rate as f32 * self.seconds).floor() as usize + 1;
        // in the basic scenario where there is only one sine wave and it is exactly one cycle,
        // the first sample will be zero and the last sample will be right before zero
        // i.e. the next sample is assumed to be the final sample
        let oversampled = (0..samples)
            .map(|sample_index| {
                self.frequencies
                    .iter()
                    .map(|frequency| {
                        let cycles = frequency * self.seconds;
                        let way_through_cycle =
                            sample_index as f32 / ((samples - 1) as f32 / cycles);
                        (way_through_cycle * 2.0 * PI).sin()
                    })
                    .collect::<Vec<f32>>()
            })
            .map(|samples| samples.iter().sum::<f32>() / samples.len() as f32)
            .collect::<Vec<f32>>();
        oversampled[..samples - 1].into()
    }
}

#[cfg(test)]
mod test {
    use chrono::Utc;
    use hound::{WavSpec, WavWriter};

    use crate::{EasWarning, Header, MultiSineWave, OriginatorCode};

    #[test]
    fn simple_sine() {
        let sine = MultiSineWave::single_from_cycles_and_seconds(1.0, 1.0);
        dbg!(sine.generate_samples(50));
    }

    #[test]
    fn e2e() {
        generate_eas();
    }

    #[test]
    #[ignore]
    fn output_wav() {
        let eas = generate_eas();
        let spec = WavSpec {
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = WavWriter::create("data/output.wav", spec).unwrap();
        for sample in eas {
            writer.write_sample(sample).unwrap();
        }
        writer.finalize().unwrap()
    }

    fn generate_eas() -> Vec<f32> {
        let sample_rate = 44_100;
        let placeholder_message = MultiSineWave {
            seconds: 5.0,
            frequencies: vec![440.0],
        }
        .generate_samples(sample_rate);

        let header = Header::builder()
            .time_of_issue(Utc::now())
            .event_code(*b"IFW")
            .purge_time(*b"0015")
            .callsign(*b"EC/GC/CA")
            .location_codes(vec![*b"082620"])
            .unwrap()
            .originator_code(OriginatorCode::Civ)
            .build();
        let warning = EasWarning::new(header, true);
        return warning.construct(sample_rate, Some(placeholder_message), true);
    }
}
