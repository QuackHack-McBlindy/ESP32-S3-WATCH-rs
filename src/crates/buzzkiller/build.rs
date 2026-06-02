// build.rs
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let dest_path = out_dir.join("tables.rs");

    let mut file = File::create(&dest_path).unwrap();

    const N: usize = 512;

    // Generate twiddle factors for forward FFT
    writeln!(file, "pub const TWIDDLE_FWD: [(f32, f32); {}] = [", N / 2).unwrap();
    for k in 0..N / 2 {
        let angle = -2.0 * std::f32::consts::PI * k as f32 / N as f32;
        let (c, s) = (angle.cos(), angle.sin());
        writeln!(file, "    ({:.20e}, {:.20e}),", c, s).unwrap();
    }
    writeln!(file, "];").unwrap();

    // Generate twiddle factors for inverse FFT
    writeln!(file, "pub const TWIDDLE_INV: [(f32, f32); {}] = [", N / 2).unwrap();
    for k in 0..N / 2 {
        let angle = 2.0 * std::f32::consts::PI * k as f32 / N as f32;
        let (c, s) = (angle.cos(), angle.sin());
        writeln!(file, "    ({:.20e}, {:.20e}),", c, s).unwrap();
    }
    writeln!(file, "];").unwrap();

    // Generate Hann window
    writeln!(file, "pub const WINDOW: [f32; {}] = [", N).unwrap();
    for i in 0..N {
        let w = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (N as f32 - 1.0)).cos());
        writeln!(file, "    {:.20e},", w).unwrap();
    }
    writeln!(file, "];").unwrap();

    println!("cargo:rerun-if-changed=build.rs");
}
