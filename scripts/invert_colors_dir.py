#!/usr/bin/env python3
"""
invert all PNG images in a directory (RGB inversion, alpha unchanged).
Usage:
    python invert_pngs.py [directory]
output is written to a subfolder 'inverted' inside the target directory.
"""

import os
import sys
from PIL import Image, ImageOps

def invert_image(input_path, output_path):
    img = Image.open(input_path).convert("RGBA")
    r, g, b, a = img.split()

    rgb = Image.merge("RGB", (r, g, b))
    inverted_rgb = ImageOps.invert(rgb)

    inverted_r, inverted_g, inverted_b = inverted_rgb.split()
    inverted_img = Image.merge("RGBA", (inverted_r, inverted_g, inverted_b, a))
    inverted_img.save(output_path, optimize=True)

def main():
    target_dir = sys.argv[1] if len(sys.argv) > 1 else "."
    out_dir = os.path.join(target_dir, "inverted")
    os.makedirs(out_dir, exist_ok=True)

    png_files = [f for f in os.listdir(target_dir) if f.lower().endswith(".png")]
    if not png_files:
        print(f"No PNG files found in {target_dir}")
        return

    print(f"Inverting {len(png_files)} PNG(s) from {target_dir} → {out_dir} …")
    for fname in png_files:
        in_path = os.path.join(target_dir, fname)
        out_path = os.path.join(out_dir, fname)
        invert_image(in_path, out_path)
        print(f"  {fname} done")

    print("All done. Inverted images are in:", out_dir)

if __name__ == "__main__":
    main()
