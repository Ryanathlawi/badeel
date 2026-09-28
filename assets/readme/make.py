# -*- coding: utf-8 -*-
"""صور صفحة المستودع.

    python assets/readme/make.py

تخرج في .assets بنسختين لكل صورة، داكنة وفاتحة، ليتبع الملف مظهر من
يقرؤه على GitHub. كل شيء مرسوم هنا بالخط المرفق مع المشروع، فلا
يعتمد الناتج على خط من النظام ولا على أي مورد خارجي.
"""
import math
import os

import arabic_reshaper
from PIL import Image, ImageDraw, ImageFilter, ImageFont

try:
    from bidi.algorithm import get_display
except ImportError:  # python-bidi >= 0.5
    from bidi import get_display

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
FONTS = os.path.join(ROOT, "assets", "fonts", "fallback")
OUT = os.path.join(ROOT, ".assets")
os.makedirs(OUT, exist_ok=True)

W = 1400          # العرض المنطقي
SS = 2            # يُرسم بالضعف ثم يُصغَّر
RESHAPER = arabic_reshaper.ArabicReshaper({"delete_harakat": False})

TEAL = (42, 182, 166)
GOLD = (228, 176, 82)

DARK = {
    "bg": (14, 18, 25),
    "deep": (9, 12, 17),
    "card": (19, 26, 36),
    "line": (37, 48, 62),
    "text": (231, 239, 248),
    "muted": (154, 168, 186),
    "faint": (102, 116, 138),
    "grid": (42, 182, 166, 18),
}

LIGHT = {
    "bg": (244, 247, 250),
    "deep": (232, 238, 243),
    "card": (255, 255, 255),
    "line": (214, 223, 232),
    "text": (16, 24, 34),
    "muted": (78, 94, 112),
    "faint": (124, 140, 158),
    "grid": (23, 121, 110, 22),
}


def ar(text):
    """وصل الحروف العربية وترتيبها، فـ PIL لا يفعلها بنفسه.

    السطر عربي دائمًا هنا، فاتجاهه من اليمين ولو بدأ بكلمة لاتينية، وإلا
    قُلب ترتيب «Ctrl K يجد أي حساب» كله لأن أول حرف فيه لاتيني."""
    return get_display(RESHAPER.reshape(text), base_dir="R")


def txt(s):
    """عربي أم لاتيني، كلاهما بنفس الخط."""
    return ar(s) if any("؀" <= c <= "ۿ" for c in s) else s


def wrap(d, raw, f, limit):
    """يقسّم النص أسطرًا قبل وصل حروفه، فالوصل يقلب الترتيب ولا يصلح
    أن يُقسَّم بعده."""
    lines, line = [], ""
    for word in raw.split(" "):
        probe = (line + " " + word) if line else word
        if line and d.textlength(txt(probe), font=f) > limit:
            lines.append(line)
            line = word
        else:
            line = probe
    if line:
        lines.append(line)
    return lines


def font(size):
    return ImageFont.truetype(os.path.join(FONTS, "IBMPlexSansArabic-Medium.ttf"), size * SS)


def hexagon(cx, cy, r):
    return [
        (cx + r * math.cos(math.pi / 2 + math.pi / 3 * i),
         cy - r * math.sin(math.pi / 2 + math.pi / 3 * i))
        for i in range(6)
    ]


def canvas(h, pal):
    """أرضية بشبكة الخلايا الخافتة نفسها التي في البرنامج."""
    w, hh = W * SS, h * SS
    im = Image.new("RGB", (w, hh), pal["bg"])
    d = ImageDraw.Draw(im, "RGBA")
    step = 74 * SS
    r = step * 0.3
    for j in range(-1, int(hh / (step * 0.87)) + 2):
        for i in range(-1, int(w / step) + 2):
            cx = i * step + (step * 0.5 if j % 2 else 0)
            cy = j * step * 0.87
            d.line(hexagon(cx, cy, r) + [hexagon(cx, cy, r)[0]], fill=pal["grid"], width=SS)
    return im


def glow(size, shape, colour, strength, blur):
    layer = Image.new("L", size, 0)
    shape(ImageDraw.Draw(layer))
    layer = layer.filter(ImageFilter.GaussianBlur(blur))
    return Image.new("RGB", size, colour), layer.point(lambda v: int(v * strength))


def mark(im, cx, cy, r, pal):
    """السداسي بسهمي التبديل، نفس علامة البرنامج."""
    tint, msk = glow(im.size, lambda dd: dd.polygon(hexagon(cx, cy, r * 1.05), fill=255),
                     TEAL, 0.45, r * 0.4)
    im.paste(tint, (0, 0), msk)
    d = ImageDraw.Draw(im, "RGBA")
    pts = hexagon(cx, cy, r)
    d.polygon(pts, fill=(16, 38, 40, 235) if pal is DARK else (225, 245, 242, 255))
    d.line(pts + [pts[0]], fill=TEAL + (255,), width=int(r * 0.075))

    ink = pal["text"] + (255,)
    span, gap, head = r * 0.98, r * 0.26, r * 0.22
    wd = int(r * 0.115)
    for sign in (-1, 1):
        y = cy + sign * gap / 2
        x0, x1 = (cx - span / 2, cx + span / 2) if sign < 0 else (cx + span / 2, cx - span / 2)
        d.line([(x0, y), (x1, y)], fill=ink, width=wd)
        tipx = x1
        back = head if x1 < x0 else -head
        d.line([(tipx + back, y - head * 0.82), (tipx, y), (tipx + back, y + head * 0.82)],
               fill=ink, width=wd)
        for px in (x0, x1):
            d.ellipse([px - wd / 2, y - wd / 2, px + wd / 2, y + wd / 2], fill=ink)


def save(im, name):
    im.resize((W, im.height // SS), Image.LANCZOS).save(os.path.join(OUT, name), optimize=True)


# ─────────────────────────────── الصور ───────────────────────────────


def banner(pal, name):
    h = 300
    im = canvas(h, pal)
    c = im.height / 2
    mark(im, (W - 168) * SS, c, 88 * SS, pal)
    d = ImageDraw.Draw(im, "RGBA")

    x = (W - 300) * SS
    d.text((x, c - 74 * SS), txt("بديل"), font=font(64), fill=pal["text"], anchor="rm")
    d.text((x, c - 8 * SS), txt("مبدّل حسابات الألعاب لسبع منصّات على ويندوز"),
           font=font(27), fill=pal["muted"], anchor="rm")
    d.text((x, c + 34 * SS),
           txt("بدّل بضغطة واحدة بلا كلمة سر، وكل جلسة مشفّرة ومربوطة بجهازك وحده"),
           font=font(20), fill=pal["faint"], anchor="rm")

    f = font(17)
    cx = x
    for label in ("Rust", "AES-256-GCM", "GPL-3.0"):
        tw = d.textlength(label, font=f)
        pad = 15 * SS
        box = [cx - tw - pad * 2, c + 62 * SS, cx, c + 100 * SS]
        d.rounded_rectangle(box, radius=12 * SS, fill=TEAL + (26,), outline=TEAL + (120,), width=SS)
        d.text((box[0] + pad + tw / 2, c + 81 * SS), label, font=f, fill=TEAL, anchor="mm")
        cx = box[0] - 11 * SS

    save(im, name)


def section(title, sub, pal, name):
    h = 104
    im = canvas(h, pal)
    d = ImageDraw.Draw(im, "RGBA")
    c = im.height / 2
    right = (W - 40) * SS

    d.line([(right - 300 * SS, c + 40 * SS), (right, c + 40 * SS)],
           fill=pal["line"] + (255,), width=SS)
    d.text((right, c - 14 * SS), txt(title), font=font(31), fill=pal["text"], anchor="rm")
    if sub:
        d.text((right, c + 22 * SS), txt(sub), font=font(19), fill=pal["faint"], anchor="rm")
    for i in range(3):
        pts = hexagon(40 * SS + i * 34 * SS, c, 13 * SS)
        d.line(pts + [pts[0]], fill=TEAL + (90 + i * 60,), width=int(1.6 * SS))
    save(im, name)


def button(label, note, pal, name, primary=True):
    h = 86
    im = canvas(h, pal)
    d = ImageDraw.Draw(im, "RGBA")
    box = [26 * SS, 12 * SS, (W - 26) * SS, (h - 12) * SS]
    d.rounded_rectangle(
        box, radius=16 * SS,
        fill=TEAL + (46,) if primary else pal["card"] + (255,),
        outline=TEAL + (255,) if primary else pal["line"] + (255,),
        width=int(1.6 * SS),
    )
    mid = (box[1] + box[3]) / 2
    d.text(((box[0] + box[2]) / 2, mid - 9 * SS), txt(label), font=font(24),
           fill=pal["text"], anchor="mm")
    d.text(((box[0] + box[2]) / 2, mid + 17 * SS), txt(note), font=font(16),
           fill=pal["faint"], anchor="mm")
    save(im, name)


def cards(items, pal, name, cols=3):
    """شبكة بطاقات: عنوان وسطر تحته."""
    rows = (len(items) + cols - 1) // cols
    pad, gap = 26, 16
    cw = (W - pad * 2 - gap * (cols - 1)) / cols
    ch = 128
    h = pad * 2 + ch * rows + gap * (rows - 1)
    im = canvas(h, pal)
    d = ImageDraw.Draw(im, "RGBA")

    for i, (title, body) in enumerate(items):
        r, c = divmod(i, cols)
        # من اليمين إلى اليسار
        x1 = (W - pad - c * (cw + gap)) * SS
        x0 = x1 - cw * SS
        y0 = (pad + r * (ch + gap)) * SS
        y1 = y0 + ch * SS
        d.rounded_rectangle([x0, y0, x1, y1], radius=15 * SS,
                            fill=pal["card"] + (255,), outline=pal["line"] + (255,), width=SS)
        pts = hexagon(x1 - 30 * SS, y0 + 30 * SS, 11 * SS)
        d.polygon(pts, fill=TEAL + (200,))
        d.text((x1 - 54 * SS, y0 + 30 * SS), txt(title), font=font(21),
               fill=pal["text"], anchor="rm")

        f = font(16)
        y = y0 + 62 * SS
        for line in wrap(d, body, f, cw * SS - 44 * SS):
            d.text((x1 - 22 * SS, y), txt(line), font=f, fill=pal["muted"], anchor="rm")
            y += 24 * SS
    save(im, name)


def steps(items, pal, name):
    h = 150
    im = canvas(h, pal)
    d = ImageDraw.Draw(im, "RGBA")
    pad, gap = 26, 16
    cw = (W - pad * 2 - gap * (len(items) - 1)) / len(items)
    for i, (n, title, body) in enumerate(items):
        x1 = (W - pad - i * (cw + gap)) * SS
        x0 = x1 - cw * SS
        d.rounded_rectangle([x0, pad * SS, x1, (h - pad) * SS], radius=15 * SS,
                            fill=pal["card"] + (255,), outline=pal["line"] + (255,), width=SS)
        d.text((x1 - 22 * SS, (pad + 26) * SS), n, font=font(26), fill=TEAL, anchor="rm")
        d.text((x1 - 22 * SS, (pad + 58) * SS), txt(title), font=font(20),
               fill=pal["text"], anchor="rm")
        d.text((x1 - 22 * SS, (pad + 84) * SS), txt(body), font=font(15),
               fill=pal["muted"], anchor="rm")
    save(im, name)


SECTIONS = [
    ("download", "التحميل", "ملف واحد بلا مثبِّت"),
    ("shots", "لقطات من البرنامج", "من النسخة المنشورة بلا تركيب"),
    ("features", "ماذا فيه", "سبع منصّات ومحرّك واحد"),
    ("security", "الأمان", "ما الذي يحمي جلساتك بالضبط"),
    ("usage", "طريقة الاستخدام", "ثلاث خطوات ثم العب"),
    ("source", "المصدر والرخصة", "كل سطر معروض أمامك"),
]

FEATURES = [
    ("تبديل ذرّي", "يغلق المنصّة ويحفظ جلستك ويركّب الحساب ثم يشغّلها، وإن تعثّرت خطوة رجع كل ملف إلى مكانه"),
    ("مربوط بجهازك", "المفتاح مقفل بحساب ويندوز وبهذا الجهاز عبر DPAPI، فنسخة مسروقة إلى حاسب آخر لا تُفتح"),
    ("بلا كلمة سر", "لا يطلب كلمة سر المنصّة ولا يقرأها، وإنما ينقل ملفات الجلسة التي أنشأتها المنصّة"),
    ("ملف لكل شخص", "الجهاز الواحد يستخدمه أكثر من لاعب، فلكل واحد حساباته ولونه وإعداداته"),
    ("يجد تثبيتك", "لا مسارات مكتوبة مسبقًا، يقرأ سجل ويندوز وملفات المنصّات ولو كانت على قرص آخر"),
    ("بلا خوادم", "لا خادم ولا تتبّع، ولا يكلّم إلا GitHub للتحديثات واللوحة، ويمكن إيقافهما"),
    ("بحث شامل", "Ctrl K يجد أي حساب في أي منصّة ويبدّل له بضغطة Enter"),
    ("ينتقل معك", "ملف مقفل بكلمة سر ينقل حساباتك إلى جهاز جديد أو بعد الفرمتة"),
    ("يحمي مباراتك", "لا يبدّل ولعبتك تعمل، فلا يقطع مباراة ولا يجلب لك عقوبة خروج"),
]

SECURITY = [
    ("AES-256-GCM", "تشفير موثَّق، فأي تعديل على الملف يُكتشف قبل أن يُقرأ منه بايت واحد"),
    ("Argon2id", "لو فعّلت كلمة سر اشتُقّت بأربعة وستين ميجابايت من الذاكرة وثلاث جولات"),
    ("حماية ويندوز", "المفتاح مربوط بحسابك وبالجهاز عبر DPAPI، ولا يوجد خارجه أبدًا"),
]

STEPS = [
    ("01", "اختر المنصّة", "من الشاشة الرئيسية أو بالأرقام من ١ إلى ٧"),
    ("02", "احفظ حسابك", "سجّل دخولك كالمعتاد وبديل يتعرّف على الحساب"),
    ("03", "بدّل بضغطة", "ولو لعبتك شغّالة يوقف فلا تنقطع مباراتك"),
]


def build():
    made = []
    for key, pal in (("dark", DARK), ("light", LIGHT)):
        banner(pal, f"banner-{key}.png")
        made.append(f"banner-{key}.png")
        for slug, title, sub in SECTIONS:
            section(title, sub, pal, f"sec-{slug}-{key}.png")
            made.append(f"sec-{slug}-{key}.png")
        button("حمّل badeel.exe", "ويندوز ١٠ و١١ · ملف واحد بلا مثبِّت ولا صلاحيات مدير",
               pal, f"dl-exe-{key}.png", True)
        button("الكود المصدري", "اقرأه أو ابنِ النسخة بنفسك وقارنها بالمنشورة",
               pal, f"dl-source-{key}.png", False)
        cards(FEATURES, pal, f"features-{key}.png", 3)
        cards(SECURITY, pal, f"security-{key}.png", 3)
        steps(STEPS, pal, f"usage-{key}.png")
        made += [f"dl-exe-{key}.png", f"dl-source-{key}.png",
                 f"features-{key}.png", f"security-{key}.png", f"usage-{key}.png"]

    total = sum(os.path.getsize(os.path.join(OUT, f)) for f in made)
    print(f"{len(made)} images, {total // 1024} KB total")


if __name__ == "__main__":
    build()
