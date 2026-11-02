"""Render the code-defined SVG geometry into release icons (Pillow, dev only)."""
from pathlib import Path
from PIL import Image, ImageDraw

assets = Path(__file__).resolve().parents[1] / 'assets'
scale = 16
image = Image.new('RGBA', (64 * scale, 64 * scale))
draw = ImageDraw.Draw(image)
draw.rounded_rectangle((0, 0, 64*scale-1, 64*scale-1), radius=14*scale, fill='#181b22')

def line(points, color, width, closed=False):
    pixels = [(x*scale, y*scale) for x,y in points]
    if closed:
        pixels.append(pixels[0])
    draw.line(pixels, fill=color, width=round(width*scale), joint='curve')
    radius = width*scale/2
    for x,y in pixels:
        draw.ellipse((x-radius,y-radius,x+radius,y+radius),fill=color)

line([(32,7),(54,20),(54,44),(32,57),(10,44),(10,20)],'#ff8f4f',3,True)
line([(21,43),(32,20),(43,43)],'#f2f4f8',4)
line([(26,34),(38,34)],'#f2f4f8',4)
logo=image.resize((256,256),Image.Resampling.LANCZOS)
logo.save(assets/'apexcontrol.png')
logo.save(assets/'apexcontrol.ico',sizes=[(16,16),(24,24),(32,32),(48,48),(64,64),(128,128),(256,256)])
(assets/'logo-32.rgba').write_bytes(image.resize((32,32),Image.Resampling.LANCZOS).tobytes())

