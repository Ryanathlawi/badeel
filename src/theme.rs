use eframe::egui::{
    self, Color32, FontData, FontDefinitions, FontFamily, Margin, Rect, Stroke, Style, Visuals,
    pos2, vec2,
};

#[derive(Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Accent {
    #[default]
    Teal,
    Azure,
    Indigo,
    Emerald,
    Amber,
    Coral,
    Plum,
    Steel,
}

impl Accent {
    pub const ALL: [Accent; 8] = [
        Accent::Teal,
        Accent::Azure,
        Accent::Indigo,
        Accent::Emerald,
        Accent::Amber,
        Accent::Coral,
        Accent::Plum,
        Accent::Steel,
    ];

    pub fn index(self) -> u8 {
        Self::ALL.iter().position(|a| *a == self).unwrap_or(0) as u8
    }

    pub fn from_index(i: u8) -> Self {
        *Self::ALL.get(i as usize).unwrap_or(&Accent::Teal)
    }

    pub fn name(self, rtl: bool) -> &'static str {
        match (self, rtl) {
            (Accent::Teal, true) => "فيروزي",
            (Accent::Teal, false) => "Teal",
            (Accent::Azure, true) => "سماوي",
            (Accent::Azure, false) => "Azure",
            (Accent::Indigo, true) => "نيلي",
            (Accent::Indigo, false) => "Indigo",
            (Accent::Emerald, true) => "زمردي",
            (Accent::Emerald, false) => "Emerald",
            (Accent::Amber, true) => "عنبري",
            (Accent::Amber, false) => "Amber",
            (Accent::Coral, true) => "مرجاني",
            (Accent::Coral, false) => "Coral",
            (Accent::Plum, true) => "برقوقي",
            (Accent::Plum, false) => "Plum",
            (Accent::Steel, true) => "فولاذي",
            (Accent::Steel, false) => "Steel",
        }
    }

    fn seed(self) -> (Color32, Color32) {
        let rgb = Color32::from_rgb;
        match (self, true) {
            (Accent::Teal, true) => (rgb(42, 182, 166), rgb(112, 178, 232)),
            (Accent::Teal, false) => (rgb(16, 128, 118), rgb(38, 108, 176)),
            (Accent::Azure, true) => (rgb(92, 152, 232), rgb(118, 202, 216)),
            (Accent::Azure, false) => (rgb(34, 100, 188), rgb(20, 132, 154)),
            (Accent::Indigo, true) => (rgb(132, 132, 232), rgb(172, 152, 238)),
            (Accent::Indigo, false) => (rgb(80, 80, 188), rgb(120, 92, 188)),
            (Accent::Emerald, true) => (rgb(74, 188, 132), rgb(150, 202, 122)),
            (Accent::Emerald, false) => (rgb(28, 128, 90), rgb(94, 138, 56)),
            (Accent::Amber, true) => (rgb(214, 166, 90), rgb(224, 142, 102)),
            (Accent::Amber, false) => (rgb(154, 108, 22), rgb(168, 84, 44)),
            (Accent::Coral, true) => (rgb(224, 126, 120), rgb(230, 160, 132)),
            (Accent::Coral, false) => (rgb(178, 64, 62), rgb(168, 96, 52)),
            (Accent::Plum, true) => (rgb(172, 134, 214), rgb(204, 142, 188)),
            (Accent::Plum, false) => (rgb(114, 72, 164), rgb(148, 72, 130)),
            (Accent::Steel, true) => (rgb(130, 152, 180), rgb(112, 178, 188)),
            (Accent::Steel, false) => (rgb(66, 86, 116), rgb(32, 110, 130)),
        }
    }
}

fn shade(c: Color32, f: f32) -> Color32 {
    let m = |x: u8| (x as f32 * f).clamp(0.0, 255.0) as u8;
    Color32::from_rgb(m(c.r()), m(c.g()), m(c.b()))
}

#[derive(Clone, Copy)]
pub struct Palette {
    pub bg: Color32,
    pub bg_deep: Color32,
    pub panel: Color32,
    pub panel_hi: Color32,
    pub line: Color32,
    pub line_hi: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub faint: Color32,
    pub accent: Color32,
    pub accent_2: Color32,
    pub accent_deep: Color32,
    pub live: Color32,
    pub warn: Color32,
    pub danger: Color32,
}

pub fn palette(accent: Accent) -> Palette {
    let (main, second) = accent.seed();
    let deep = shade(main, 0.66);
    Palette {
        bg: Color32::from_rgb(14, 18, 25),
        bg_deep: Color32::from_rgb(9, 12, 17),
        panel: Color32::from_rgba_unmultiplied(23, 29, 39, 236),
        panel_hi: Color32::from_rgba_unmultiplied(main.r(), main.g(), main.b(), 22),
        line: Color32::from_rgba_unmultiplied(255, 255, 255, 18),
        line_hi: Color32::from_rgba_unmultiplied(main.r(), main.g(), main.b(), 110),
        text: Color32::from_rgb(226, 232, 240),
        muted: Color32::from_rgb(148, 160, 178),
        faint: Color32::from_rgb(100, 112, 132),
        accent: main,
        accent_2: second,
        accent_deep: deep,
        live: Color32::from_rgb(64, 198, 148),
        warn: Color32::from_rgb(228, 176, 82),
        danger: Color32::from_rgb(226, 108, 112),
    }
}

pub fn fonts() -> FontDefinitions {
    let mut fonts = FontDefinitions::empty();
    fonts.font_data.insert(
        "arabic".to_owned(),
        std::sync::Arc::new(FontData::from_static(include_bytes!(concat!(
            env!("OUT_DIR"),
            "/arabic-font.bin"
        )))),
    );
    fonts.font_data.insert(
        "latin".to_owned(),
        std::sync::Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/DMMono-Medium.ttf"
        ))),
    );
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        let list = fonts.families.entry(family).or_default();
        list.insert(0, "latin".to_owned());
        list.insert(0, "arabic".to_owned());
    }
    fonts
}

pub fn apply(ctx: &egui::Context, accent: Accent) {
    let pal = palette(accent);
    let mut style = Style {
        visuals: Visuals::dark(),
        ..Default::default()
    };

    let v = &mut style.visuals;
    v.panel_fill = pal.bg;
    v.window_fill = pal.panel;
    v.extreme_bg_color = pal.bg_deep;
    v.faint_bg_color = pal.panel_hi;
    v.override_text_color = Some(pal.text);
    v.window_stroke = Stroke::new(1.0, pal.line);
    v.window_corner_radius = 20.into();
    v.menu_corner_radius = 14.into();
    v.selection.bg_fill = pal.accent.gamma_multiply(0.30);
    v.selection.stroke = Stroke::new(1.0, pal.accent);
    v.window_shadow = egui::epaint::Shadow {
        offset: [0, 18],
        blur: 40,
        spread: 0,
        color: Color32::from_black_alpha(140),
    };
    v.popup_shadow = v.window_shadow;

    for w in [
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.corner_radius = 12.into();
    }
    v.widgets.noninteractive.corner_radius = 12.into();
    v.widgets.inactive.weak_bg_fill = pal.panel_hi;
    v.widgets.inactive.bg_fill = pal.panel_hi;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, pal.line);
    v.widgets.hovered.weak_bg_fill = pal.accent.gamma_multiply(0.22);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, pal.line_hi);
    v.widgets.hovered.expansion = 1.0;
    v.widgets.active.weak_bg_fill = pal.accent.gamma_multiply(0.32);
    v.widgets.active.bg_stroke = Stroke::new(1.0, pal.accent);
    v.widgets.active.expansion = 0.0;

    style.spacing.item_spacing = vec2(10.0, 10.0);
    style.spacing.button_padding = vec2(14.0, 8.0);
    style.spacing.window_margin = Margin::same(18);
    style.spacing.scroll.bar_width = 8.0;
    style.animation_time = 0.14;

    ctx.all_styles_mut(|s| *s = style.clone());
    ctx.set_fonts(fonts());
}

pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
    Color32::from_rgba_unmultiplied(
        f(a.r(), b.r()),
        f(a.g(), b.g()),
        f(a.b(), b.b()),
        f(a.a(), b.a()),
    )
}

pub fn gradient_rect(
    painter: &egui::Painter,
    rect: Rect,
    a: Color32,
    b: Color32,
    radius: f32,
    steps: usize,
) {
    let steps = steps.max(2);
    let h = rect.height() / steps as f32;
    for i in 0..steps {
        let f = i as f32 / (steps - 1) as f32;
        let seg = Rect::from_min_size(
            pos2(rect.left(), rect.top() + h * i as f32),
            vec2(rect.width(), h + 1.0),
        );
        let r = if i == 0 || i + 1 == steps { radius } else { 0.0 };
        painter.rect_filled(seg, r, lerp_color(a, b, f));
    }
}

pub fn hex_points(center: egui::Pos2, r: f32, rot: f32) -> Vec<egui::Pos2> {
    (0..6)
        .map(|i| {
            let a = rot + std::f32::consts::TAU * i as f32 / 6.0 - std::f32::consts::FRAC_PI_2;
            pos2(center.x + r * a.cos(), center.y + r * a.sin())
        })
        .collect()
}

fn hex_outline(painter: &egui::Painter, center: egui::Pos2, r: f32, rot: f32, stroke: Stroke) {
    let mut pts = hex_points(center, r, rot);
    pts.push(pts[0]);
    painter.add(egui::Shape::line(pts, stroke));
}

pub fn draw_backdrop(painter: &egui::Painter, rect: Rect, pal: &Palette, t: f32, alive: bool) {
    painter.rect_filled(rect, 0.0, pal.bg);

    let w = rect.width();
    let h = rect.height();
    let big = w.max(h);
    let m = if alive { 1.0 } else { 0.0 };

    let blobs = [
        (0.14, 0.08, 0.62, pal.accent, 0.11, 0.20),
        (0.90, 0.18, 0.50, pal.accent_2, 0.09, 0.17),
        (0.60, 0.96, 0.56, pal.accent_deep, 0.10, 0.22),
        (0.06, 0.82, 0.38, pal.accent_2, 0.14, 0.15),
    ];
    for (cx, cy, r, col, speed, drift) in blobs {
        let x = cx + m * drift * 0.40 * (t * speed).sin();
        let y = cy + m * drift * 0.32 * (t * speed * 1.27).cos();
        let breath = 1.0 + m * 0.09 * (t * speed * 0.8 + cx * 6.0).sin();
        let center = rect.min + vec2(w * x, h * y);
        let radius = big * r * breath;
        for k in (1..=7).rev() {
            let f = k as f32 / 7.0;
            painter.circle_filled(
                center,
                radius * f,
                col.gamma_multiply(0.024 * (1.0 - f) * (1.0 - f)),
            );
        }
    }

    let step = 84.0;
    let hr = step * 0.30;
    let cols = (w / step).ceil() as i32 + 1;
    let rows = (h / (step * 0.87)).ceil() as i32 + 1;
    for j in 0..rows {
        for i in 0..cols {
            let odd = j % 2 != 0;
            let bx = rect.left() + i as f32 * step + if odd { step * 0.5 } else { 0.0 };
            let by = rect.top() + j as f32 * step * 0.87;
            if bx > rect.right() + step || by > rect.bottom() + step {
                continue;
            }
            let seed = i as f32 * 1.7 + j as f32 * 2.9;
            let wave = ((t * 0.34 * m) - (bx / w) * 2.4 - (by / h) * 1.2 + seed * 0.02).sin();
            let lit = (wave * 0.5 + 0.5).powi(4);
            let a = 0.030 + 0.13 * lit;
            let col = if (i + j) % 4 == 0 { pal.accent_2 } else { pal.accent };
            hex_outline(
                painter,
                pos2(bx, by),
                hr * (0.92 + 0.10 * lit),
                0.0,
                Stroke::new(1.0, col.gamma_multiply(a)),
            );
            if lit > 0.72 {
                painter.circle_filled(
                    pos2(bx, by),
                    1.5,
                    col.gamma_multiply(0.22 * (lit - 0.72) / 0.28),
                );
            }
        }
    }

    let sweep = (t * 0.055 * m).rem_euclid(1.0);
    let sy = rect.top() - 60.0 + sweep * (h + 120.0);
    for k in 0..6 {
        let f = k as f32 / 5.0;
        painter.rect_filled(
            Rect::from_min_size(pos2(rect.left(), sy - 30.0 + f * 60.0), vec2(w, 11.0)),
            0.0,
            pal.accent_2.gamma_multiply(0.014 * (1.0 - (f - 0.5).abs() * 2.0).max(0.0) * m),
        );
    }
}

pub use crate::motion::{back_out, ease_out};

pub fn pulse(t: f32, speed: f32) -> f32 {
    0.5 + 0.5 * (t * speed).sin()
}
