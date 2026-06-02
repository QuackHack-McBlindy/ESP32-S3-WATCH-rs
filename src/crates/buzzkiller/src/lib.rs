#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use alloc::vec;
use libm::{sqrtf, atan2f, cosf, sinf};

// Include the precomputed tables from the build script
include!(concat!(env!("OUT_DIR"), "/tables.rs"));

// ─────────────────────────────────────────────────────────────
// 512‑point real FFT / IFFT (hand‑written, no dependencies)
// ─────────────────────────────────────────────────────────────

const N: usize = 512;

// Bit‑reversed indices for 512
const BIT_REV: [usize; N] = {
    let mut br = [0usize; N];
    let mut i = 0;
    while i < N {
        br[i] = (i as usize).reverse_bits() >> 23;
        i += 1;
    }
    br
};

/// In‑place complex FFT (1024 floats = 512 complex numbers).
/// `twiddle` should be the forward or inverse twiddle factors.
fn complex_fft_512(data: &mut [f32; N * 2], twiddle: &[(f32, f32); N / 2]) {
    // Bit‑reversal permutation
    for i in 0..N {
        let j = BIT_REV[i];
        if i < j {
            let ri = data[2 * i];
            let ii = data[2 * i + 1];
            let rj = data[2 * j];
            let ij = data[2 * j + 1];
            data[2 * i] = rj;
            data[2 * i + 1] = ij;
            data[2 * j] = ri;
            data[2 * j + 1] = ii;
        }
    }

    // Cooley–Tukey
    let mut len = 2;
    while len <= N {
        let half = len / 2;
        let step = N / len;
        for group in (0..N).step_by(len) {
            for pair in 0..half {
                let idx1 = group + pair;
                let idx2 = idx1 + half;
                let t = twiddle[pair * step];

                let re1 = data[2 * idx1];
                let im1 = data[2 * idx1 + 1];
                let re2 = data[2 * idx2];
                let im2 = data[2 * idx2 + 1];

                let tr = re2 * t.0 - im2 * t.1;
                let ti = re2 * t.1 + im2 * t.0;

                data[2 * idx1] = re1 + tr;
                data[2 * idx1 + 1] = im1 + ti;
                data[2 * idx2] = re1 - tr;
                data[2 * idx2 + 1] = im1 - ti;
            }
        }
        len <<= 1;
    }
}

/// Forward real FFT: 512 real samples → 257 complex bins (interleaved re,im).
/// Output `spectrum` is a `[f32; N + 2]` → length 514, indices:
///   [re0, im0, re1, im1, …, re255, im255, re256, im256]
/// Note that im0 and im256 are always zero.
fn real_fft_512(input: &[f32; N], spectrum: &mut [f32; N + 2]) {
    let mut data = [0.0f32; N * 2];
    for i in 0..N {
        data[2 * i] = input[i];
        // imaginary part left 0.0
    }

    complex_fft_512(&mut data, &TWIDDLE_FWD);

    // Unscramble to compact real‑FFT format
    for k in 0..=N / 2 {
        spectrum[2 * k] = data[2 * k];
        spectrum[2 * k + 1] = data[2 * k + 1];
    }
}

/// Inverse real FFT: 257 complex bins → 512 real samples.
/// The input `spectrum` has the same layout as the output of `real_fft_512`.
fn real_ifft_512(spectrum: &[f32; N + 2], output: &mut [f32; N]) {
    let mut data = [0.0f32; N * 2];

    // Rebuild full complex array (conjugate symmetry)
    data[0] = spectrum[0];
    data[1] = spectrum[1]; // should be 0
    for k in 1..N / 2 {
        let re = spectrum[2 * k];
        let im = spectrum[2 * k + 1];
        data[2 * k] = re;
        data[2 * k + 1] = im;
        data[2 * (N - k)] = re;
        data[2 * (N - k) + 1] = -im;
    }
    // Nyquist bin
    data[2 * (N / 2)] = spectrum[N];
    data[2 * (N / 2) + 1] = spectrum[N + 1];

    complex_fft_512(&mut data, &TWIDDLE_INV);

    // Scale by 1/N
    for i in 0..N {
        output[i] = data[2 * i] / N as f32;
    }
}

// ─────────────────────────────────────────────────────────────
// Noise profile (precomputed for frame size 512, 16 kHz mono)
// ─────────────────────────────────────────────────────────────

pub static NOISE_PROFILE: [f32; 257] = [
    2.9622477293e-01,    4.2800217867e-01,    4.6643093228e-01,    4.5134428144e-01,    4.8320764303e-01,    1.0062615871e+00,    9.7112739086e-01,    4.1892996430e-01,
    3.6424595118e-01,    3.6635911465e-01,    3.8349029422e-01,    3.9469474554e-01,    4.1842782497e-01,    4.0275514126e-01,    3.8138243556e-01,    3.8822716475e-01,
    4.0049105883e-01,    3.7654945254e-01,    3.5409063101e-01,    3.4758976102e-01,    3.4798860550e-01,    3.5197985172e-01,    3.5356876254e-01,    3.6978614330e-01,
    3.8198623061e-01,    3.7661361694e-01,    3.7864392996e-01,    4.0061253309e-01,    4.0442207456e-01,    3.9953815937e-01,    4.0667495131e-01,    4.6624618769e-01,
    5.3760451078e-01,    5.7828712463e-01,    5.8859306574e-01,    5.9801810980e-01,    5.6609040499e-01,    5.2864223719e-01,    4.5428895950e-01,    3.9588248730e-01,
    3.8149014115e-01,    3.6419853568e-01,    3.5152843595e-01,    3.7250086665e-01,    3.8180905581e-01,    3.8394579291e-01,    3.5518422723e-01,    3.6589232087e-01,
    3.5416224599e-01,    3.4652495384e-01,    3.2839062810e-01,    3.3882775903e-01,    3.3833730221e-01,    3.3721593022e-01,    3.3658722043e-01,    3.5548341274e-01,
    3.3378523588e-01,    3.3108845353e-01,    3.2483416796e-01,    3.3685147762e-01,    3.3005318046e-01,    3.2934537530e-01,    3.2308188081e-01,    3.3941733837e-01,
    3.3776587248e-01,    3.3158391714e-01,    3.1914380193e-01,    3.2272914052e-01,    3.1788265705e-01,    3.2926577330e-01,    3.1849411130e-01,    3.2079738379e-01,
    3.1891295314e-01,    3.2634714246e-01,    3.0687630177e-01,    3.1586411595e-01,    3.1692588329e-01,    3.2587075233e-01,    3.0967149138e-01,    3.2741919160e-01,
    3.2621827722e-01,    3.2528588176e-01,    3.0303731561e-01,    3.1783744693e-01,    3.1586828828e-01,    3.2121053338e-01,    2.9850700498e-01,    3.2148665190e-01,
    3.1708937883e-01,    3.1981498003e-01,    3.0279365182e-01,    3.2503592968e-01,    3.2369226217e-01,    3.2802608609e-01,    3.1047284603e-01,    3.2648494840e-01,
    3.2007741928e-01,    3.2032787800e-01,    3.0617344379e-01,    3.2769876719e-01,    3.2013571262e-01,    3.2500177622e-01,    3.1318557262e-01,    3.3643451333e-01,
    3.2545563579e-01,    3.2630681992e-01,    3.1602764130e-01,    3.3201608062e-01,    3.2271865010e-01,    3.3079102635e-01,    3.1751909852e-01,    3.2778775692e-01,
    3.1980520487e-01,    3.3005359769e-01,    3.1493461132e-01,    3.2440498471e-01,    3.2188594341e-01,    3.3384868503e-01,    3.1215277314e-01,    3.2577389479e-01,
    3.2442218065e-01,    3.3533829451e-01,    3.1580293179e-01,    3.2799127698e-01,    3.2374796271e-01,    3.3010056615e-01,    3.0679666996e-01,    3.2561144233e-01,
    3.2156696916e-01,    3.2876476645e-01,    3.1070390344e-01,    3.2304421067e-01,    3.2036012411e-01,    3.2548311353e-01,    3.0665501952e-01,    3.2141059637e-01,
    3.1663152575e-01,    3.2402414083e-01,    3.0314773321e-01,    3.2200244069e-01,    3.1828892231e-01,    3.1965115666e-01,    3.0589652061e-01,    3.2141831517e-01,
    3.2181435823e-01,    3.2561999559e-01,    3.0973455310e-01,    3.2688605785e-01,    3.2771125436e-01,    3.3009511232e-01,    3.0991032720e-01,    3.2467779517e-01,
    3.2367736101e-01,    3.2658246160e-01,    3.0933630466e-01,    3.2485309243e-01,    3.2146507502e-01,    3.2910019159e-01,    3.1006672978e-01,    3.2840093970e-01,
    3.2960438728e-01,    3.3515992761e-01,    3.1246599555e-01,    3.2913982868e-01,    3.2917347550e-01,    3.3545297384e-01,    3.1717398763e-01,    3.3283933997e-01,
    3.2923066616e-01,    3.2818138599e-01,    3.1256377697e-01,    3.2921668887e-01,    3.2684943080e-01,    3.2896721363e-01,    3.1210473180e-01,    3.3302855492e-01,
    3.2585680485e-01,    3.2689535618e-01,    3.0981308222e-01,    3.3009111881e-01,    3.2159522176e-01,    3.2287102938e-01,    3.0724096298e-01,    3.2863482833e-01,
    3.2108569145e-01,    3.2249104977e-01,    3.0605053902e-01,    3.2669323683e-01,    3.1924816966e-01,    3.1734323502e-01,    3.0472555757e-01,    3.2564866543e-01,
    3.1573244929e-01,    3.1683820486e-01,    3.0229040980e-01,    3.2147741318e-01,    3.1308579445e-01,    3.1339219213e-01,    2.9703411460e-01,    3.1557065248e-01,
    3.1304061413e-01,    3.1597390771e-01,    3.0120876431e-01,    3.2017335296e-01,    3.1441891193e-01,    3.1832721829e-01,    3.0146205425e-01,    3.2129824162e-01,
    3.1547048688e-01,    3.1555977464e-01,    2.9811218381e-01,    3.1972870231e-01,    3.1213814020e-01,    3.1480163336e-01,    3.0082631111e-01,    3.2098197937e-01,
    3.1206190586e-01,    3.1424584985e-01,    2.9739272594e-01,    3.2047748566e-01,    3.1367868185e-01,    3.1435137987e-01,    2.9860168695e-01,    3.2046613097e-01,
    3.1169137359e-01,    3.1320726871e-01,    3.0341330171e-01,    3.2294070721e-01,    3.1216695905e-01,    3.1407168508e-01,    3.0121934414e-01,    3.1661388278e-01,
    3.0506670475e-01,    3.1558555365e-01,    3.0024841428e-01,    3.1348004937e-01,    3.0652844906e-01,    3.1704553962e-01,    2.9851734638e-01,    3.1349578500e-01,
    3.0946347117e-01,    3.1712111831e-01,    2.9678356647e-01,    3.1374794245e-01,    3.0977767706e-01,    3.1844606996e-01,    2.9924505949e-01,    3.1449246407e-01,
    3.1387293339e-01,    3.2022029161e-01,    3.0175012350e-01,    3.2297217846e-01,    3.1456369162e-01,    3.1440114975e-01,    3.0016547441e-01,    3.1785455346e-01,
    3.0503737926e-01,
];

/// Apply spectral subtraction using the embedded noise profile.
///
/// `audio` – mono `f32` samples, 16 kHz.
/// `strength` – typical 1.0–4.0 (2.0 is a good start).
///
/// Returns a new `Vec<f32>` with the buzz reduced.
pub fn reduce_noise(audio: &[f32], strength: f32) -> Vec<f32> {
    const FRAME: usize = 512;
    const HOP: usize = FRAME / 4;      // 75 % overlap
    const FLOOR: f32 = 0.02;           // spectral floor factor

    if audio.len() < FRAME {
        return audio.to_vec();
    }

    let num_frames = (audio.len() - FRAME) / HOP + 1;
    let output_len = (num_frames - 1) * HOP + FRAME;
    let mut output = vec![0.0f32; output_len];
    let mut window_sum = vec![0.0f32; output_len];

    let mut frame_buf = [0.0f32; FRAME];
    let mut spectrum = [0.0f32; FRAME + 2];   // real FFT output

    for i in 0..num_frames {
        let start = i * HOP;

        // Copy and window
        for j in 0..FRAME {
            frame_buf[j] = audio[start + j] * WINDOW[j];
        }

        // Forward real FFT
        real_fft_512(&frame_buf, &mut spectrum);

        // Spectral subtraction
        for k in 0..=FRAME / 2 {
            let idx = 2 * k;
            let re = spectrum[idx];
            let im = spectrum[idx + 1];
            let mag = sqrtf(re * re + im * im);
            let phase = atan2f(im, re);

            let noise_mag = NOISE_PROFILE[k];
            let mut cleaned_mag = mag - strength * noise_mag;
            let floor = FLOOR * mag;
            if cleaned_mag < floor {
                cleaned_mag = floor;
            }
            if cleaned_mag > mag {
                cleaned_mag = mag;
            }

            spectrum[idx] = mag * cosf(phase);
            spectrum[idx + 1] = mag * sinf(phase);
        }

        // Inverse real FFT (in‑place into frame_buf)
        real_ifft_512(&spectrum, &mut frame_buf);

        // Overlap‑add
        for j in 0..FRAME {
            let idx_out = start + j;
            output[idx_out] += frame_buf[j] * WINDOW[j];
            window_sum[idx_out] += WINDOW[j] * WINDOW[j];
        }
    }

    // Normalise overlap‑add
    for (out, wsum) in output.iter_mut().zip(window_sum.iter()) {
        if *wsum > 1e-10 {
            *out /= wsum;
        }
    }

    output
}
