# samenew

Experimental Rust implementation of the EAS / SAME digital radio alert encoding system. This crate focuses on correctness and configurability. It is intended to let you programmatically generate raw (`Vec<f32>`) audio warnings.

### Example
```rs
use chrono::Utc;
use samenew::{EasWarning, Header, OriginatorCode};

let sample_rate = 44_100;
let placeholder_audio: Vec<f32> = vec![]; // add your mono, f32 audio message here (be sure to match sample rate)
let header = Header::builder()
    .time_of_issue(Utc::now())
    .event_code(*b"IFW")
    .purge_time(*b"0015")
    .callsign(*b"EC/GC/CA")
    .location_codes(vec![*b"082620"])
    .unwrap()
    .originator_code(OriginatorCode::Civ)
    .build();
let warning = EasWarning::new(header, 8.0, true).unwrap();
let audio: Vec<f32> = warning.construct(sample_rate, Some(placeholder_message), true);
```

### Disclaimers
This software may contain bugs, has not been formally verified, and is **not** intended for safety-critical applications. There may be regulations restricting your use of the output of this software. Please do not use this to transmit false emergency messages.

### Thanks
Inspired by the crate `sameold`, which implements a SAME decoder. Reference SAME specification taken from [NOAA](https://web.archive.org/web/20240224093737/http://www.nws.noaa.gov/directives/sym/pd01017012curr.pdf), please consult this source to use this software properly.
