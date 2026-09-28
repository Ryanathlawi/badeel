use eframe::egui::{self, Align2, Color32, FontId, Rect, Sense, StrokeKind, pos2, vec2};

use crate::i18n::Lang;
use crate::motion::Motion;
use crate::theme::{Palette, ease_out, lerp_color, pulse};
use crate::ui;

pub struct Card {
    tag: (&'static str, &'static str),
    title: (&'static str, &'static str),
    body: (&'static str, &'static str),
    glyph: u8,
    hue: u8,
}

pub const CARDS: [Card; 7] = [
    Card {
        tag: ("المحرّك", "ENGINE"),
        title: ("تبديل ذرّي لا يترك أثرًا", "An atomic switch"),
        body: (
            "يغلق المنصّة، يحفظ جلستك الحالية، يركّب الحساب الجديد، ثم يشغّلها، وإن تعثّرت أي خطوة رجع كل ملف إلى مكانه بالضبط",
            "It closes the platform, saves your current session, restores the account you picked, then starts it again. If any step fails, every file goes back exactly where it was.",
        ),
        glyph: 0,
        hue: 0,
    },
    Card {
        tag: ("الأمان", "SECURITY"),
        title: ("AES-256 مربوط بجهازك", "AES-256, bound to your PC"),
        body: (
            "كل جلسة تُحفظ مشفّرة بمفتاح مشتقّ من حماية ويندوز لحسابك، ونسخة مسروقة إلى جهاز آخر لا تُفتح أصلًا",
            "Every session is stored encrypted under a key derived from your Windows account protection. A copy stolen to another machine simply will not open.",
        ),
        glyph: 1,
        hue: 2,
    },
    Card {
        tag: ("الخصوصية", "PRIVACY"),
        title: ("ما نشوف كلمة سرّك", "We never see your password"),
        body: (
            "بديل لا يطلب كلمة سر المنصّة ولا يقرأها ولا يخزّنها، وإنما يتعامل مع ملفات الجلسة نفسها التي أنشأتها المنصّة",
            "badeel never asks for, reads or stores a platform password. It moves the very session files the platform itself created.",
        ),
        glyph: 2,
        hue: 1,
    },
    Card {
        tag: ("المنصّات", "PLATFORMS"),
        title: ("كل منصّاتك بنافذة واحدة", "Every launcher, one window"),
        body: (
            "ستيم، باتل نت، رايوت، إيبك، يوبيسوفت، روكستار، وجوج — كل واحدة بحساباتها وصورها وذاكرة استخدامها",
            "Steam, Battle.net, Riot, Epic, Ubisoft, Rockstar and GOG — each with its own accounts, pictures and usage memory.",
        ),
        glyph: 3,
        hue: 1,
    },
    Card {
        tag: ("الاكتشاف", "DISCOVERY"),
        title: ("يلقى تثبيتك مهما كان مكانه", "Finds your install anywhere"),
        body: (
            "لا مسارات مكتوبة مسبقًا، يقرأ سجل ويندوز وملفات المنصّات ليعرف أين ثُبّتت، ولو كانت على قرص آخر",
            "No hardcoded paths. It reads the Windows registry and the platforms' own files to locate them, even on another drive.",
        ),
        glyph: 4,
        hue: 0,
    },
    Card {
        tag: ("التحديثات", "UPDATES"),
        title: ("يحدّث نفسه بنفسه", "It updates itself"),
        body: (
            "أي إصدار جديد يصلك كتنبيه داخل التطبيق، وينزّل ويثبّت بضغطة واحدة، مع الاحتفاظ بالنسخة السابقة",
            "A new release reaches you as a notice inside the app, downloads and installs in one click, and keeps the previous build.",
        ),
        glyph: 5,
        hue: 3,
    },
    Card {
        tag: ("بلا تتبّع", "NO TRACKING"),
        title: ("ما يطلع منه شيء", "Nothing leaves your PC"),
        body: (
            "بلا حسابات وبلا خوادم وبلا تحليلات، وما يكلّم إلا GitHub يسأله عن التحديثات ويجيب منه اللوحة، وتقدر توقف الاثنين",
            "No accounts, no servers, no analytics. It only ever talks to GitHub, to check for updates and fetch the board, and you can turn both off.",
        ),
        glyph: 6,
        hue: 2,
    },
];

pub const DWELL: f32 = 6.5;
const SLIDE: f32 = 26.0;

pub struct Showcase {
    pub(crate) i: usize,
    pub(crate) prev: usize,
    pub(crate) t: f32,
    pub(crate) hold: f32,
}

impl Default for Showcase {
    fn default() -> Self {
        Self {
            i: 0,
            prev: 0,
            t: 1.0,
            hold: 0.0,
        }
    }
}

impl Showcase {
    pub fn tick(&mut self, dt: f32, mo: Motion) {
        self.tick_n(dt, mo, CARDS.len(), DWELL);
    }

    /// الدوران نفسه لأي عدد من البطاقات، وبه تدور لوحة المنصّة أيضًا
    /// فتتحرّك بطاقتها كما تتحرّك بطاقة الشاشة الأولى لا بحركة غريبة عنها
    pub fn tick_n(&mut self, dt: f32, mo: Motion, n: usize, dwell: f32) {
        if self.t < 1.0 {
            self.t = if mo.enabled { (self.t + dt * 1.9).min(1.0) } else { 1.0 };
            return;
        }
        if n < 2 {
            self.hold = 0.0;
            return;
        }
        self.hold += dt;
        if self.hold >= dwell {
            self.go_n((self.i + 1) % n, mo, n);
        }
    }

    pub fn go(&mut self, to: usize, mo: Motion) {
        self.go_n(to, mo, CARDS.len());
    }

    fn go_n(&mut self, to: usize, mo: Motion, n: usize) {
        if to == self.i || n == 0 {
            return;
        }
        self.prev = self.i;
        self.i = to.min(n - 1);
        self.hold = 0.0;
        self.t = if mo.enabled { 0.0 } else { 1.0 };
    }

    pub fn moving(&self) -> bool {
        self.t < 1.0
    }
}

fn hue(pal: &Palette, k: u8) -> Color32 {
    match k {
        0 => pal.accent,
        1 => pal.accent_2,
        2 => pal.live,
        _ => pal.warn,
    }
}

fn arc(center: egui::Pos2, r: f32, from: f32, to: f32, steps: usize) -> Vec<egui::Pos2> {
    (0..=steps)
        .map(|i| {
            let a = from + (to - from) * i as f32 / steps as f32;
            pos2(center.x + r * a.cos(), center.y + r * a.sin())
        })
        .collect()
}

fn glyph(p: &egui::Painter, rect: Rect, kind: u8, col: Color32) {
    let c = rect.center();
    let s = rect.width().min(rect.height()) * 0.5;
    let w = (s * 0.14).max(1.6);
    let stroke = egui::Stroke::new(w, col);
    match kind {
        0 => {
            let pts = vec![
                pos2(c.x + s * 0.18, c.y - s * 0.88),
                pos2(c.x - s * 0.62, c.y + s * 0.10),
                pos2(c.x - s * 0.04, c.y + s * 0.10),
                pos2(c.x - s * 0.18, c.y + s * 0.88),
                pos2(c.x + s * 0.62, c.y - s * 0.14),
                pos2(c.x + s * 0.04, c.y - s * 0.14),
            ];
            p.add(egui::Shape::convex_polygon(
                pts,
                col.gamma_multiply(0.22),
                stroke,
            ));
        }
        1 => {
            let body = Rect::from_center_size(
                pos2(c.x, c.y + s * 0.26),
                vec2(s * 1.30, s * 1.00),
            );
            p.rect_filled(body, s * 0.22, col.gamma_multiply(0.22));
            p.rect_stroke(body, s * 0.22, stroke, StrokeKind::Inside);
            p.add(egui::Shape::line(
                arc(
                    pos2(c.x, c.y - s * 0.24),
                    s * 0.46,
                    std::f32::consts::PI,
                    std::f32::consts::TAU,
                    14,
                ),
                stroke,
            ));
            p.circle_filled(pos2(c.x, c.y + s * 0.24), w * 0.9, col);
        }
        2 => {
            p.circle_stroke(pos2(c.x - s * 0.34, c.y), s * 0.36, stroke);
            p.line_segment(
                [pos2(c.x + s * 0.02, c.y), pos2(c.x + s * 0.82, c.y)],
                stroke,
            );
            p.line_segment(
                [
                    pos2(c.x + s * 0.52, c.y),
                    pos2(c.x + s * 0.52, c.y + s * 0.30),
                ],
                stroke,
            );
            p.line_segment(
                [
                    pos2(c.x - s * 0.82, c.y - s * 0.78),
                    pos2(c.x + s * 0.82, c.y + s * 0.78),
                ],
                egui::Stroke::new(w, col),
            );
        }
        3 => {
            for i in 0..5 {
                let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::TAU / 5.0;
                let q = pos2(c.x + s * 0.66 * a.cos(), c.y + s * 0.66 * a.sin());
                p.circle_filled(q, s * 0.20, col);
            }
            p.circle_stroke(c, s * 0.92, egui::Stroke::new(w * 0.7, col.gamma_multiply(0.45)));
        }
        4 => {
            for k in 1..=3 {
                let r = s * 0.30 * k as f32;
                p.add(egui::Shape::line(
                    arc(
                        pos2(c.x - s * 0.30, c.y + s * 0.42),
                        r,
                        -std::f32::consts::FRAC_PI_2,
                        0.0,
                        12,
                    ),
                    egui::Stroke::new(w * 0.85, col.gamma_multiply(1.0 - k as f32 * 0.22)),
                ));
            }
            p.circle_filled(pos2(c.x - s * 0.30, c.y + s * 0.42), w * 1.2, col);
        }
        5 => {
            p.line_segment(
                [pos2(c.x, c.y - s * 0.86), pos2(c.x, c.y + s * 0.22)],
                stroke,
            );
            p.add(egui::Shape::line(
                vec![
                    pos2(c.x - s * 0.40, c.y - s * 0.18),
                    pos2(c.x, c.y + s * 0.26),
                    pos2(c.x + s * 0.40, c.y - s * 0.18),
                ],
                stroke,
            ));
            p.line_segment(
                [
                    pos2(c.x - s * 0.74, c.y + s * 0.74),
                    pos2(c.x + s * 0.74, c.y + s * 0.74),
                ],
                stroke,
            );
        }
        _ => {
            p.add(egui::Shape::line(
                arc(
                    pos2(c.x, c.y + s * 0.62),
                    s * 0.86,
                    std::f32::consts::PI + 0.5,
                    std::f32::consts::TAU - 0.5,
                    16,
                ),
                stroke,
            ));
            p.add(egui::Shape::line(
                arc(
                    pos2(c.x, c.y - s * 0.62),
                    s * 0.86,
                    0.5,
                    std::f32::consts::PI - 0.5,
                    16,
                ),
                stroke,
            ));
            p.circle_filled(c, s * 0.22, col);
            p.line_segment(
                [
                    pos2(c.x - s * 0.80, c.y - s * 0.80),
                    pos2(c.x + s * 0.80, c.y + s * 0.80),
                ],
                stroke,
            );
        }
    }
}

fn lay(
    p: &egui::Painter,
    rtl: bool,
    text: &str,
    size: f32,
    color: Color32,
    width: f32,
) -> std::sync::Arc<egui::Galley> {
    let mut job =
        egui::text::LayoutJob::simple(text.to_owned(), FontId::proportional(size), color, width);
    job.halign = if rtl {
        egui::Align::RIGHT
    } else {
        egui::Align::LEFT
    };
    p.layout_job(job)
}

fn face(
    p: &egui::Painter,
    area: Rect,
    pal: &Palette,
    rtl: bool,
    card: &Card,
    alpha: f32,
    dx: f32,
    t: f32,
) {
    if alpha <= 0.01 {
        return;
    }
    let col = hue(pal, card.hue).gamma_multiply(alpha);
    let w = area.width();
    let title = if rtl { card.title.0 } else { card.title.1 };
    let body = if rtl { card.body.0 } else { card.body.1 };
    let tg = lay(p, rtl, title, 17.5, pal.text.gamma_multiply(alpha), w);
    let bg = lay(p, rtl, body, 12.5, pal.muted.gamma_multiply(alpha * 0.95), w);

    let box_side = 58.0;
    let total = box_side + 18.0 + 20.0 + 14.0 + tg.size().y + 10.0 + bg.size().y;
    let top = area.top() + ((area.height() - total) * 0.40).max(0.0);
    let x = if rtl { area.right() } else { area.left() } + dx;
    let anchor = |w: f32| if rtl { x - w } else { x };

    let gbox = Rect::from_min_size(pos2(anchor(box_side), top), vec2(box_side, box_side));
    let breath = 0.5 + 0.5 * pulse(t, 0.9);
    p.rect_filled(
        gbox,
        18.0,
        hue(pal, card.hue).gamma_multiply((0.10 + 0.05 * breath) * alpha),
    );
    p.rect_stroke(
        gbox,
        18.0,
        egui::Stroke::new(1.0, hue(pal, card.hue).gamma_multiply(0.35 * alpha)),
        StrokeKind::Inside,
    );
    glyph(p, gbox.shrink(16.0), card.glyph, col);

    let tag = if rtl { card.tag.0 } else { card.tag.1 };
    let tgal = p.layout_no_wrap(tag.to_owned(), FontId::proportional(9.5), col);
    let tw = tgal.size().x + 20.0;
    let trect = Rect::from_min_size(pos2(anchor(tw), gbox.bottom() + 18.0), vec2(tw, 20.0));
    p.rect_filled(trect, 999.0, col.gamma_multiply(0.14));
    p.text(
        trect.center(),
        Align2::CENTER_CENTER,
        tag,
        FontId::proportional(9.5),
        col.gamma_multiply(0.95),
    );

    let ty = trect.bottom() + 14.0;
    p.galley(pos2(x, ty), tg.clone(), pal.text.gamma_multiply(alpha));
    p.galley(
        pos2(x, ty + tg.size().y + 10.0),
        bg,
        pal.muted.gamma_multiply(alpha),
    );
}

fn stat(p: &egui::Painter, rect: Rect, faint: Color32, big: &str, small: &str, col: Color32) {
    p.text(
        pos2(rect.center().x, rect.top() + 9.0),
        Align2::CENTER_CENTER,
        big,
        FontId::proportional(13.5),
        col,
    );
    p.text(
        pos2(rect.center().x, rect.top() + 26.0),
        Align2::CENTER_CENTER,
        small,
        FontId::proportional(9.5),
        faint,
    );
}

pub fn strip(
    ui: &mut egui::Ui,
    rect: Rect,
    pal: &Palette,
    rtl: bool,
    sc: &Showcase,
    t: f32,
    alpha: f32,
) -> Option<usize> {
    let mut clicked = None;
    {
        let p = ui.painter();
        p.rect_filled(rect, 16.0, pal.panel.gamma_multiply(0.50 * alpha));
        p.rect_stroke(
            rect,
            16.0,
            egui::Stroke::new(1.0, pal.line.gamma_multiply(0.85 * alpha)),
            StrokeKind::Inside,
        );
    }

    let dots_w = 26.0 + CARDS.len() as f32 * 15.0;
    let pad = 16.0;
    let body = if rtl {
        Rect::from_min_max(
            pos2(rect.left() + dots_w, rect.top() + pad),
            pos2(rect.right() - pad, rect.bottom() - pad),
        )
    } else {
        Rect::from_min_max(
            pos2(rect.left() + pad, rect.top() + pad),
            pos2(rect.right() - dots_w, rect.bottom() - pad),
        )
    };

    let dir = if rtl { -1.0 } else { 1.0 };
    let leaving = 1.0 - (sc.t / 0.45).clamp(0.0, 1.0);
    let arriving = ((sc.t - 0.35) / 0.65).clamp(0.0, 1.0);
    {
        let p = ui.painter().with_clip_rect(rect);
        if leaving > 0.001 && sc.prev != sc.i {
            row(&p, body, pal, rtl, &CARDS[sc.prev], leaving * alpha, -dir * 22.0 * (1.0 - leaving), t);
        }
        row(
            &p,
            body,
            pal,
            rtl,
            &CARDS[sc.i],
            arriving * alpha,
            dir * 22.0 * (1.0 - ease_out(arriving)),
            t,
        );
    }

    let n = CARDS.len();
    let gap = 15.0;
    let start = if rtl {
        rect.left() + 17.0
    } else {
        rect.right() - 17.0
    };
    let step = if rtl { gap } else { -gap };
    let dy = rect.center().y;
    for i in 0..n {
        let cx = start + step * i as f32;
        let hit = Rect::from_center_size(pos2(cx, dy), vec2(gap, 26.0));
        let r = ui.interact(hit, ui.id().with(("strip", i)), Sense::click());
        let hot = ui
            .ctx()
            .animate_bool_with_time(r.id, r.hovered(), crate::motion::hover_time());
        if r.clicked() {
            clicked = Some(i);
        }
        let on = i == sc.i;
        let col = if on {
            hue(pal, CARDS[i].hue)
        } else {
            lerp_color(pal.line_hi, pal.text, 0.20 * hot)
        };
        let p = ui.painter();
        p.circle_filled(
            pos2(cx, dy),
            if on { 4.0 } else { 2.5 + 0.8 * hot },
            col.gamma_multiply(alpha),
        );
        if on {
            let frac = (sc.hold / DWELL).clamp(0.0, 1.0);
            p.add(egui::Shape::line(
                arc(
                    pos2(cx, dy),
                    7.0,
                    -std::f32::consts::FRAC_PI_2,
                    -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * frac,
                    18,
                ),
                egui::Stroke::new(1.4, col.gamma_multiply(0.85 * alpha)),
            ));
        }
    }
    clicked
}

fn row(
    p: &egui::Painter,
    area: Rect,
    pal: &Palette,
    rtl: bool,
    card: &Card,
    alpha: f32,
    dx: f32,
    t: f32,
) {
    if alpha <= 0.01 {
        return;
    }
    let col = hue(pal, card.hue);
    let side = 44.0;
    let gx = if rtl {
        area.right() - side + dx
    } else {
        area.left() + dx
    };
    let gbox = Rect::from_min_size(pos2(gx, area.center().y - side * 0.5), vec2(side, side));
    let breath = 0.5 + 0.5 * pulse(t, 0.9);
    p.rect_filled(
        gbox,
        14.0,
        col.gamma_multiply((0.11 + 0.05 * breath) * alpha),
    );
    glyph(p, gbox.shrink(12.0), card.glyph, col.gamma_multiply(alpha));

    let x = if rtl {
        gbox.left() - 14.0
    } else {
        gbox.right() + 14.0
    };
    let w = (area.width() - side - 14.0).max(80.0);
    let tag = if rtl { card.tag.0 } else { card.tag.1 };
    let tgal = p.layout_no_wrap(tag.to_owned(), FontId::proportional(9.0), col);
    let tw = tgal.size().x + 16.0;
    let trect = Rect::from_min_size(
        pos2(if rtl { x - tw } else { x }, area.top()),
        vec2(tw, 18.0),
    );
    p.rect_filled(trect, 999.0, col.gamma_multiply(0.14 * alpha));
    p.text(
        trect.center(),
        Align2::CENTER_CENTER,
        tag,
        FontId::proportional(9.0),
        col.gamma_multiply(0.95 * alpha),
    );

    let title = if rtl { card.title.0 } else { card.title.1 };
    let body = if rtl { card.body.0 } else { card.body.1 };
    let tg = lay(p, rtl, title, 14.5, pal.text.gamma_multiply(alpha), w);
    p.galley(
        pos2(x, trect.bottom() + 6.0),
        tg.clone(),
        pal.text.gamma_multiply(alpha),
    );
    let bg = lay(p, rtl, body, 11.5, pal.muted.gamma_multiply(alpha), w);
    p.galley(
        pos2(x, trect.bottom() + 6.0 + tg.size().y + 4.0),
        bg,
        pal.muted.gamma_multiply(alpha),
    );
}

pub fn panel(
    ui: &mut egui::Ui,
    rect: Rect,
    pal: &Palette,
    lang: Lang,
    s: &Showcase,
    t: f32,
    alpha: f32,
) -> Option<usize> {
    let rtl = lang.rtl();
    ui::panel_card(ui.painter(), rect, pal);
    let inner = rect.shrink2(vec2(20.0, 18.0));
    let mut clicked = None;

    let p = ui.painter();
    let eyebrow = if rtl { "لماذا بديل؟" } else { "WHY BADEEL" };
    let ex = if rtl { inner.right() } else { inner.left() };
    p.circle_filled(
        pos2(
            if rtl { ex - 3.0 } else { ex + 3.0 },
            inner.top() + 7.0,
        ),
        3.0,
        pal.accent.gamma_multiply(alpha),
    );
    p.text(
        pos2(if rtl { ex - 14.0 } else { ex + 14.0 }, inner.top() + 7.0),
        if rtl {
            Align2::RIGHT_CENTER
        } else {
            Align2::LEFT_CENTER
        },
        eyebrow,
        FontId::proportional(10.0),
        pal.faint.gamma_multiply(alpha),
    );

    let dots_y = inner.bottom() - 74.0;
    let body = Rect::from_min_max(
        pos2(inner.left(), inner.top() + 30.0),
        pos2(inner.right(), dots_y - 22.0),
    );

    let dir = if rtl { -1.0 } else { 1.0 };
    let leaving = 1.0 - (s.t / 0.45).clamp(0.0, 1.0);
    if leaving > 0.001 && s.prev != s.i {
        face(
            p,
            body,
            pal,
            rtl,
            &CARDS[s.prev],
            leaving * alpha,
            -dir * SLIDE * (1.0 - leaving) * 0.7,
            t,
        );
    }
    let arriving = ((s.t - 0.35) / 0.65).clamp(0.0, 1.0);
    face(
        p,
        body,
        pal,
        rtl,
        &CARDS[s.i],
        arriving * alpha,
        dir * SLIDE * (1.0 - ease_out(arriving)),
        t,
    );
    let n = CARDS.len();
    let gap = 16.0;
    let start = if rtl {
        inner.right() - 5.0
    } else {
        inner.left() + 5.0
    };
    for i in 0..n {
        let cx = start + dir * i as f32 * gap;
        let hit = Rect::from_center_size(pos2(cx, dots_y), vec2(gap, 22.0));
        let r = ui.interact(hit, ui.id().with(("dot", i)), Sense::click());
        let hot = ui
            .ctx()
            .animate_bool_with_time(r.id, r.hovered(), crate::motion::hover_time());
        if r.clicked() {
            clicked = Some(i);
        }
        let on = i == s.i;
        let col = if on {
            hue(pal, CARDS[i].hue)
        } else {
            lerp_color(pal.line_hi, pal.text, 0.20 * hot)
        };
        ui.painter().circle_filled(
            pos2(cx, dots_y),
            if on { 4.2 } else { 2.6 + 0.8 * hot },
            col.gamma_multiply(alpha),
        );
        if on {
            let frac = (s.hold / DWELL).clamp(0.0, 1.0);
            ui.painter().circle_stroke(
                pos2(cx, dots_y),
                7.0,
                egui::Stroke::new(1.4, col.gamma_multiply(0.22 * alpha)),
            );
            ui.painter().add(egui::Shape::line(
                arc(
                    pos2(cx, dots_y),
                    7.0,
                    -std::f32::consts::FRAC_PI_2,
                    -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * frac,
                    18,
                ),
                egui::Stroke::new(1.4, col.gamma_multiply(0.85 * alpha)),
            ));
        }
    }

    let p = ui.painter();
    let line_y = inner.bottom() - 46.0;
    p.line_segment(
        [
            pos2(inner.left(), line_y),
            pos2(inner.right(), line_y),
        ],
        egui::Stroke::new(1.0, pal.line.gamma_multiply(alpha)),
    );

    let cellw = inner.width() / 3.0;
    let cells: [(&str, &str, Color32); 3] = [
        (
            "7",
            if rtl { "منصّات" } else { "platforms" },
            pal.accent,
        ),
        ("AES-256", if rtl { "تشفير" } else { "encryption" }, pal.live),
        (
            "0",
            if rtl { "كلمات سر" } else { "passwords" },
            pal.accent_2,
        ),
    ];
    for (i, (big, small, col)) in cells.into_iter().enumerate() {
        let r = Rect::from_min_size(
            pos2(inner.left() + cellw * i as f32, line_y + 12.0),
            vec2(cellw, 34.0),
        );
        stat(
            p,
            r,
            pal.faint.gamma_multiply(alpha),
            big,
            small,
            col.gamma_multiply(alpha),
        );
        if i > 0 {
            p.line_segment(
                [
                    pos2(r.left(), line_y + 16.0),
                    pos2(r.left(), line_y + 40.0),
                ],
                egui::Stroke::new(1.0, pal.line.gamma_multiply(0.7 * alpha)),
            );
        }
    }

    clicked
}
