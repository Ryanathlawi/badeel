# -*- coding: utf-8 -*-
"""صور نشاط ديسكورد لبديل.

    python assets/discord/make.py

يخرج في نفس المجلد: large.png و small.png بمقاس 1024 وهو الموصى به
عند ديسكورد لصور النشاط، الأولى الصورة الكبيرة والثانية الشارة التي
تظهر على ركنها، و cover.png بمقاس 1024 في 576 لصورة دعوة الانضمام،
و banner.png بنفس التصميم بمقاس 1920 في 1080 للاستعمال خارج ديسكورد.

يُرسم كل شيء بأربعة أضعاف المقاس ثم يُصغَّر، فتخرج الحواف ناعمة بلا
أي اعتماد على خط أو ملف خارجي.
"""
import math
import os

import arabic_reshaper
from PIL import Image, ImageDraw, ImageFilter, ImageFont

try:
    from bidi.algorithm import get_display
except ImportError:  # python-bidi >= 0.5
    from bidi import get_display

OUT = os.path.dirname(os.path.abspath(__file__))
SIZE = 1024  # الموصى به عند ديسكورد، وأقلّه المقبول 512
SS = 2  # التكبير قبل التصغير

FONTS = os.path.join(os.path.dirname(OUT), "fonts", "fallback")
RESHAPER = arabic_reshaper.ArabicReshaper({"delete_harakat": False})

BG = (14, 18, 25)
DEEP = (9, 12, 17)
TEAL = (42, 182, 166)
BLUE = (112, 178, 232)
INK = (233, 241, 248)


def ar(text):
    """تشكيل الحروف العربية ووصلها قبل رسمها، فـ PIL لا يفعلها بنفسه."""
    return get_display(RESHAPER.reshape(text))


def font(size):
    return ImageFont.truetype(os.path.join(FONTS, "IBMPlexSansArabic-Medium.ttf"), size)


def hexagon(cx, cy, r, rot=math.pi / 2):
    """سداسي رأسه للأعلى، نفس الذي يرسمه البرنامج وواجهته."""
    return [
        (cx + r * math.cos(rot + math.pi / 3 * i), cy - r * math.sin(rot + math.pi / 3 * i))
        for i in range(6)
    ]


def glow(size, draw_shape, colour, strength, blur):
    """هالة ناعمة خلف شكل ما."""
    layer = Image.new("L", size, 0)
    draw_shape(ImageDraw.Draw(layer))
    layer = layer.filter(ImageFilter.GaussianBlur(blur))
    tint = Image.new("RGB", size, colour)
    return tint, layer.point(lambda v: int(v * strength))


def backdrop(w, h=None):
    """أرضية داكنة عليها شبكة خلايا خافتة وهالتان."""
    h = h or w
    n = max(w, h)
    im = Image.new("RGB", (w, h), BG)
    d = ImageDraw.Draw(im, "RGBA")

    # تدرّج من المنتصف
    mid = Image.new("L", (w, h), 0)
    ImageDraw.Draw(mid).ellipse([-w * 0.15, -h * 0.25, w * 1.15, h * 1.05], fill=255)
    im.paste(Image.new("RGB", (w, h), (18, 32, 36)), (0, 0), mid.filter(ImageFilter.GaussianBlur(n * 0.22)))

    # شبكة الخلايا
    step = n * 0.115
    r = step * 0.3
    rows = int(h / (step * 0.87)) + 3
    cols = int(w / step) + 3
    for j in range(rows):
        for i in range(cols):
            cx = i * step + (step * 0.5 if j % 2 else 0) - step
            cy = j * step * 0.87 - step
            tone = TEAL if (i + j) % 3 else BLUE
            d.line(hexagon(cx, cy, r) + [hexagon(cx, cy, r)[0]], fill=tone + (20,), width=max(1, int(n // 512)))

    # هالتان لونيتان
    for centre, colour, strength in (((0.24, 0.18), TEAL, 0.30), ((0.82, 0.86), BLUE, 0.16)):
        c = (centre[0] * w, centre[1] * h)
        rad = (w * h) ** 0.5 * 0.45  # المتوسط الهندسي، فلا تبتلع الهالة الإطار العريض
        tint, mask = glow(
            (w, h),
            lambda dd, c=c, rad=rad: dd.ellipse([c[0] - rad, c[1] - rad, c[0] + rad, c[1] + rad], fill=255),
            colour,
            strength,
            rad * 0.5,
        )
        im.paste(tint, (0, 0), mask)

    # تعتيم خفيف عند الأطراف يجمع الصورة بلا أن يسوّد الأركان
    vig = Image.new("L", (w, h), 0)
    ImageDraw.Draw(vig).ellipse([-w * 0.34, -h * 0.34, w * 1.34, h * 1.34], fill=255)
    vig = vig.filter(ImageFilter.GaussianBlur(n * 0.10)).point(lambda v: int((255 - v) * 0.55))
    im.paste(Image.new("RGB", (w, h), DEEP), (0, 0), vig)
    return im


def swap_arrows(d, cx, cy, span, w, colour):
    """سهما التبديل: واحد يمشي يمينًا وفوقه الآخر يرجع يسارًا."""
    half = span / 2
    gap = span * 0.26
    head = span * 0.22

    # الأعلى: من اليسار إلى اليمين
    y = cy - gap / 2
    d.line([(cx - half, y), (cx + half, y)], fill=colour, width=w, joint="curve")
    d.line([(cx + half - head, y - head * 0.82), (cx + half, y), (cx + half - head, y + head * 0.82)],
           fill=colour, width=w, joint="curve")

    # الأسفل: من اليمين إلى اليسار
    y = cy + gap / 2
    d.line([(cx + half, y), (cx - half, y)], fill=colour, width=w, joint="curve")
    d.line([(cx - half + head, y - head * 0.82), (cx - half, y), (cx - half + head, y + head * 0.82)],
           fill=colour, width=w, joint="curve")

    # نهايات مستديرة
    for px, py in ((cx - half, cy - gap / 2), (cx + half, cy - gap / 2),
                   (cx + half, cy + gap / 2), (cx - half, cy + gap / 2)):
        d.ellipse([px - w / 2, py - w / 2, px + w / 2, py + w / 2], fill=colour)


def large():
    n = SIZE * SS
    im = backdrop(n)
    c = n / 2

    # حلقتان سداسيتان خارجيتان تعطيان عمقًا
    d = ImageDraw.Draw(im, "RGBA")
    for rr, alpha, wd in ((0.455, 40, 2), (0.395, 70, 2)):
        pts = hexagon(c, c, n * rr)
        d.line(pts + [pts[0]], fill=TEAL + (alpha,), width=int(n * 0.004 * wd / 2))

    # هالة السداسي الرئيسي
    r = n * 0.325
    tint, mask = glow(
        (n, n),
        lambda dd: dd.polygon(hexagon(c, c, r * 1.04), fill=255),
        TEAL,
        0.55,
        n * 0.055,
    )
    im.paste(tint, (0, 0), mask)

    # السداسي الرئيسي
    d = ImageDraw.Draw(im, "RGBA")
    pts = hexagon(c, c, r)
    d.polygon(pts, fill=(16, 38, 40, 235))
    d.line(pts + [pts[0]], fill=TEAL + (255,), width=int(n * 0.012))

    # السهمان في قلبه
    swap_arrows(d, c, c, span=r * 0.98, w=int(n * 0.030), colour=INK + (255,))

    im.resize((SIZE, SIZE), Image.LANCZOS).save(os.path.join(OUT, "large.png"), optimize=True)


def small():
    """شارة القفل الصغيرة التي تظهر على ركن الصورة الكبيرة."""
    n = SIZE * SS
    im = Image.new("RGB", (n, n), BG)
    d = ImageDraw.Draw(im, "RGBA")
    c = n / 2

    tint, mask = glow(
        (n, n),
        lambda dd: dd.ellipse([n * 0.1, n * 0.1, n * 0.9, n * 0.9], fill=255),
        TEAL,
        0.5,
        n * 0.08,
    )
    im.paste(tint, (0, 0), mask)

    d = ImageDraw.Draw(im, "RGBA")
    d.ellipse([n * 0.06, n * 0.06, n * 0.94, n * 0.94], fill=(13, 33, 34, 255), outline=TEAL + (255,), width=int(n * 0.022))

    # قفل مغلق
    w = int(n * 0.05)
    body = [c - n * 0.17, c - n * 0.04, c + n * 0.17, c + n * 0.24]
    d.rounded_rectangle(body, radius=n * 0.05, fill=TEAL + (46,), outline=INK + (255,), width=w)
    d.arc([c - n * 0.105, c - n * 0.15, c + n * 0.105, c + n * 0.07], 180, 360, fill=INK + (255,), width=w)
    d.ellipse([c - n * 0.035, c + n * 0.055, c + n * 0.035, c + n * 0.125], fill=INK + (255,))

    im.resize((SIZE, SIZE), Image.LANCZOS).save(os.path.join(OUT, "small.png"), optimize=True)




def banner():
    """غلاف التطبيق: العلامة والاسم وسطر واحد وثلاث شرائح، كلها في
    وسط الإطار لأن الأغلفة تُقصّ من أطرافها حسب مكان عرضها."""
    w, h = 1920 * 2, 1080 * 2
    im = backdrop(w, h)
    d = ImageDraw.Draw(im, "RGBA")
    cx = w / 2

    # العلامة
    my = h * 0.285
    r = h * 0.155
    for rr, alpha in ((1.40, 34), (1.21, 64)):
        pts = hexagon(cx, my, r * rr)
        d.line(pts + [pts[0]], fill=TEAL + (alpha,), width=int(h * 0.0035))
    tint, mask = glow(
        (w, h), lambda dd: dd.polygon(hexagon(cx, my, r * 1.04), fill=255), TEAL, 0.5, h * 0.03
    )
    im.paste(tint, (0, 0), mask)
    d = ImageDraw.Draw(im, "RGBA")
    pts = hexagon(cx, my, r)
    d.polygon(pts, fill=(16, 38, 40, 235))
    d.line(pts + [pts[0]], fill=TEAL + (255,), width=int(h * 0.0075))
    swap_arrows(d, cx, my, span=r * 0.98, w=int(h * 0.019), colour=INK + (255,))

    # الاسم والسطر تحته
    d.text((cx, h * 0.545), ar("بديل"), font=font(int(h * 0.135)), fill=INK, anchor="mm")
    d.text(
        (cx, h * 0.665),
        ar("مبدّل حسابات الألعاب لسبع منصّات على ويندوز"),
        font=font(int(h * 0.040)),
        fill=(150, 168, 186),
        anchor="mm",
    )

    # ثلاث شرائح
    chips = ["مفتوح المصدر", "بدون كلمة سر", "AES-256"]
    f = font(int(h * 0.031))
    pad = h * 0.026
    gap = h * 0.020
    boxes = []
    for label in chips:
        latin = all(ord(c) < 0x590 for c in label)
        text = label if latin else ar(label)
        boxes.append((text, d.textlength(text, font=f) + pad * 2))
    total = sum(b[1] for b in boxes) + gap * (len(boxes) - 1)
    x = cx + total / 2
    top = h * 0.755
    for i, (text, bw) in enumerate(boxes):
        d.rounded_rectangle(
            [x - bw, top, x, top + h * 0.072],
            radius=h * 0.020,
            fill=TEAL + (30,),
            outline=TEAL + (120,),
            width=max(1, int(h * 0.0016)),
        )
        d.text((x - bw / 2, top + h * 0.036), text, font=f, fill=INK if i else TEAL, anchor="mm")
        x -= bw + gap

    d.text(
        (cx, h * 0.905),
        "ryanathlawi.github.io/badeel-site",
        font=font(int(h * 0.029)),
        fill=TEAL,
        anchor="mm",
    )

    for name, out in (("banner.png", (1920, 1080)), ("cover.png", (1024, 576))):
        im.resize(out, Image.LANCZOS).save(os.path.join(OUT, name), optimize=True)


if __name__ == "__main__":
    large()
    small()
    banner()
    for f in ("large.png", "small.png", "cover.png", "banner.png"):
        p = os.path.join(OUT, f)
        print(f, Image.open(p).size, os.path.getsize(p) // 1024, "KB")
