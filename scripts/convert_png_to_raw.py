from PIL import Image
import sys
import os

# Input PNG from command line
input_png = sys.argv[1]

# Output RAW filename
output_raw = os.path.splitext(input_png)[0] + ".raw"

# Convert to RGB565 raw bytes
img = Image.open(input_png).convert("RGB")
pixels = img.load()

raw = bytearray()

for y in range(img.height):
    for x in range(img.width):
        r, g, b = pixels[x, y]

        rgb565 = ((r & 0xF8) << 8) | ((g & 0xFC) << 3) | (b >> 3)

        # Little-endian
        raw.append(rgb565 & 0xFF)
        raw.append((rgb565 >> 8) & 0xFF)

with open(output_raw, "wb") as f:
    f.write(raw)

print(f"Saved: {output_raw}")
