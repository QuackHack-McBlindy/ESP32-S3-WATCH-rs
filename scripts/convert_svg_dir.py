#!/usr/bin/env python3
"""
batch convert SVG files to 180×180 PNGs with transparency and reduced colours.
Requires: cairosvg, Pillow, pngquant
Install:
    pip install Pillow cairosvg
"""

import os, sys, subprocess, tempfile, io
import cairosvg
from PIL import Image

def convert_svg(svg_path, output_path, size=(180, 180)):

    png_data = cairosvg.svg2png(url=svg_path, output_width=size[0], output_height=size[1])
    
    try:
        result = subprocess.run(
            ["pngquant", "--force", "--quality=0-100", "--output", output_path, "-"],
            input=png_data,
            capture_output=True,
            timeout=5
        )
        if result.returncode == 0:
            print(f"  {svg_path} → {output_path} (quantized with pngquant)")
            return
        else:
            print(f"  pngquant failed, falling back to full-colour PNG.")
    except FileNotFoundError:
        print("  pngquant not found, saving full-colour PNG.")

    img = Image.open(io.BytesIO(png_data))
    img.save(output_path, optimize=True)
    print(f"  {svg_path} → {output_path} (full colour)")

if __name__ == "__main__":
    target_dir = sys.argv[1] if len(sys.argv) > 1 else "."
    out_dir = os.path.join(target_dir, "converted")
    os.makedirs(out_dir, exist_ok=True)

    svg_files = [f for f in os.listdir(target_dir) if f.lower().endswith(".svg")]
    if not svg_files:
        print(f"No SVG files in {target_dir}")
        sys.exit(0)

    print(f"Converting {len(svg_files)} SVG(s)…")
    for fname in svg_files:
        in_path = os.path.join(target_dir, fname)
        out_path = os.path.join(out_dir, fname.replace(".svg", ".png"))
        convert_svg(in_path, out_path)

    print("Done. Output in:", out_dir)
