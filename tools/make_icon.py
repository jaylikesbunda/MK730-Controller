from PIL import Image, ImageDraw, ImageFont
import struct
S = 256
img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
d = ImageDraw.Draw(img)
d.rounded_rectangle([8, 8, S-8, S-8], radius=56, fill=(18, 21, 28, 255))
d.rounded_rectangle([8, 8, S-8, S-8], radius=56, outline=(124, 58, 237, 255), width=6)
try:
    f = ImageFont.truetype("arial.ttf", 110)
except Exception:
    f = ImageFont.load_default()
d.text((S//2, S//2-10), "MK", fill=(196, 181, 253, 255), font=f, anchor="mm")
img.save("src-tauri/icons/icon.png")
print("png saved")
png = open("src-tauri/icons/icon.png", "rb").read()
h = struct.pack("<HHH", 0, 1, 1)
e = struct.pack("<BBBBHHII", 0, 0, 0, 0, 1, 32, len(png), 22)
open("src-tauri/icons/icon.ico", "wb").write(h + e + png)
print("ico saved", len(png))
