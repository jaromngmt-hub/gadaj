//! Resampling audio do 16kHz mono dla parakeet.cpp.
//!
//! MVP: linear interpolation - szybkie i wystarczające dla STT.
//! W produkcji warto użyć `rubato` (Sinc) dla lepszej jakości.

pub fn resample_linear(input: &[f32], src_rate: u32, dst_rate: u32) -> Vec<f32> {
    if src_rate == dst_rate {
        return input.to_vec();
    }
    let ratio = dst_rate as f64 / src_rate as f64;
    let new_len = (input.len() as f64 * ratio).ceil() as usize;
    let mut out = Vec::with_capacity(new_len);

    for i in 0..new_len {
        let src_idx_f = i as f64 / ratio;
        let idx0 = src_idx_f.floor() as usize;
        let idx1 = (idx0 + 1).min(input.len() - 1);
        let t = src_idx_f - idx0 as f64;
        let s0 = input[idx0] as f64;
        let s1 = input[idx1] as f64;
        out.push((s0 + (s1 - s0) * t) as f32);
    }
    out
}

/// Resampling wysokiej jakości (Sinc) z użyciem rubato.
pub fn resample_sinc(
    input: &[f32],
    src_rate: u32,
    dst_rate: u32,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    use rubato::{FftFixedIn, Resampler};

    if src_rate == dst_rate {
        return Ok(input.to_vec());
    }

    let chunk_size = 1024;
    let mut resampler = FftFixedIn::<f32>::new(
        src_rate as usize,
        dst_rate as usize,
        chunk_size,
        1, // mono
        1, // sub_chunks
    )?;

    let mut output = Vec::with_capacity((input.len() as f64 * dst_rate as f64 / src_rate as f64) as usize);
    let mut pos = 0;
    while pos + chunk_size <= input.len() {
        let waves: Vec<Vec<f32>> = vec![input[pos..pos + chunk_size].to_vec()];
        let resampled = resampler.process(&waves, None)?;
        output.extend_from_slice(&resampled[0]);
        pos += chunk_size;
    }

    // Ostatni kawałek
    if pos < input.len() {
        let mut last = input[pos..].to_vec();
        last.resize(chunk_size, 0.0);
        let waves: Vec<Vec<f32>> = vec![last];
        let resampled = resampler.process(&waves, None)?;
        let valid = (input.len() - pos) * dst_rate as usize / src_rate as usize;
        output.extend_from_slice(&resampled[0][..valid.min(resampled[0].len())]);
    }

    Ok(output)
}
