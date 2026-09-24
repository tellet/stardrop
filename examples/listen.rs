//! Design tool: writes every sound in the game to `target/sounds/` as a WAV,
//! so they can be listened to without playing.
//!
//! `cargo run --release --example listen`
#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a command-line tool: printing is the point"
)]

use std::fs;
use std::io::Write as _;
use std::path::Path;

use stardrop::music::{Key, key_for};
use stardrop::synth::{Bed, SAMPLE_RATE, Sound, bed, render};

/// Writes mono samples as a 16-bit WAV, the format everything can play.
fn write_wav(path: &Path, samples: &[f32]) -> std::io::Result<()> {
    let data: Vec<u8> = samples
        .iter()
        .flat_map(|sample| {
            let clamped = sample.clamp(-1.0, 1.0);
            // 16-bit signed, the usual range.
            #[expect(clippy::cast_possible_truncation, reason = "clamped above")]
            let value = (clamped * f32::from(i16::MAX)) as i16;
            value.to_le_bytes()
        })
        .collect();
    let mut file = fs::File::create(path)?;
    let data_len = u32::try_from(data.len()).unwrap_or(u32::MAX);
    let byte_rate = SAMPLE_RATE * 2;
    file.write_all(b"RIFF")?;
    file.write_all(&(36 + data_len).to_le_bytes())?;
    file.write_all(b"WAVEfmt ")?;
    file.write_all(&16_u32.to_le_bytes())?; // header length
    file.write_all(&1_u16.to_le_bytes())?; // uncompressed
    file.write_all(&1_u16.to_le_bytes())?; // one channel
    file.write_all(&SAMPLE_RATE.to_le_bytes())?;
    file.write_all(&byte_rate.to_le_bytes())?;
    file.write_all(&2_u16.to_le_bytes())?; // bytes per frame
    file.write_all(&16_u16.to_le_bytes())?; // bits per sample
    file.write_all(b"data")?;
    file.write_all(&data_len.to_le_bytes())?;
    file.write_all(&data)
}

fn main() {
    let folder = Path::new("target/sounds");
    if let Err(error) = fs::create_dir_all(folder) {
        eprintln!("{}: {error}", folder.display());
        return;
    }
    let mut written = 0;
    for sound in Sound::ALL {
        let name = format!("{sound:?}").to_lowercase();
        let samples = render(sound);
        match write_wav(&folder.join(format!("{name}.wav")), &samples) {
            Ok(()) => written += 1,
            Err(error) => eprintln!("{name}: {error}"),
        }
    }
    // The music bed of the first and last altitudes, and how a pause sounds.
    for (label, theme) in [
        ("top", stardrop::level::Theme::DeepSpace),
        ("bottom", stardrop::level::Theme::Homecoming),
    ] {
        let key: Key = key_for(theme);
        let air = if label == "top" { 0.0 } else { 1.0 };
        for (part, name) in [
            (Bed::Pad, "pad"),
            (Bed::Deeper, "deeper"),
            (Bed::Ambience, "ambience"),
        ] {
            let samples = bed(part, key.root_hz, air, false);
            let file = folder.join(format!("bed-{label}-{name}.wav"));
            if let Err(error) = write_wav(&file, &samples) {
                eprintln!("{}: {error}", file.display());
            } else {
                written += 1;
            }
        }
        let paused = bed(Bed::Pad, key.root_hz, air, true);
        if write_wav(&folder.join(format!("bed-{label}-pad-paused.wav")), &paused).is_ok() {
            written += 1;
        }
        println!("{label}: {}", key.name);
    }
    println!("{written} sounds written to {}", folder.display());
}
