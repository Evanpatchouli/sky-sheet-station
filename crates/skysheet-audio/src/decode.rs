use crate::{Error, MAX_DURATION_SECONDS, MAX_FILE_BYTES, Observer, SAMPLE_RATE, progress};
use rubato::{FftFixedInOut, Resampler};
use std::{fs::File, path::Path};
use symphonia::core::{
    audio::SampleBuffer, codecs::DecoderOptions, errors::Error as DecodeError,
    formats::FormatOptions, io::MediaSourceStream, meta::MetadataOptions, probe::Hint,
};

pub struct Audio {
    pub samples: Vec<f32>,
    pub source_sample_rate: u32,
    pub source_channels: usize,
}

/// Decode the entire source, preserving leading silence. Corrupt packets fail
/// instead of silently removing time. Limits apply before model allocation.
pub fn decode(path: &Path, observer: &Observer<'_>) -> Result<Audio, Error> {
    progress(observer, "decode", 0, None)?;
    let file = File::open(path)
        .map_err(|e| Error::new("audio_read", format!("{}: {e}", path.display())))?;
    let metadata = file.metadata().map_err(|e| Error::new("audio_read", e))?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_FILE_BYTES {
        return Err(Error::new(
            "audio_size",
            "Audio must be a nonempty regular file of at most 128 MiB",
        ));
    }
    let stream = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(extension) = path.extension().and_then(|s| s.to_str()) {
        hint.with_extension(extension);
    }
    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            stream,
            &FormatOptions {
                enable_gapless: true,
                ..Default::default()
            },
            &MetadataOptions::default(),
        )
        .map_err(|e| Error::new("audio_format", format!("Cannot probe MP3/WAV/FLAC: {e}")))?;
    let mut format = probed.format;
    let track = format
        .default_track()
        .ok_or_else(|| Error::new("audio_track", "No audio track found"))?;
    let track_id = track.id;
    let expected_frames = track.codec_params.n_frames;
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| Error::new("audio_codec", e))?;
    let mut rate = 0;
    let mut channels = 0;
    let mut mono = Vec::new();
    let mut reported = 0;
    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(DecodeError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(Error::new("audio_packet", e)),
        };
        if packet.track_id() != track_id {
            continue;
        }
        let decoded = decoder
            .decode(&packet)
            .map_err(|e| Error::new("audio_decode", e))?;
        let spec = *decoded.spec();
        if rate == 0 {
            rate = spec.rate;
            channels = spec.channels.count();
            if !(8_000..=192_000).contains(&rate) || !(1..=8).contains(&channels) {
                return Err(Error::new(
                    "audio_parameters",
                    "Supported sample rates: 8000..192000 Hz; channels: 1..8",
                ));
            }
        } else if spec.rate != rate || spec.channels.count() != channels {
            return Err(Error::new(
                "audio_parameters",
                "Sample rate/channel count changed within the audio stream",
            ));
        }
        if mono.len() + decoded.frames() > rate as usize * MAX_DURATION_SECONDS as usize {
            return Err(Error::new(
                "audio_duration",
                "Audio exceeds the 600 second transcription limit; split the source explicitly",
            ));
        }
        let mut samples = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
        samples.copy_interleaved_ref(decoded);
        for frame in samples.samples().chunks_exact(channels) {
            if frame.iter().any(|sample| !sample.is_finite()) {
                return Err(Error::new(
                    "audio_sample",
                    "Audio contains NaN or infinite samples",
                ));
            }
            mono.push(frame.iter().map(|v| f64::from(*v)).sum::<f64>() as f32 / channels as f32);
        }
        if mono.len().saturating_sub(reported) >= rate as usize / 2 {
            progress(observer, "decode", mono.len() as u64, expected_frames)?;
            reported = mono.len();
        }
    }
    if mono.is_empty() {
        return Err(Error::new("audio_empty", "No decodable audio samples"));
    }
    // A declared length substantially longer than decoded PCM usually signals a
    // truncated file. Allow one codec packet plus gapless delay/padding metadata.
    if expected_frames.is_some_and(|expected| expected > mono.len() as u64 + 4096) {
        return Err(Error::new(
            "audio_truncated",
            "Decoded audio ends before its declared frame count",
        ));
    }
    progress(
        observer,
        "decode",
        mono.len() as u64,
        Some(mono.len() as u64),
    )?;
    let samples = resample(&mono, rate, observer)?;
    Ok(Audio {
        samples,
        source_sample_rate: rate,
        source_channels: channels,
    })
}

fn resample(input: &[f32], rate: u32, observer: &Observer<'_>) -> Result<Vec<f32>, Error> {
    progress(observer, "resample", 0, Some(input.len() as u64))?;
    if rate == SAMPLE_RATE {
        return Ok(input.to_vec());
    }
    let mut resampler = FftFixedInOut::<f32>::new(rate as usize, SAMPLE_RATE as usize, 2048, 1)
        .map_err(|e| Error::new("audio_resample", e))?;
    let chunk = resampler.input_frames_next();
    let delay = resampler.output_delay();
    let target = (input.len() as u64 * u64::from(SAMPLE_RATE)).div_ceil(u64::from(rate)) as usize;
    let mut output = Vec::with_capacity(target + delay + resampler.output_frames_max());
    let mut buffer = vec![0.0; chunk];
    let mut offset = 0;
    while output.len() < target + delay {
        buffer.fill(0.0);
        let count = chunk.min(input.len().saturating_sub(offset));
        buffer[..count].copy_from_slice(&input[offset..offset + count]);
        offset += count;
        let result = resampler
            .process(&[&buffer], None)
            .map_err(|e| Error::new("audio_resample", e))?;
        output.extend_from_slice(&result[0]);
        if offset % (chunk * 32) == 0 || offset == input.len() {
            progress(
                observer,
                "resample",
                offset as u64,
                Some(input.len() as u64),
            )?;
        }
    }
    Ok(output[delay..delay + target].to_vec())
}
