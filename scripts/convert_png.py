from PIL import Image
import sys

img = Image.open(sys.argv[1]).convert("RGBA")

# resize
img = img.resize((180, 180), Image.LANCZOS)

# reduce colors
img = img.quantize(colors=256)

# save
img.save("converted.png")
print("Done.")
