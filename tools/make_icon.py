from PIL import Image, ImageDraw, ImageFont
import struct

S = 512
img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
d = ImageDraw.Draw(img)
# solid rounded tile
d.rounded_rectangle([8, 8, S-8, S-8], radius=112, fill=(17, 20, 27, 255))
d.rounded_rectangle([8, 8, S-8, S-8], radius=112, outline=(124, 58, 237, 255), width=10)

# centered letters: measure exactly, no anchor guesswork
for size in (220, 200, 180, 160):
    try:
        f = ImageFont.truetype("arial.ttf", size)
    except Exception:
        try:
            f = ImageFont.truetype("DejaVuSans-Bold.ttf", size)
        except Exception:
            f = ImageFont.load_default()
            break
    bb = d.textbbox((0, 0), "MK", font=f)
    w, h = bb[2]-bb[0], bb[3]-bb[1]
    if w < S*0.62 and h < S*0.5:
        break
x = (S - w)/2 - bb[0]
y = (S - h)/2 - bb[1] - 6
d.text((x, y), "MK", fill=(196, 181, 253, 255), font=f)

img.save("src-tauri/icons/icon.png")
print("png saved", img.size)
png = open("src-tauri/icons/icon.png", "rb").read()
h = struct.pack("<HHH", 0, 1, 1)
e = struct.pack("<BBBBHHII", 0, 0, 0, 0, 1, 32, len(png), 22)
open("src-tauri/icons/icon.ico", "wb").write(h + e + png)
print("ico saved", len(png))
