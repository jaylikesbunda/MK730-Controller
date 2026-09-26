from PIL import Image, ImageDraw, ImageFont
import shutil
import subprocess
import tempfile
from pathlib import Path

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

root = Path(__file__).resolve().parents[1]
icon_dir = root / "src-tauri" / "icons"
png_path = icon_dir / "icon.png"
img.save(png_path)
print("png saved", img.size)

# Tauri writes correctly sized ICO and ICNS variants. Hand-writing an ICO
# directory around a 512px PNG labels the image as 256px and looks rough in
# the Windows taskbar.
with tempfile.TemporaryDirectory() as temp_dir:
    subprocess.run(
        ["cargo", "tauri", "icon", str(png_path), "--output", temp_dir],
        cwd=root / "src-tauri",
        check=True,
    )
    for name in ("icon.ico", "icon.icns"):
        shutil.copy2(Path(temp_dir) / name, icon_dir / name)
print("generated multi-size Windows and macOS icons")
