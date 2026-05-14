from PIL import Image
import sys
import os

filename = sys.argv[1]

img = Image.open(filename)

print("=" * 60)
print("FILE")
print("=" * 60)
print("Filename:", filename)
print("Format:", img.format)
print("MIME:", Image.MIME.get(img.format))
print("Mode:", img.mode)
print("Size:", img.size)
print("Width:", img.width)
print("Height:", img.height)
print("Animated:", getattr(img, "is_animated", False))
print("Frames:", getattr(img, "n_frames", 1))

print("\n" + "=" * 60)
print("IMAGE INFO")
print("=" * 60)

for k, v in img.info.items():
    print(f"{k}: {v}")

print("\n" + "=" * 60)
print("COLOR / PIXEL INFO")
print("=" * 60)

print("Bands:", img.getbands())

try:
    colors = img.getcolors(maxcolors=10_000_000)
    if colors:
        print("Unique colors:", len(colors))

        print("\nTop 10 colors:")
        colors = sorted(colors, reverse=True)

        for count, color in colors[:10]:
            print(f"{count:10} -> {color}")
    else:
        print("Too many colors to count")
except Exception as e:
    print("Color analysis failed:", e)

print("\n" + "=" * 60)
print("PALETTE")
print("=" * 60)

if img.palette:
    print("Palette mode:", img.palette.mode)
    print("Palette colors:", len(img.palette.palette) // len(img.palette.mode))
else:
    print("No palette")

print("\n" + "=" * 60)
print("METADATA")
print("=" * 60)

if hasattr(img, "text"):
    for k, v in img.text.items():
        print(f"{k}: {v}")

print("\n" + "=" * 60)
print("PNG CHUNKS / EXTRA")
print("=" * 60)

for k, v in img.info.items():
    print(f"{k}: {type(v)}")

print("\n" + "=" * 60)
print("EXTREMA")
print("=" * 60)

try:
    print("Channel extrema:", img.getextrema())
except Exception as e:
    print("Extrema failed:", e)

print("\n" + "=" * 60)
print("HISTOGRAM")
print("=" * 60)

hist = img.histogram()
print("Histogram length:", len(hist))
print("Histogram sample:", hist[:32])

print("\n" + "=" * 60)
print("TRANSPARENCY")
print("=" * 60)

if "transparency" in img.info:
    print("Transparency info:", img.info["transparency"])
else:
    print("No transparency info")

print("\n" + "=" * 60)
print("DPI")
print("=" * 60)

print("DPI:", img.info.get("dpi"))

print("\n" + "=" * 60)
print("ICC PROFILE")
print("=" * 60)

icc = img.info.get("icc_profile")
if icc:
    print("ICC profile size:", len(icc), "bytes")
else:
    print("No ICC profile")

print("\n" + "=" * 60)
print("DONE")
print("=" * 60)
