use eframe::egui::{self, Align2, Color32, FontId, Rect, Sense, StrokeKind, Vec2, pos2, vec2};

use crate::app::ToastKind;
use crate::core::store::Account;
use crate::i18n::Lang;
use crate::theme::{Palette, back_out, ease_out, gradient_rect, lerp_color, pulse};

pub fn platform_image(id: &str) -> egui::ImageSource<'static> {
    match id {
        "steam" => egui::include_image!("../assets/icons/steam.svg"),
        "battlenet" => egui::include_image!("../assets/icons/battledotnet.svg"),
        "riot" => egui::include_image!("../assets/icons/riotgames.svg"),
        "epic" => egui::include_image!("../assets/icons/epicgames.svg"),
        "ubisoft" => egui::include_image!("../assets/icons/ubisoft.svg"),
        "rockstar" => egui::include_image!("../assets/icons/rockstar.svg"),
        _ => egui::include_image!("../assets/icons/gog.svg"),
    }
}

const ICON_RASTER: u32 = 192;

pub fn platform_icon(ui: &mut egui::Ui, rect: Rect, id: &str, color: Color32, alpha: f32) {
    if alpha <= 0.01 || rect.width() < 1.0 {
        return;
    }
    let poll = platform_image(id).load(
        ui.ctx(),
        egui::TextureOptions::LINEAR,
        egui::SizeHint::Size {
            width: ICON_RASTER,
            height: ICON_RASTER,
            maintain_aspect_ratio: true,
        },
    );
    if let Ok(egui::load::TexturePoll::Ready { texture }) = poll {
        let side = rect.width().min(rect.height());
        let square = Rect::from_center_size(rect.center(), vec2(side, side));
        ui.painter().image(
            texture.id,
            square,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            color.gamma_multiply(alpha),
        );
    }
}

pub fn file_texture(ui: &egui::Ui, path: &str) -> Option<egui::TextureId> {
    let clean = path.replace('\\', "/");
    let uri = format!("file:///{}", clean.trim_start_matches('/'));
    let poll = egui::ImageSource::Uri(uri.into()).load(
        ui.ctx(),
        egui::TextureOptions::LINEAR,
        egui::SizeHint::Size {
            width: 256,
            height: 256,
            maintain_aspect_ratio: true,
        },
    );
    match poll {
        Ok(egui::load::TexturePoll::Ready { texture }) => Some(texture.id),
        _ => None,
    }
}

pub fn circle_image(
    painter: &egui::Painter,
    center: egui::Pos2,
    radius: f32,
    tex: egui::TextureId,
    tint: Color32,
) {
    let n = 56;
    let mut mesh = egui::epaint::Mesh::with_texture(tex);
    mesh.vertices.push(egui::epaint::Vertex {
        pos: center,
        uv: pos2(0.5, 0.5),
        color: tint,
    });
    for i in 0..=n {
        let a = std::f32::consts::TAU * i as f32 / n as f32;
        let (c, sn) = (a.cos(), a.sin());
        mesh.vertices.push(egui::epaint::Vertex {
            pos: pos2(center.x + c * radius, center.y + sn * radius),
            uv: pos2(0.5 + c * 0.5, 0.5 + sn * 0.5),
            color: tint,
        });
    }
    for i in 1..=n as u32 {
        mesh.indices.extend([0, i, i + 1]);
    }
    painter.add(egui::Shape::mesh(mesh));
}

fn hash_hue(seed: &str) -> f32 {
    let mut h: u32 = 2166136261;
    for b in seed.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(16777619);
    }
    (h % 360) as f32 / 360.0
}

fn hsv(h: f32, s: f32, v: f32) -> Color32 {
    let i = (h * 6.0).floor();
    let f = h * 6.0 - i;
    let (p, q, t) = (v * (1.0 - s), v * (1.0 - f * s), v * (1.0 - (1.0 - f) * s));
    let (r, g, b) = match (i as i32) % 6 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    Color32::from_rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}

#[allow(clippy::too_many_arguments)]
pub fn avatar(
    ui: &mut egui::Ui,
    center: egui::Pos2,
    radius: f32,
    acc: &Account,
    platform_id: &str,
    pal: &Palette,
    is_live: bool,
    t: f32,
    alpha: f32,
) {
    if alpha <= 0.01 {
        return;
    }
    let tex = acc
        .avatar
        .as_deref()
        .filter(|p| !p.is_empty())
        .and_then(|p| file_texture(ui, p));

    if is_live {
        ui.painter().circle_stroke(
            center,
            radius + 5.0 + 2.5 * pulse(t, 1.6),
            egui::Stroke::new(1.4, pal.live.gamma_multiply(0.45 * alpha)),
        );
    }

    match tex {
        Some(id) => {
            circle_image(
                ui.painter(),
                center,
                radius,
                id,
                Color32::WHITE.gamma_multiply(alpha),
            );
        }
        None => {
            let h = hash_hue(&acc.id);
            let a1 = hsv(h, 0.58, 0.88);
            let a2 = hsv((h + 0.10) % 1.0, 0.70, 0.55);
            let p = ui.painter();
            p.circle_filled(center, radius, a2.gamma_multiply(alpha));
            for k in 0..8 {
                let f = k as f32 / 8.0;
                p.circle_filled(
                    pos2(center.x, center.y - radius * 0.22 * f),
                    radius * (1.0 - f * 0.55),
                    a1.gamma_multiply(alpha * 0.16),
                );
            }
            platform_icon(
                ui,
                Rect::from_center_size(center, vec2(radius * 1.0, radius * 1.0)),
                platform_id,
                Color32::WHITE,
                0.62 * alpha,
            );
        }
    }
    ui.painter().circle_stroke(
        center,
        radius,
        egui::Stroke::new(
            1.0,
            if is_live {
                pal.live.gamma_multiply(0.75 * alpha)
            } else {
                pal.line_hi.gamma_multiply(0.55 * alpha)
            },
        ),
    );
}

pub fn search_box(ui: &mut egui::Ui, pal: &Palette, rtl: bool, text: &mut String, hint: &str) {
    let h = 38.0;
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 12.0, pal.panel_hi.gamma_multiply(0.35));
    p.rect_stroke(
        rect,
        12.0,
        egui::Stroke::new(1.0, pal.line.gamma_multiply(0.7)),
        StrokeKind::Inside,
    );
    let ic = if rtl {
        pos2(rect.right() - 20.0, rect.center().y)
    } else {
        pos2(rect.left() + 20.0, rect.center().y)
    };
    p.circle_stroke(
        pos2(ic.x, ic.y - 1.0),
        5.5,
        egui::Stroke::new(1.5, pal.faint),
    );
    p.line_segment(
        [pos2(ic.x + 4.0, ic.y + 3.0), pos2(ic.x + 8.0, ic.y + 7.0)],
        egui::Stroke::new(1.5, pal.faint),
    );
    let inner = if rtl {
        Rect::from_min_size(
            pos2(rect.left() + 14.0, rect.top() + 7.0),
            vec2(rect.width() - 46.0, h - 14.0),
        )
    } else {
        Rect::from_min_size(
            pos2(rect.left() + 38.0, rect.top() + 7.0),
            vec2(rect.width() - 52.0, h - 14.0),
        )
    };
    ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
        ui.with_layout(
            if rtl {
                egui::Layout::right_to_left(egui::Align::Center)
            } else {
                egui::Layout::left_to_right(egui::Align::Center)
            },
            |ui| {
                ui.add(
                    egui::TextEdit::singleline(text)
                        .hint_text(hint)
                        .desired_width(f32::INFINITY)
                        .frame(egui::Frame::NONE),
                );
            },
        );
    });
}

pub fn brand_mark(painter: &egui::Painter, rect: Rect, pal: &Palette, t: f32) {
    let breath = pulse(t, 1.0);
    gradient_rect(
        painter,
        rect,
        pal.accent.gamma_multiply(0.30 + 0.10 * breath),
        pal.accent_2.gamma_multiply(0.18 + 0.08 * breath),
        rect.width() * 0.30,
        14,
    );
    painter.rect_stroke(
        rect,
        rect.width() * 0.30,
        egui::Stroke::new(1.0, pal.accent.gamma_multiply(0.45 + 0.30 * breath)),
        StrokeKind::Inside,
    );
    let c = rect.center();
    let r = rect.width() * 0.23;
    for (i, sign) in [-1.0_f32, 1.0].into_iter().enumerate() {
        let y = c.y + sign * rect.height() * 0.13;
        let phase = pulse(t + i as f32 * 1.4, 1.5);
        let shift = rect.width() * 0.045 * sign * phase;
        let (x0, x1) = (c.x - r + shift, c.x + r + shift);
        let w = rect.width() * 0.055;
        let col = Color32::WHITE;
        painter.line_segment([pos2(x0, y), pos2(x1, y)], egui::Stroke::new(w, col));
        let (tip, dir) = if sign < 0.0 { (x1, -1.0) } else { (x0, 1.0) };
        let a = rect.width() * 0.10;
        painter.line_segment(
            [pos2(tip, y), pos2(tip + dir * a, y - a * 0.85)],
            egui::Stroke::new(w, col),
        );
        painter.line_segment(
            [pos2(tip, y), pos2(tip + dir * a, y + a * 0.85)],
            egui::Stroke::new(w, col),
        );
    }
}

pub fn icon_button(ui: &mut egui::Ui, pal: &Palette, label: &str, size: f32) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(size.max(36.0), 36.0), Sense::click());
    let hot = ui.ctx().animate_bool_with_time(resp.id, resp.hovered(), crate::motion::hover_time());
    let p = ui.painter();
    p.rect_filled(rect, 11.0, pal.panel_hi.gamma_multiply(0.5 + 1.0 * hot));
    p.rect_stroke(
        rect,
        11.0,
        egui::Stroke::new(1.0, pal.line.gamma_multiply(0.7 + 0.9 * hot)),
        StrokeKind::Inside,
    );
    p.text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(12.0),
        pal.text.gamma_multiply(0.72 + 0.28 * hot),
    );
    resp
}

pub fn pill(ui: &mut egui::Ui, pal: &Palette, text: &str, color: Color32, t: f32) -> egui::Response {
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_string(), FontId::proportional(11.5), color);
    let (rect, resp) = ui.allocate_exact_size(vec2(galley.size().x + 36.0, 32.0), Sense::click());
    let hot = ui.ctx().animate_bool_with_time(resp.id, resp.hovered(), crate::motion::hover_time());
    let glow = 0.16 + 0.10 * pulse(t, 2.2) + 0.25 * hot;
    let p = ui.painter();
    p.rect_filled(rect, 999.0, color.gamma_multiply(glow));
    p.rect_stroke(
        rect,
        999.0,
        egui::Stroke::new(1.0, color.gamma_multiply(0.5 + 0.45 * hot)),
        StrokeKind::Inside,
    );
    p.circle_filled(
        pos2(rect.left() + 15.0, rect.center().y),
        3.2 + 0.8 * pulse(t, 2.6),
        color,
    );
    p.text(
        pos2(rect.center().x + 7.0, rect.center().y),
        Align2::CENTER_CENTER,
        text,
        FontId::proportional(11.5),
        color,
    );
    let _ = pal;
    resp
}

pub fn solid_button(ui: &mut egui::Ui, pal: &Palette, text: &str, color: Color32) -> egui::Response {
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_string(), FontId::proportional(13.0), pal.text);
    let (rect, resp) = ui.allocate_exact_size(vec2(galley.size().x + 38.0, 40.0), Sense::click());
    let hot = ui.ctx().animate_bool_with_time(resp.id, resp.hovered(), crate::motion::hover_time());
    let down = if resp.is_pointer_button_down_on() {
        1.0
    } else {
        0.0
    };
    let r = rect.shrink(down * 1.5);
    let p = ui.painter();
    p.rect_filled(
        r.translate(vec2(0.0, 5.0)).shrink(3.0),
        13.0,
        color.gamma_multiply(0.35 * hot),
    );
    gradient_rect(
        p,
        r,
        lerp_color(color, Color32::WHITE, 0.10 * hot),
        color.gamma_multiply(0.82),
        13.0,
        10,
    );
    p.text(
        r.center(),
        Align2::CENTER_CENTER,
        text,
        FontId::proportional(13.0),
        Color32::WHITE,
    );
    resp
}

pub fn ghost_button(ui: &mut egui::Ui, pal: &Palette, text: &str) -> egui::Response {
    let w = ui
        .painter()
        .layout_no_wrap(text.to_string(), FontId::proportional(13.0), pal.muted)
        .size()
        .x
        + 32.0;
    ghost_button_sized(ui, pal, text, vec2(w, 40.0))
}

pub fn ghost_button_sized(
    ui: &mut egui::Ui,
    pal: &Palette,
    text: &str,
    size: Vec2,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let hot = ui.ctx().animate_bool_with_time(resp.id, resp.hovered(), crate::motion::hover_time());
    let p = ui.painter();
    p.rect_filled(rect, 13.0, pal.panel_hi.gamma_multiply(0.3 + 1.2 * hot));
    p.rect_stroke(
        rect,
        13.0,
        egui::Stroke::new(1.0, pal.line.gamma_multiply(0.6 + 0.9 * hot)),
        StrokeKind::Inside,
    );
    p.text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        FontId::proportional(13.0),
        pal.text.gamma_multiply(0.78 + 0.22 * hot),
    );
    resp
}

pub fn dialog_title(ui: &mut egui::Ui, pal: &Palette, rtl: bool, text: &str) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 26.0), Sense::hover());
    let (anchor, x) = if rtl {
        (Align2::RIGHT_CENTER, rect.right())
    } else {
        (Align2::LEFT_CENTER, rect.left())
    };
    ui.painter().text(
        pos2(x, rect.center().y),
        anchor,
        text,
        FontId::proportional(16.5),
        pal.text,
    );
    let (line, _) = ui.allocate_exact_size(vec2(ui.available_width(), 10.0), Sense::hover());
    ui.painter().rect_filled(
        Rect::from_min_size(pos2(line.left(), line.center().y), vec2(line.width(), 1.0)),
        0.0,
        pal.line,
    );
}

pub fn toggle(
    ui: &mut egui::Ui,
    pal: &Palette,
    rtl: bool,
    title: &str,
    body: &str,
    on: bool,
) -> bool {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 58.0), Sense::click());
    let hot = ui.ctx().animate_bool_with_time(resp.id, resp.hovered(), crate::motion::hover_time());
    let anim = ui
        .ctx()
        .animate_bool_with_time(resp.id.with("state"), on, crate::motion::hover_time());
    let p = ui.painter();
    p.rect_filled(rect, 14.0, pal.panel_hi.gamma_multiply(0.25 + 0.5 * hot));
    p.rect_stroke(
        rect,
        14.0,
        egui::Stroke::new(1.0, pal.line.gamma_multiply(0.5 + 0.5 * hot)),
        StrokeKind::Inside,
    );

    let pad = 16.0;
    let (anchor, tx) = if rtl {
        (Align2::RIGHT_TOP, rect.right() - pad)
    } else {
        (Align2::LEFT_TOP, rect.left() + pad)
    };
    p.text(
        pos2(tx, rect.top() + 12.0),
        anchor,
        title,
        FontId::proportional(13.5),
        pal.text,
    );
    if !body.is_empty() {
        p.text(
            pos2(tx, rect.top() + 32.0),
            anchor,
            body,
            FontId::proportional(10.5),
            pal.faint,
        );
    }

    let sw = 46.0;
    let sx = if rtl {
        rect.left() + pad
    } else {
        rect.right() - pad - sw
    };
    let track = Rect::from_min_size(pos2(sx, rect.center().y - 12.0), vec2(sw, 24.0));
    let off_track = lerp_color(pal.faint, pal.panel, 0.45);
    p.rect_filled(track, 999.0, lerp_color(off_track, pal.accent, anim));
    p.rect_stroke(
        track,
        999.0,
        egui::Stroke::new(1.0, pal.line.gamma_multiply(1.0 - anim)),
        StrokeKind::Inside,
    );
    let travel = sw - 24.0;
    let knob_x = if rtl {
        track.right() - 12.0 - travel * anim
    } else {
        track.left() + 12.0 + travel * anim
    };
    p.circle_filled(pos2(knob_x, track.center().y), 9.0, Color32::WHITE);

    resp.clicked()
}

pub fn setting_row(
    ui: &mut egui::Ui,
    pal: &Palette,
    rtl: bool,
    title: &str,
    body: &str,
    action: &str,
) -> bool {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 58.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 14.0, pal.panel_hi.gamma_multiply(0.25));
    p.rect_stroke(
        rect,
        14.0,
        egui::Stroke::new(1.0, pal.line.gamma_multiply(0.5)),
        StrokeKind::Inside,
    );
    let pad = 16.0;
    let (anchor, tx) = if rtl {
        (Align2::RIGHT_TOP, rect.right() - pad)
    } else {
        (Align2::LEFT_TOP, rect.left() + pad)
    };
    p.text(
        pos2(tx, rect.top() + 12.0),
        anchor,
        title,
        FontId::proportional(13.5),
        pal.text,
    );
    if !body.is_empty() {
        p.text(
            pos2(tx, rect.top() + 32.0),
            anchor,
            body,
            FontId::proportional(10.5),
            pal.faint,
        );
    }

    let g = ui
        .painter()
        .layout_no_wrap(action.to_string(), FontId::proportional(12.0), pal.accent);
    let bw = g.size().x + 26.0;
    let bx = if rtl {
        rect.left() + pad
    } else {
        rect.right() - pad - bw
    };
    let br = Rect::from_min_size(pos2(bx, rect.center().y - 15.0), vec2(bw, 30.0));
    let bresp = ui.interact(br, resp.id.with("btn"), Sense::click());
    let hot = ui.ctx().animate_bool_with_time(bresp.id, bresp.hovered(), crate::motion::hover_time());
    let p = ui.painter();
    p.rect_filled(br, 10.0, pal.accent.gamma_multiply(0.14 + 0.2 * hot));
    p.text(
        br.center(),
        Align2::CENTER_CENTER,
        action,
        FontId::proportional(12.0),
        pal.accent,
    );
    bresp.clicked()
}

pub fn progress_overlay(
    ctx: &egui::Context,
    pal: &Palette,
    title: &str,
    step: &str,
    hint: &str,
    t: f32,
) {
    egui::Area::new(egui::Id::new("busy"))
        .order(egui::Order::Foreground)
        .fixed_pos(pos2(0.0, 0.0))
        .show(ctx, |ui| {
            let screen = ctx.viewport_rect();
            let p = ui.painter();
            p.rect_filled(screen, 0.0, pal.bg_deep.gamma_multiply(0.80));

            let extra = if hint.is_empty() { 0.0 } else { 26.0 };
            let r = Rect::from_center_size(screen.center(), vec2(360.0, 168.0 + extra));
            p.rect_filled(
                r.translate(vec2(0.0, 14.0)),
                22.0,
                Color32::BLACK.gamma_multiply(0.35),
            );
            p.rect_filled(r, 22.0, pal.panel);
            p.rect_stroke(
                r,
                22.0,
                egui::Stroke::new(1.0, pal.line_hi),
                StrokeKind::Inside,
            );

            let c = pos2(r.center().x, r.top() + 56.0);
            let radius = 20.0;
            p.circle_stroke(c, radius, egui::Stroke::new(2.0, pal.line));
            let arc_len = 18;
            for i in 0..arc_len {
                let f = i as f32 / arc_len as f32;
                let a = t * 3.2 + f * 1.9;
                p.circle_filled(
                    pos2(c.x + a.cos() * radius, c.y + a.sin() * radius),
                    2.0,
                    pal.accent.gamma_multiply(f),
                );
            }
            p.text(
                pos2(r.center().x, r.top() + 104.0),
                Align2::CENTER_CENTER,
                title,
                FontId::proportional(15.0),
                pal.text,
            );
            p.text(
                pos2(r.center().x, r.top() + 128.0),
                Align2::CENTER_CENTER,
                step,
                FontId::proportional(11.5),
                pal.muted,
            );
            if !hint.is_empty() {
                p.text(
                    pos2(r.center().x, r.top() + 152.0),
                    Align2::CENTER_CENTER,
                    hint,
                    FontId::proportional(10.5),
                    pal.warn.gamma_multiply(0.9),
                );
            }
        });
}

pub fn toasts(ctx: &egui::Context, pal: &Palette, items: &[(String, ToastKind, f32)]) {
    if items.is_empty() {
        return;
    }
    egui::Area::new(egui::Id::new("toasts"))
        .order(egui::Order::Tooltip)
        .fixed_pos(pos2(0.0, 0.0))
        .show(ctx, |ui| {
            let screen = ctx.viewport_rect();
            let p = ui.painter();
            let mut y = screen.top() + 88.0;
            for (text, kind, age) in items {
                let fade = if *age < 0.3 {
                    back_out(age / 0.3)
                } else if *age > 3.6 {
                    ((4.2 - age) / 0.6).clamp(0.0, 1.0)
                } else {
                    1.0
                };
                let color = if *kind == ToastKind::Ok {
                    pal.live
                } else {
                    pal.danger
                };
                let g = p.layout_no_wrap(text.clone(), FontId::proportional(12.5), color);
                let w = (g.size().x + 52.0).min(screen.width() - 60.0);
                let r = Rect::from_center_size(
                    pos2(screen.center().x, y - (1.0 - fade) * 14.0),
                    vec2(w, 42.0),
                );
                p.rect_filled(
                    r.translate(vec2(0.0, 7.0)),
                    15.0,
                    Color32::BLACK.gamma_multiply(0.28 * fade),
                );
                p.rect_filled(r, 15.0, pal.panel.gamma_multiply(fade));
                p.rect_stroke(
                    r,
                    15.0,
                    egui::Stroke::new(1.0, color.gamma_multiply(0.5 * fade)),
                    StrokeKind::Inside,
                );
                p.circle_filled(
                    pos2(r.left() + 19.0, r.center().y),
                    4.2,
                    color.gamma_multiply(fade),
                );
                p.text(
                    pos2(r.center().x + 10.0, r.center().y),
                    Align2::CENTER_CENTER,
                    text,
                    FontId::proportional(12.5),
                    pal.text.gamma_multiply(fade),
                );
                y += 50.0;
            }
        });
}

pub fn titlebar_button(ui: &mut egui::Ui, pal: &Palette, kind: u8) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(46.0, 32.0), Sense::click());
    let hot = ui.ctx().animate_bool_with_time(resp.id, resp.hovered(), crate::motion::hover_time());
    let p = ui.painter();
    let bg = if kind == 2 {
        Color32::from_rgb(232, 17, 60).gamma_multiply(hot)
    } else {
        pal.panel_hi.gamma_multiply(1.2 * hot)
    };
    p.rect_filled(rect, 0.0, bg);
    let c = rect.center();
    let col = if kind == 2 && hot > 0.4 {
        Color32::WHITE
    } else {
        pal.muted.gamma_multiply(0.75 + 0.25 * hot)
    };
    let st = egui::Stroke::new(1.2, col);
    match kind {
        0 => {
            p.line_segment([pos2(c.x - 5.0, c.y), pos2(c.x + 5.0, c.y)], st);
        }
        1 => {
            p.rect_stroke(
                Rect::from_center_size(c, vec2(9.0, 9.0)),
                1.0,
                st,
                StrokeKind::Inside,
            );
        }
        _ => {
            p.line_segment([pos2(c.x - 5.0, c.y - 5.0), pos2(c.x + 5.0, c.y + 5.0)], st);
            p.line_segment([pos2(c.x + 5.0, c.y - 5.0), pos2(c.x - 5.0, c.y + 5.0)], st);
        }
    }
    resp
}

pub fn status_chip(
    ui: &mut egui::Ui,
    pal: &Palette,
    text: &str,
    dot: Color32,
    t: f32,
) -> egui::Response {
    let g = ui
        .painter()
        .layout_no_wrap(text.to_string(), FontId::proportional(11.0), pal.muted);
    let (rect, resp) = ui.allocate_exact_size(vec2(g.size().x + 40.0, 30.0), Sense::click());
    let hot = ui.ctx().animate_bool_with_time(resp.id, resp.hovered(), crate::motion::hover_time());
    let p = ui.painter();
    p.rect_filled(rect, 999.0, pal.panel.gamma_multiply(0.75 + 0.25 * hot));
    p.rect_stroke(
        rect,
        999.0,
        egui::Stroke::new(1.0, pal.line.gamma_multiply(0.8 + 0.6 * hot)),
        StrokeKind::Inside,
    );
    p.circle_filled(
        pos2(rect.right() - 15.0, rect.center().y),
        3.4,
        dot.gamma_multiply(0.7 + 0.3 * pulse(t, 1.8)),
    );
    p.text(
        pos2(rect.right() - 26.0, rect.center().y),
        Align2::RIGHT_CENTER,
        text,
        FontId::proportional(11.0),
        pal.muted.gamma_multiply(0.9 + 0.1 * hot),
    );
    resp
}

#[allow(clippy::too_many_arguments)]
pub fn rail_button(
    ui: &mut egui::Ui,
    pal: &Palette,
    id: &str,
    color: Color32,
    selected: bool,
    installed: bool,
    count: usize,
    side: f32,
    t: f32,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(side, side), Sense::click());
    let hot = ui.ctx().animate_bool_with_time(resp.id, resp.hovered(), crate::motion::hover_time());
    let sel = ui
        .ctx()
        .animate_bool_with_time(resp.id.with("sel"), selected, crate::motion::hover_time());
    let p = ui.painter();
    p.rect_filled(
        rect,
        13.0,
        lerp_color(
            pal.panel_hi.gamma_multiply(0.35 + 0.8 * hot),
            color.gamma_multiply(0.20),
            sel,
        ),
    );
    p.rect_stroke(
        rect,
        13.0,
        egui::Stroke::new(
            1.0,
            lerp_color(pal.line.gamma_multiply(0.5 + 0.6 * hot), color, sel)
                .gamma_multiply(0.55 + 0.45 * sel),
        ),
        StrokeKind::Inside,
    );
    let icon = 22.0 + 1.5 * hot;
    platform_icon(
        ui,
        Rect::from_center_size(rect.center(), vec2(icon, icon)),
        id,
        if installed {
            color.gamma_multiply(0.55 + 0.45 * (sel.max(hot)))
        } else {
            pal.faint.gamma_multiply(0.7)
        },
        1.0,
    );
    let p = ui.painter();
    if selected {
        p.rect_filled(
            Rect::from_min_size(
                pos2(rect.left() - 7.0, rect.center().y - 9.0),
                vec2(3.0, 18.0),
            ),
            2.0,
            color.gamma_multiply(0.85),
        );
    }
    // العلامة تلتصق بالأيقونة نفسها لا بركن الزر، فالزر بلا خلفية ظاهرة
    // ما لم يكن مختارًا أو تحت المؤشر، فتبدو العلامة عائمة بعيدة عنها
    let mark = pos2(rect.center().x + icon * 0.55, rect.center().y - icon * 0.5);
    if count > 0 {
        let br = Rect::from_center_size(mark, vec2(16.0, 14.0));
        p.rect_filled(br, 999.0, pal.bg_deep);
        p.rect_stroke(
            br,
            999.0,
            egui::Stroke::new(1.0, color.gamma_multiply(0.5)),
            StrokeKind::Inside,
        );
        p.text(
            br.center(),
            Align2::CENTER_CENTER,
            format!("{count}"),
            FontId::proportional(8.5),
            pal.muted,
        );
    } else if installed {
        p.circle_filled(mark, 3.0, pal.live.gamma_multiply(0.5 + 0.35 * pulse(t, 1.7)));
    }
    resp
}

pub fn rail_icon(
    ui: &mut egui::Ui,
    pal: &Palette,
    kind: u8,
    selected: bool,
    side: f32,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(side, side), Sense::click());
    let hot = ui.ctx().animate_bool_with_time(resp.id, resp.hovered(), crate::motion::hover_time());
    let sel = ui
        .ctx()
        .animate_bool_with_time(resp.id.with("s"), selected, crate::motion::hover_time());
    let p = ui.painter();
    p.rect_filled(
        rect,
        13.0,
        lerp_color(
            pal.panel_hi.gamma_multiply(0.3 + 0.8 * hot),
            pal.accent.gamma_multiply(0.22),
            sel,
        ),
    );
    p.rect_stroke(
        rect,
        13.0,
        egui::Stroke::new(
            1.0,
            lerp_color(pal.line.gamma_multiply(0.5 + 0.6 * hot), pal.accent, sel)
                .gamma_multiply(0.5 + 0.5 * sel),
        ),
        StrokeKind::Inside,
    );
    let c = rect.center();
    let col = lerp_color(pal.muted.gamma_multiply(0.8 + 0.2 * hot), pal.accent, sel);
    match kind {
        0 => {
            for i in 0..8 {
                let a = std::f32::consts::TAU * i as f32 / 8.0 + hot * 0.5;
                let (sx, sy) = (a.cos(), a.sin());
                p.line_segment(
                    [
                        pos2(c.x + sx * 5.2, c.y + sy * 5.2),
                        pos2(c.x + sx * 8.6, c.y + sy * 8.6),
                    ],
                    egui::Stroke::new(2.0, col),
                );
            }
            p.circle_stroke(c, 4.4, egui::Stroke::new(1.7, col));
        }
        1 => {
            p.add(egui::Shape::line(
                vec![
                    pos2(c.x - 9.0, c.y - 0.5),
                    pos2(c.x, c.y - 8.5),
                    pos2(c.x + 9.0, c.y - 0.5),
                ],
                egui::Stroke::new(1.8, col),
            ));
            p.rect_stroke(
                Rect::from_min_max(pos2(c.x - 6.5, c.y - 1.0), pos2(c.x + 6.5, c.y + 8.0)),
                2.5,
                egui::Stroke::new(1.6, col),
                StrokeKind::Inside,
            );
            p.rect_filled(
                Rect::from_min_max(pos2(c.x - 2.2, c.y + 2.0), pos2(c.x + 2.2, c.y + 8.0)),
                1.2,
                col.gamma_multiply(0.75),
            );
        }
        _ => {
            for (dx, r) in [(-4.4_f32, 3.3_f32), (4.4, 3.3)] {
                p.circle_stroke(
                    pos2(c.x + dx, c.y - 4.0),
                    r,
                    egui::Stroke::new(1.5, col),
                );
                p.add(egui::Shape::line(
                    arc_pts(
                        pos2(c.x + dx, c.y + 8.4),
                        6.2,
                        std::f32::consts::PI + 0.35,
                        std::f32::consts::TAU - 0.35,
                        10,
                    ),
                    egui::Stroke::new(1.5, col),
                ));
            }
        }
    }
    resp
}

pub fn key_chip(ui: &mut egui::Ui, pal: &Palette, key: &str, label: &str) {
    let kg = ui
        .painter()
        .layout_no_wrap(key.to_string(), FontId::proportional(9.5), pal.muted);
    let lg = ui
        .painter()
        .layout_no_wrap(label.to_string(), FontId::proportional(10.0), pal.faint);
    let kw = kg.size().x + 16.0;
    let (rect, _) =
        ui.allocate_exact_size(vec2(kw + lg.size().x + 12.0, 20.0), Sense::hover());
    let p = ui.painter();
    let kr = Rect::from_min_size(rect.min, vec2(kw, 20.0));
    p.rect_filled(kr, 6.0, pal.panel_hi.gamma_multiply(0.8));
    p.rect_stroke(
        kr,
        6.0,
        egui::Stroke::new(1.0, pal.line.gamma_multiply(0.7)),
        StrokeKind::Inside,
    );
    p.text(
        kr.center(),
        Align2::CENTER_CENTER,
        key,
        FontId::proportional(9.5),
        pal.muted,
    );
    p.text(
        pos2(kr.right() + 6.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(10.0),
        pal.faint,
    );
}

#[allow(clippy::too_many_arguments)]
pub fn account_row(
    ui: &mut egui::Ui,
    pal: &Palette,
    rtl: bool,
    acc: &Account,
    platform_id: &str,
    is_live: bool,
    selected: bool,
    appear: f32,
    t: f32,
    lang: Lang,
) -> Option<u8> {
    let (slot, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 56.0), Sense::click());
    if appear <= 0.01 {
        return None;
    }
    let e = ease_out(appear);
    let hot = ui.ctx().animate_bool_with_time(resp.id, resp.hovered(), crate::motion::hover_time());
    let sel = ui
        .ctx()
        .animate_bool_with_time(resp.id.with("s"), selected, crate::motion::hover_time());
    let rect = slot.translate(vec2(if rtl { 1.0 } else { -1.0 } * (1.0 - e) * 18.0, 0.0));
    let a = e;
    let mut action = None;

    let p = ui.painter();
    p.rect_filled(
        rect,
        14.0,
        lerp_color(
            pal.panel_hi.gamma_multiply((0.22 + 0.5 * hot) * a),
            pal.accent.gamma_multiply(0.16 * a),
            sel,
        ),
    );
    p.rect_stroke(
        rect,
        14.0,
        egui::Stroke::new(
            1.0,
            if is_live {
                pal.live.gamma_multiply((0.45 + 0.25 * pulse(t, 1.7)) * a)
            } else {
                lerp_color(pal.line.gamma_multiply(0.45 + 0.6 * hot), pal.accent, sel)
                    .gamma_multiply(a)
            },
        ),
        StrokeKind::Inside,
    );

    let pad = 12.0;
    let av_c = if rtl {
        pos2(rect.right() - pad - 18.0, rect.center().y)
    } else {
        pos2(rect.left() + pad + 18.0, rect.center().y)
    };
    avatar(ui, av_c, 18.0, acc, platform_id, pal, is_live, t, a);

    let tx = if rtl {
        rect.right() - pad - 46.0
    } else {
        rect.left() + pad + 46.0
    };
    let p = ui.painter();
    p.text(
        pos2(tx, rect.center().y - 8.0),
        if rtl {
            Align2::RIGHT_CENTER
        } else {
            Align2::LEFT_CENTER
        },
        &acc.name,
        FontId::proportional(13.0),
        pal.text.gamma_multiply(a),
    );
    if !acc.note.is_empty() {
        p.text(
            pos2(tx, rect.center().y + 9.0),
            if rtl {
                Align2::RIGHT_CENTER
            } else {
                Align2::LEFT_CENTER
            },
            &acc.note,
            FontId::proportional(9.5),
            pal.faint.gamma_multiply(a),
        );
    }

    if is_live {
        let label = lang.t("الفعّال", "active");
        let g = p.layout_no_wrap(label.to_string(), FontId::proportional(9.0), pal.live);
        let w = g.size().x + 18.0;
        let br = Rect::from_min_size(
            pos2(
                if rtl {
                    rect.left() + pad
                } else {
                    rect.right() - pad - w
                },
                rect.center().y - 9.0,
            ),
            vec2(w, 18.0),
        );
        p.rect_filled(br, 999.0, pal.live.gamma_multiply(0.16 * a));
        p.text(
            br.center(),
            Align2::CENTER_CENTER,
            label,
            FontId::proportional(9.0),
            pal.live.gamma_multiply(a),
        );
    } else if hot > 0.04 {
        let label = lang.t("بدّل", "Switch");
        let g = p.layout_no_wrap(label.to_string(), FontId::proportional(10.0), pal.accent);
        let w = g.size().x + 22.0;
        let br = Rect::from_min_size(
            pos2(
                if rtl {
                    rect.left() + pad
                } else {
                    rect.right() - pad - w
                },
                rect.center().y - 11.0,
            ),
            vec2(w, 22.0),
        );
        let id = resp.id.with("go");
        let sub = ui.interact(br, id, Sense::click());
        let sh = ui.ctx().animate_bool_with_time(id, sub.hovered(), crate::motion::hover_time());
        let p = ui.painter();
        p.rect_filled(
            br,
            999.0,
            pal.accent.gamma_multiply((0.16 + 0.3 * sh) * hot * a),
        );
        p.text(
            br.center(),
            Align2::CENTER_CENTER,
            label,
            FontId::proportional(10.0),
            pal.accent.gamma_multiply((0.75 + 0.25 * sh) * hot * a),
        );
        if sub.clicked() {
            action = Some(1);
        }
    }

    if resp.clicked() && action.is_none() {
        action = Some(0);
    }
    if resp.secondary_clicked() {
        action = Some(2);
    }
    action
}

pub fn stat_tile(
    painter: &egui::Painter,
    rect: Rect,
    pal: &Palette,
    label: &str,
    value: &str,
    accent: Option<Color32>,
    alpha: f32,
) {
    painter.rect_filled(rect, 14.0, pal.panel_hi.gamma_multiply(0.75 * alpha));
    painter.rect_stroke(
        rect,
        14.0,
        egui::Stroke::new(1.0, pal.line.gamma_multiply(0.8 * alpha)),
        StrokeKind::Inside,
    );
    painter.text(
        pos2(rect.center().x, rect.top() + 19.0),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(9.5),
        pal.faint.gamma_multiply(alpha),
    );
    let col = accent.unwrap_or(pal.text);
    let mut font = FontId::proportional(13.0);
    let mut w = painter
        .layout_no_wrap(value.to_owned(), font.clone(), col)
        .size()
        .x;
    while w > rect.width() - 16.0 && font.size > 9.0 {
        font.size -= 0.5;
        w = painter
            .layout_no_wrap(value.to_owned(), font.clone(), col)
            .size()
            .x;
    }
    painter.text(
        pos2(rect.center().x, rect.bottom() - 19.0),
        Align2::CENTER_CENTER,
        value,
        font,
        col.gamma_multiply(alpha),
    );
}

pub fn home_platform(
    ui: &mut egui::Ui,
    pal: &Palette,
    rect: Rect,
    id: &str,
    color: Color32,
    name: &str,
    sub: &str,
    installed: bool,
    alpha: f32,
) -> egui::Response {
    let resp = ui.interact(rect, ui.id().with(("home", id)), Sense::click());
    let hot = ui
        .ctx()
        .animate_bool_with_time(resp.id, resp.hovered(), crate::motion::hover_time());
    let r = rect.translate(vec2(0.0, -4.0 * hot));
    {
        let p = ui.painter();
        p.rect_filled(
            r.translate(vec2(0.0, 7.0 + 4.0 * hot)).shrink(6.0),
            18.0,
            Color32::BLACK.gamma_multiply((0.14 + 0.20 * hot) * alpha),
        );
        p.rect_filled(r, 18.0, pal.panel.gamma_multiply(0.92 * alpha));
        p.rect_filled(
            Rect::from_min_size(r.min, vec2(r.width(), r.height() * 0.55)),
            18.0,
            color.gamma_multiply(0.05 * (0.4 + hot) * alpha),
        );
        p.rect_stroke(
            r,
            18.0,
            egui::Stroke::new(
                1.0 + 0.4 * hot,
                lerp_color(pal.line, color, 0.22 + 0.55 * hot).gamma_multiply(alpha),
            ),
            StrokeKind::Inside,
        );
    }
    // المواضع محسوبة من ارتفاع البطاقة لا بأرقام ثابتة، فالشبكة تضيّقها
    // إلى النصف حين تنزل إلى صفّين، وكانت الأيقونة تركب على الاسم
    let pad = (r.height() * 0.12).clamp(6.0, 14.0);
    let sub_y = r.bottom() - pad - 5.0;
    let name_y = sub_y - 15.0;
    let head = name_y - 9.0 - r.top();
    let side = (head * 0.78).clamp(17.0, 40.0);
    let icon = Rect::from_center_size(
        pos2(r.center().x, r.top() + pad + side * 0.5),
        vec2(side + 3.0 * hot, side + 3.0 * hot),
    );
    platform_icon(
        ui,
        icon,
        id,
        if installed {
            color
        } else {
            lerp_color(pal.faint, color, 0.25)
        },
        alpha * (0.70 + 0.30 * hot + if installed { 0.0 } else { -0.25 }),
    );
    let p = ui.painter();
    p.text(
        pos2(r.center().x, name_y),
        Align2::CENTER_CENTER,
        name,
        FontId::proportional(if r.height() < 78.0 { 12.0 } else { 13.5 }),
        pal.text.gamma_multiply(alpha * (0.82 + 0.18 * hot)),
    );
    p.text(
        pos2(r.center().x, sub_y),
        Align2::CENTER_CENTER,
        sub,
        FontId::proportional(10.0),
        if installed {
            pal.muted.gamma_multiply(alpha)
        } else {
            pal.faint.gamma_multiply(alpha)
        },
    );
    if installed {
        p.circle_filled(
            pos2(r.right() - pad - 3.0, r.top() + pad + 3.0),
            3.2,
            pal.live.gamma_multiply(alpha),
        );
    }
    resp
}

pub fn band_label(painter: &egui::Painter, rect: Rect, pal: &Palette, rtl: bool, text: &str) {
    let ink = pal.faint;
    let (anchor, x) = if rtl {
        (Align2::RIGHT_CENTER, rect.right())
    } else {
        (Align2::LEFT_CENTER, rect.left())
    };
    let g = painter.layout_no_wrap(text.to_owned(), FontId::proportional(11.0), ink);
    painter.text(
        pos2(x, rect.center().y),
        anchor,
        text,
        FontId::proportional(11.0),
        ink,
    );
    let pad = g.size().x + 14.0;
    let (a, b) = if rtl {
        (rect.left(), rect.right() - pad)
    } else {
        (rect.left() + pad, rect.right())
    };
    if b > a {
        painter.line_segment(
            [pos2(a, rect.center().y), pos2(b, rect.center().y)],
            egui::Stroke::new(1.0, ink.gamma_multiply(0.45)),
        );
    }
}

pub fn info_row(ui: &mut egui::Ui, pal: &Palette, rtl: bool, title: &str, value: &str) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 46.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 14.0, pal.panel.gamma_multiply(0.38));
    p.rect_stroke(
        rect,
        14.0,
        egui::Stroke::new(1.0, pal.line.gamma_multiply(0.7)),
        StrokeKind::Inside,
    );
    let pad = 16.0;
    if rtl {
        p.text(
            pos2(rect.right() - pad, rect.center().y),
            Align2::RIGHT_CENTER,
            title,
            FontId::proportional(12.5),
            pal.text,
        );
        p.text(
            pos2(rect.left() + pad, rect.center().y),
            Align2::LEFT_CENTER,
            value,
            FontId::proportional(11.5),
            pal.muted,
        );
    } else {
        p.text(
            pos2(rect.left() + pad, rect.center().y),
            Align2::LEFT_CENTER,
            title,
            FontId::proportional(12.5),
            pal.text,
        );
        p.text(
            pos2(rect.right() - pad, rect.center().y),
            Align2::RIGHT_CENTER,
            value,
            FontId::proportional(11.5),
            pal.muted,
        );
    }
}

pub fn swatches(
    ui: &mut egui::Ui,
    pal: &Palette,
    rtl: bool,
    label: &str,
    value: &str,
    colors: &[Color32],
    current: u8,
) -> Option<u8> {
    let h = 58.0;
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::hover());
    {
        let p = ui.painter();
        p.rect_filled(rect, 14.0, pal.panel.gamma_multiply(0.38));
        p.rect_stroke(
            rect,
            14.0,
            egui::Stroke::new(1.0, pal.line.gamma_multiply(0.7)),
            StrokeKind::Inside,
        );
    }
    {
        let p = ui.painter();
        let (anchor, x) = if rtl {
            (Align2::RIGHT_TOP, rect.right() - 16.0)
        } else {
            (Align2::LEFT_TOP, rect.left() + 16.0)
        };
        p.text(
            pos2(x, rect.top() + 9.0),
            anchor,
            label,
            FontId::proportional(12.5),
            pal.text,
        );
        p.text(
            pos2(x, rect.top() + 27.0),
            anchor,
            value,
            FontId::proportional(10.5),
            pal.faint,
        );
    }
    let mut picked = None;
    let n = colors.len();
    let side = 26.0;
    let gap = 10.0;
    let total = n as f32 * side + (n - 1) as f32 * gap;
    let start = if rtl {
        rect.left() + 16.0
    } else {
        rect.right() - 16.0 - side
    };
    let dir = if rtl { 1.0 } else { -1.0 };
    let _ = total;
    for (i, c) in colors.iter().enumerate() {
        let x = start + dir * i as f32 * (side + gap);
        let cell = Rect::from_min_size(pos2(x, rect.center().y - side * 0.5), vec2(side, side));
        let r = ui.interact(cell, ui.id().with(("sw", i)), Sense::click());
        let hot = ui
            .ctx()
            .animate_bool_with_time(r.id, r.hovered(), crate::motion::hover_time());
        if r.clicked() {
            picked = Some(i as u8);
        }
        let on = i as u8 == current;
        let p = ui.painter();
        p.circle_filled(cell.center(), side * 0.42 + 1.5 * hot, *c);
        if on {
            p.circle_stroke(
                cell.center(),
                side * 0.62,
                egui::Stroke::new(1.6, c.gamma_multiply(0.85)),
            );
        }
    }
    picked
}

pub fn profile_chip(
    ui: &mut egui::Ui,
    pal: &Palette,
    label: &str,
    color: Color32,
    selected: bool,
) -> egui::Response {
    let g = ui
        .painter()
        .layout_no_wrap(label.to_owned(), FontId::proportional(11.5), pal.text);
    let (rect, resp) =
        ui.allocate_exact_size(vec2(g.size().x + 44.0, 34.0), Sense::click());
    let hot = ui
        .ctx()
        .animate_bool_with_time(resp.id, resp.hovered(), crate::motion::hover_time());
    let p = ui.painter();
    p.rect_filled(
        rect,
        999.0,
        color.gamma_multiply(if selected { 0.20 } else { 0.07 + 0.08 * hot }),
    );
    p.rect_stroke(
        rect,
        999.0,
        egui::Stroke::new(
            1.0,
            if selected {
                color.gamma_multiply(0.85)
            } else {
                pal.line.gamma_multiply(0.8 + 0.6 * hot)
            },
        ),
        StrokeKind::Inside,
    );
    p.circle_filled(pos2(rect.left() + 17.0, rect.center().y), 5.0, color);
    p.text(
        pos2(rect.center().x + 8.0, rect.center().y),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(11.5),
        if selected {
            pal.text
        } else {
            pal.muted.gamma_multiply(0.85 + 0.15 * hot)
        },
    );
    resp
}

pub fn shimmer_text(
    painter: &egui::Painter,
    center: egui::Pos2,
    text: &str,
    size: f32,
    base: Color32,
    glint: Color32,
    width: f32,
    t: f32,
    alpha: f32,
) {
    let font = FontId::proportional(size);
    painter.text(center, Align2::CENTER_CENTER, text, font.clone(), base);
    let band = 110.0;
    let span = width + band * 2.0;
    let sweep = (t * 0.22).rem_euclid(1.0);
    let x = center.x - width * 0.5 - band + sweep * span;
    let clip = Rect::from_min_size(pos2(x, center.y - size * 0.62), vec2(band, size * 1.24));
    painter
        .with_clip_rect(clip)
        .text(center, Align2::CENTER_CENTER, text, font, glint.gamma_multiply(0.55 * alpha));
}

pub fn loader(
    painter: &egui::Painter,
    center: egui::Pos2,
    r: f32,
    pal: &Palette,
    t: f32,
    alpha: f32,
) {
    if alpha <= 0.004 {
        return;
    }
    let ring = |from: f32, sweep: f32, radius: f32, width: f32, col: Color32| {
        let steps = 40;
        let pts: Vec<egui::Pos2> = (0..=steps)
            .map(|i| {
                let a = from + sweep * i as f32 / steps as f32;
                pos2(center.x + radius * a.cos(), center.y + radius * a.sin())
            })
            .collect();
        painter.add(egui::Shape::line(pts, egui::Stroke::new(width, col)));
    };

    let tau = std::f32::consts::TAU;
    painter.circle_stroke(
        center,
        r * 1.62,
        egui::Stroke::new(1.2, pal.line.gamma_multiply(1.6 * alpha)),
    );
    let spin = t * 2.1;
    let breathe = 0.55 + 0.45 * (t * 1.6).sin();
    ring(
        spin,
        tau * (0.16 + 0.22 * breathe),
        r * 1.62,
        2.4,
        pal.accent.gamma_multiply(alpha),
    );
    ring(
        spin + tau * 0.5,
        tau * (0.10 + 0.14 * breathe),
        r * 1.62,
        2.4,
        pal.accent_2.gamma_multiply(0.85 * alpha),
    );
    ring(
        -spin * 0.62,
        tau * 0.20,
        r * 2.02,
        1.4,
        pal.accent_2.gamma_multiply(0.35 * alpha),
    );
    for k in 0..3 {
        let f = ((t * 0.7 + k as f32 / 3.0).rem_euclid(1.0)).clamp(0.0, 1.0);
        painter.circle_stroke(
            center,
            r * (1.62 + f * 1.1),
            egui::Stroke::new(1.0, pal.accent.gamma_multiply(0.26 * (1.0 - f) * alpha)),
        );
    }

    let scale = 1.0 + 0.03 * (t * 1.9).sin();
    brand_mark(
        painter,
        Rect::from_center_size(center, vec2(r * 2.0 * scale, r * 2.0 * scale)),
        pal,
        t,
    );
}

fn arc_pts(center: egui::Pos2, r: f32, from: f32, to: f32, steps: usize) -> Vec<egui::Pos2> {
    (0..=steps)
        .map(|i| {
            let a = from + (to - from) * i as f32 / steps as f32;
            pos2(center.x + r * a.cos(), center.y + r * a.sin())
        })
        .collect()
}

pub fn mini_glyph(painter: &egui::Painter, rect: Rect, kind: u8, col: Color32) {
    let c = rect.center();
    let s = rect.width().min(rect.height()) * 0.5;
    let w = (s * 0.17).max(1.3);
    let st = egui::Stroke::new(w, col);
    match kind {
        0 => {
            let pts = vec![
                pos2(c.x, c.y - s),
                pos2(c.x + s * 0.82, c.y - s * 0.55),
                pos2(c.x + s * 0.82, c.y + s * 0.18),
                pos2(c.x, c.y + s),
                pos2(c.x - s * 0.82, c.y + s * 0.18),
                pos2(c.x - s * 0.82, c.y - s * 0.55),
            ];
            painter.add(egui::Shape::convex_polygon(
                pts,
                col.gamma_multiply(0.16),
                st,
            ));
            painter.add(egui::Shape::line(
                vec![
                    pos2(c.x - s * 0.36, c.y),
                    pos2(c.x - s * 0.08, c.y + s * 0.30),
                    pos2(c.x + s * 0.42, c.y - s * 0.28),
                ],
                egui::Stroke::new(w * 1.1, col),
            ));
        }
        1 => {
            let body = Rect::from_center_size(pos2(c.x, c.y + s * 0.28), vec2(s * 1.26, s * 0.98));
            painter.rect_filled(body, s * 0.20, col.gamma_multiply(0.16));
            painter.rect_stroke(body, s * 0.20, st, StrokeKind::Inside);
            painter.add(egui::Shape::line(
                arc_pts(
                    pos2(c.x, c.y - s * 0.22),
                    s * 0.44,
                    std::f32::consts::PI,
                    std::f32::consts::TAU,
                    14,
                ),
                st,
            ));
            painter.circle_filled(pos2(c.x, c.y + s * 0.26), w * 0.85, col);
        }
        2 => {
            let body = Rect::from_center_size(c, vec2(s * 1.1, s * 1.1));
            painter.rect_stroke(body, s * 0.18, st, StrokeKind::Inside);
            painter.rect_filled(
                Rect::from_center_size(c, vec2(s * 0.46, s * 0.46)),
                s * 0.10,
                col.gamma_multiply(0.75),
            );
            for i in 0..3 {
                let o = (i as f32 - 1.0) * s * 0.42;
                for (dx, dy, ex, ey) in [
                    (o, -s * 0.55, o, -s * 0.92),
                    (o, s * 0.55, o, s * 0.92),
                    (-s * 0.55, o, -s * 0.92, o),
                    (s * 0.55, o, s * 0.92, o),
                ] {
                    painter.line_segment(
                        [pos2(c.x + dx, c.y + dy), pos2(c.x + ex, c.y + ey)],
                        egui::Stroke::new(w * 0.8, col.gamma_multiply(0.85)),
                    );
                }
            }
        }
        _ => {
            for k in 1..=3 {
                let r = s * 0.28 * k as f32;
                painter.add(egui::Shape::line(
                    arc_pts(
                        pos2(c.x, c.y + s * 0.5),
                        r,
                        -std::f32::consts::FRAC_PI_2 - 0.7,
                        -std::f32::consts::FRAC_PI_2 + 0.7,
                        12,
                    ),
                    egui::Stroke::new(w * 0.85, col.gamma_multiply(1.0 - k as f32 * 0.16)),
                ));
            }
            painter.circle_filled(pos2(c.x, c.y + s * 0.5), w * 1.1, col);
            painter.line_segment(
                [
                    pos2(c.x - s * 0.82, c.y - s * 0.82),
                    pos2(c.x + s * 0.82, c.y + s * 0.82),
                ],
                egui::Stroke::new(w * 1.05, col),
            );
        }
    }
}

pub fn emblem(
    painter: &egui::Painter,
    center: egui::Pos2,
    r: f32,
    pal: &Palette,
    t: f32,
    safe: bool,
    alpha: f32,
) {
    if alpha <= 0.004 {
        return;
    }
    let tau = std::f32::consts::TAU;
    let key = if safe { pal.live } else { pal.warn };

    for k in 0..3 {
        let f = ((t * 0.30 + k as f32 / 3.0).rem_euclid(1.0)).clamp(0.0, 1.0);
        painter.add(egui::Shape::line(
            {
                let mut v = crate::theme::hex_points(center, r * (1.30 + f * 0.85), 0.0);
                v.push(v[0]);
                v
            },
            egui::Stroke::new(
                1.1,
                pal.accent.gamma_multiply(0.24 * (1.0 - f) * alpha),
            ),
        ));
    }

    let spin = t * 0.55;
    for (i, (rad, dir, col)) in [
        (r * 1.42, 1.0, pal.accent),
        (r * 1.60, -0.72, pal.accent_2),
    ]
    .into_iter()
    .enumerate()
    {
        for seg in 0..3 {
            let base = spin * dir + tau * seg as f32 / 3.0 + i as f32 * 0.6;
            painter.add(egui::Shape::line(
                arc_pts(center, rad, base, base + tau * 0.13, 14),
                egui::Stroke::new(1.8, col.gamma_multiply(0.55 * alpha)),
            ));
        }
    }

    let mut inner = crate::theme::hex_points(center, r * 0.98, 0.0);
    inner.push(inner[0]);
    painter.add(egui::Shape::convex_polygon(
        crate::theme::hex_points(center, r * 0.98, 0.0),
        pal.bg_deep.gamma_multiply(0.85 * alpha),
        egui::Stroke::NONE,
    ));
    for k in (1..=5).rev() {
        let f = k as f32 / 5.0;
        painter.add(egui::Shape::convex_polygon(
            crate::theme::hex_points(center, r * 0.98 * f, 0.0),
            pal.accent.gamma_multiply(0.045 * (1.0 - f) * alpha),
            egui::Stroke::NONE,
        ));
    }
    painter.add(egui::Shape::line(
        inner,
        egui::Stroke::new(1.6, pal.accent.gamma_multiply(0.85 * alpha)),
    ));
    let mut mid = crate::theme::hex_points(center, r * 0.70, 0.0);
    mid.push(mid[0]);
    painter.add(egui::Shape::line(
        mid,
        egui::Stroke::new(1.0, pal.line_hi.gamma_multiply(0.9 * alpha)),
    ));
    for v in crate::theme::hex_points(center, r * 0.98, 0.0) {
        painter.circle_filled(v, 2.2, pal.accent.gamma_multiply(0.9 * alpha));
    }

    let scan = (t * 0.42).rem_euclid(1.0);
    let sy = center.y - r * 0.9 + scan * r * 1.8;
    let half = (r * 0.9 - (sy - center.y).abs()).max(0.0) * 0.98;
    if half > 1.0 {
        painter.line_segment(
            [pos2(center.x - half, sy), pos2(center.x + half, sy)],
            egui::Stroke::new(
                1.4,
                pal.accent_2.gamma_multiply(0.55 * (1.0 - (scan - 0.5).abs() * 1.4) * alpha),
            ),
        );
    }

    mini_glyph(
        painter,
        Rect::from_center_size(center, vec2(r * 0.86, r * 0.86)),
        if safe { 1 } else { 0 },
        key.gamma_multiply(alpha),
    );

    let tick = 0.5 + 0.5 * (t * 1.4).sin();
    painter.circle_filled(
        pos2(center.x + r * 0.62, center.y - r * 0.72),
        3.0 + 0.8 * tick,
        key.gamma_multiply((0.55 + 0.45 * tick) * alpha),
    );
}

pub fn stat_line(
    painter: &egui::Painter,
    rect: Rect,
    pal: &Palette,
    rtl: bool,
    value: &str,
    label: &str,
    col: Color32,
    alpha: f32,
) {
    let (anchor, x) = if rtl {
        (Align2::RIGHT_CENTER, rect.right())
    } else {
        (Align2::LEFT_CENTER, rect.left())
    };
    let g = painter.layout_no_wrap(value.to_owned(), FontId::proportional(18.0), col);
    painter.text(
        pos2(x, rect.top() + 12.0),
        anchor,
        value,
        FontId::proportional(18.0),
        col.gamma_multiply(alpha),
    );
    painter.text(
        pos2(x, rect.top() + 30.0),
        anchor,
        label,
        FontId::proportional(9.5),
        pal.faint.gamma_multiply(alpha),
    );
    let bar_x = if rtl { rect.right() + 9.0 } else { rect.left() - 9.0 };
    painter.line_segment(
        [pos2(bar_x, rect.top() + 4.0), pos2(bar_x, rect.top() + 36.0)],
        egui::Stroke::new(2.0, col.gamma_multiply(0.45 * alpha)),
    );
    let _ = g;
}

pub fn assure_chip(
    painter: &egui::Painter,
    rect: Rect,
    pal: &Palette,
    rtl: bool,
    kind: u8,
    text: &str,
    col: Color32,
    alpha: f32,
) {
    painter.rect_filled(rect, 11.0, col.gamma_multiply(0.10 * alpha));
    painter.rect_stroke(
        rect,
        11.0,
        egui::Stroke::new(1.0, col.gamma_multiply(0.32 * alpha)),
        StrokeKind::Inside,
    );
    let gx = if rtl {
        rect.right() - 11.0 - 7.0
    } else {
        rect.left() + 11.0 + 7.0
    };
    mini_glyph(
        painter,
        Rect::from_center_size(pos2(gx, rect.center().y), vec2(14.0, 14.0)),
        kind,
        col.gamma_multiply(alpha),
    );
    let (anchor, x) = if rtl {
        (Align2::RIGHT_CENTER, gx - 12.0)
    } else {
        (Align2::LEFT_CENTER, gx + 12.0)
    };
    painter.text(
        pos2(x, rect.center().y),
        anchor,
        text,
        FontId::proportional(10.5),
        pal.text.gamma_multiply(0.92 * alpha),
    );
    let _ = pal;
}

pub fn chip_width(painter: &egui::Painter, pal: &Palette, text: &str) -> f32 {
    painter
        .layout_no_wrap(text.to_owned(), FontId::proportional(10.5), pal.text)
        .size()
        .x
        + 44.0
}

pub fn hex_image(
    painter: &egui::Painter,
    center: egui::Pos2,
    r: f32,
    tex: egui::TextureId,
    tint: Color32,
) {
    let mut mesh = egui::epaint::Mesh::with_texture(tex);
    mesh.colored_vertex(center, tint);
    mesh.vertices[0].uv = egui::epaint::WHITE_UV;
    mesh.vertices[0].uv = pos2(0.5, 0.5);
    let n = 6;
    for i in 0..=n {
        let a = std::f32::consts::TAU * i as f32 / n as f32 - std::f32::consts::FRAC_PI_2;
        let (dx, dy) = (a.cos(), a.sin());
        mesh.colored_vertex(pos2(center.x + r * dx, center.y + r * dy), tint);
        let last = mesh.vertices.len() - 1;
        mesh.vertices[last].uv = pos2(0.5 + dx * 0.5, 0.5 + dy * 0.5);
        if i > 0 {
            mesh.add_triangle(0, last as u32 - 1, last as u32);
        }
    }
    painter.add(egui::Shape::mesh(mesh));
}

pub fn monogram(
    painter: &egui::Painter,
    center: egui::Pos2,
    r: f32,
    pal: &Palette,
    text: &str,
    photo: Option<egui::TextureId>,
    col: Color32,
    t: f32,
    alpha: f32,
) {
    let mut outer = crate::theme::hex_points(center, r, 0.0);
    outer.push(outer[0]);
    painter.add(egui::Shape::convex_polygon(
        crate::theme::hex_points(center, r, 0.0),
        col.gamma_multiply(0.16 * alpha),
        egui::Stroke::NONE,
    ));
    painter.add(egui::Shape::line(
        outer,
        egui::Stroke::new(1.4, col.gamma_multiply(0.75 * alpha)),
    ));
    let spin = t * 0.35;
    for seg in 0..2 {
        let base = spin + std::f32::consts::TAU * seg as f32 / 2.0;
        painter.add(egui::Shape::line(
            arc_pts(
                center,
                r * 1.22,
                base,
                base + std::f32::consts::TAU * 0.16,
                12,
            ),
            egui::Stroke::new(1.4, col.gamma_multiply(0.40 * alpha)),
        ));
    }
    if let Some(tex) = photo {
        hex_image(
            painter,
            center,
            r * 0.94,
            tex,
            Color32::WHITE.gamma_multiply(alpha),
        );
    } else {
        painter.text(
            center,
            Align2::CENTER_CENTER,
            text,
            FontId::proportional(r * 0.78),
            pal.text.gamma_multiply(alpha),
        );
    }
}

pub fn glass_card(painter: &egui::Painter, rect: Rect, pal: &Palette, tint: Color32, alpha: f32) {
    painter.rect_filled(
        rect.translate(vec2(0.0, 10.0)).shrink(6.0),
        22.0,
        Color32::BLACK.gamma_multiply(0.17 * alpha),
    );
    painter.rect_filled(rect, 22.0, pal.panel.gamma_multiply(0.94 * alpha));
    gradient_rect(
        painter,
        Rect::from_min_size(rect.min, vec2(rect.width(), rect.height() * 0.62)),
        tint.gamma_multiply(0.055 * alpha),
        tint.gamma_multiply(0.0),
        22.0,
        10,
    );
    painter.rect_stroke(
        rect,
        22.0,
        egui::Stroke::new(1.0, pal.line.gamma_multiply(alpha)),
        StrokeKind::Inside,
    );
    let n = 16.0;
    for (a, b) in [
        (rect.left_top() + vec2(0.0, n), rect.left_top()),
        (rect.left_top(), rect.left_top() + vec2(n, 0.0)),
        (rect.right_bottom() - vec2(0.0, n), rect.right_bottom()),
        (rect.right_bottom(), rect.right_bottom() - vec2(n, 0.0)),
    ] {
        painter.line_segment(
            [a, b],
            egui::Stroke::new(1.6, tint.gamma_multiply(0.42 * alpha)),
        );
    }
}

pub fn nav_glyph(painter: &egui::Painter, rect: Rect, kind: u8, col: Color32) {
    let c = rect.center();
    let s = rect.width().min(rect.height()) * 0.5;
    let w = (s * 0.19).max(1.3);
    let st = egui::Stroke::new(w, col);
    match kind {
        0 => {
            for dx in [-s * 0.42, s * 0.42] {
                painter.circle_stroke(pos2(c.x + dx, c.y - s * 0.34), s * 0.30, st);
                painter.add(egui::Shape::line(
                    arc_pts(
                        pos2(c.x + dx, c.y + s * 0.74),
                        s * 0.56,
                        std::f32::consts::PI + 0.35,
                        std::f32::consts::TAU - 0.35,
                        10,
                    ),
                    st,
                ));
            }
        }
        1 => {
            painter.circle_stroke(c, s * 0.84, st);
            let mut half = Vec::new();
            half.push(c);
            for i in 0..=18 {
                let a = -std::f32::consts::FRAC_PI_2
                    + std::f32::consts::PI * i as f32 / 18.0;
                half.push(pos2(c.x + s * 0.84 * a.cos(), c.y + s * 0.84 * a.sin()));
            }
            painter.add(egui::Shape::convex_polygon(
                half,
                col.gamma_multiply(0.55),
                egui::Stroke::NONE,
            ));
        }
        2 => {
            for (sign, off) in [(-1.0_f32, -s * 0.40), (1.0, s * 0.40)] {
                let y = c.y + off;
                painter.line_segment(
                    [pos2(c.x - s * 0.72, y), pos2(c.x + s * 0.72, y)],
                    st,
                );
                let tip = if sign < 0.0 {
                    c.x + s * 0.72
                } else {
                    c.x - s * 0.72
                };
                let d = -sign * s * 0.34;
                painter.add(egui::Shape::line(
                    vec![
                        pos2(tip + d, y - s * 0.30),
                        pos2(tip, y),
                        pos2(tip + d, y + s * 0.30),
                    ],
                    st,
                ));
            }
        }
        3 => {
            let pts = vec![
                pos2(c.x, c.y - s),
                pos2(c.x + s * 0.80, c.y - s * 0.52),
                pos2(c.x + s * 0.80, c.y + s * 0.18),
                pos2(c.x, c.y + s),
                pos2(c.x - s * 0.80, c.y + s * 0.18),
                pos2(c.x - s * 0.80, c.y - s * 0.52),
            ];
            painter.add(egui::Shape::convex_polygon(
                pts,
                col.gamma_multiply(0.14),
                st,
            ));
            painter.circle_filled(pos2(c.x, c.y - s * 0.04), s * 0.17, col);
            painter.line_segment(
                [
                    pos2(c.x, c.y + s * 0.10),
                    pos2(c.x, c.y + s * 0.44),
                ],
                egui::Stroke::new(w * 1.1, col),
            );
        }
        4 => {
            painter.line_segment(
                [pos2(c.x, c.y - s * 0.82), pos2(c.x, c.y + s * 0.22)],
                st,
            );
            painter.add(egui::Shape::line(
                vec![
                    pos2(c.x - s * 0.38, c.y - s * 0.18),
                    pos2(c.x, c.y + s * 0.26),
                    pos2(c.x + s * 0.38, c.y - s * 0.18),
                ],
                st,
            ));
            painter.line_segment(
                [
                    pos2(c.x - s * 0.72, c.y + s * 0.72),
                    pos2(c.x + s * 0.72, c.y + s * 0.72),
                ],
                st,
            );
        }
        _ => {
            painter.circle_stroke(c, s * 0.86, st);
            painter.circle_filled(pos2(c.x, c.y - s * 0.40), w * 0.9, col);
            painter.line_segment(
                [
                    pos2(c.x, c.y - s * 0.08),
                    pos2(c.x, c.y + s * 0.48),
                ],
                egui::Stroke::new(w * 1.05, col),
            );
        }
    }
}

pub fn nav_item(
    ui: &mut egui::Ui,
    pal: &Palette,
    rtl: bool,
    kind: u8,
    label: &str,
    selected: bool,
) -> egui::Response {
    let (rect, resp) =
        ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::click());
    let hot = ui
        .ctx()
        .animate_bool_with_time(resp.id, resp.hovered(), crate::motion::hover_time());
    let sel = ui
        .ctx()
        .animate_bool_with_time(resp.id.with("s"), selected, crate::motion::hover_time());
    let p = ui.painter();
    p.rect_filled(
        rect,
        11.0,
        lerp_color(
            pal.panel_hi.gamma_multiply(0.35 + 0.9 * hot),
            pal.accent.gamma_multiply(0.18),
            sel,
        ),
    );
    if sel > 0.01 {
        let bar = if rtl {
            Rect::from_min_size(pos2(rect.right() - 3.0, rect.top() + 9.0), vec2(3.0, 22.0))
        } else {
            Rect::from_min_size(pos2(rect.left(), rect.top() + 9.0), vec2(3.0, 22.0))
        };
        p.rect_filled(bar, 2.0, pal.accent.gamma_multiply(sel));
    }
    let col = lerp_color(pal.muted.gamma_multiply(0.85 + 0.15 * hot), pal.accent, sel);
    let gx = if rtl {
        rect.right() - 24.0
    } else {
        rect.left() + 24.0
    };
    nav_glyph(
        p,
        Rect::from_center_size(pos2(gx, rect.center().y), vec2(17.0, 17.0)),
        kind,
        col,
    );
    let (anchor, x) = if rtl {
        (Align2::RIGHT_CENTER, gx - 16.0)
    } else {
        (Align2::LEFT_CENTER, gx + 16.0)
    };
    p.text(
        pos2(x, rect.center().y),
        anchor,
        label,
        FontId::proportional(12.5),
        lerp_color(pal.muted, pal.text, (0.25 + 0.75 * sel).max(hot)),
    );
    resp
}

pub fn pane_header(
    painter: &egui::Painter,
    rect: Rect,
    pal: &Palette,
    rtl: bool,
    title: &str,
    sub: &str,
    col: Color32,
) {
    let (anchor, x) = if rtl {
        (Align2::RIGHT_TOP, rect.right())
    } else {
        (Align2::LEFT_TOP, rect.left())
    };
    painter.text(
        pos2(x, rect.top()),
        anchor,
        title,
        FontId::proportional(17.0),
        pal.text,
    );
    painter.text(
        pos2(x, rect.top() + 24.0),
        anchor,
        sub,
        FontId::proportional(11.0),
        pal.muted,
    );
    let y = rect.top() + 48.0;
    let seg = 46.0;
    let (a, b) = if rtl {
        (rect.right() - seg, rect.right())
    } else {
        (rect.left(), rect.left() + seg)
    };
    painter.line_segment(
        [pos2(a, y), pos2(b, y)],
        egui::Stroke::new(2.0, col.gamma_multiply(0.85)),
    );
    let (c, d) = if rtl {
        (rect.left(), rect.right() - seg - 8.0)
    } else {
        (rect.left() + seg + 8.0, rect.right())
    };
    if d > c {
        painter.line_segment(
            [pos2(c, y), pos2(d, y)],
            egui::Stroke::new(1.0, pal.line),
        );
    }
}

pub fn initials(name: &str) -> String {
    let words: Vec<&str> = name.split_whitespace().take(2).collect();
    if words.is_empty() {
        return "?".to_string();
    }
    let mut out = String::new();
    for w in &words {
        if let Some(c) = w.chars().next() {
            out.extend(c.to_uppercase());
        }
    }
    out
}

pub fn profile_tile(
    ui: &mut egui::Ui,
    pal: &Palette,
    rect: Rect,
    key: &str,
    name: &str,
    sub: &str,
    col: Color32,
    locked: bool,
    t: f32,
    alpha: f32,
) -> egui::Response {
    let resp = ui.interact(rect, ui.id().with(("ptile", key)), Sense::click());
    let hot = ui
        .ctx()
        .animate_bool_with_time(resp.id, resp.hovered(), crate::motion::hover_time());
    let r = rect.translate(vec2(0.0, -6.0 * hot));
    let side = r.width().min(r.height() - 56.0) * 0.5;
    let center = pos2(r.center().x, r.top() + side + 6.0);

    {
        let p = ui.painter();
        for k in 0..3 {
            let f = ((t * 0.28 + k as f32 / 3.0).rem_euclid(1.0)).clamp(0.0, 1.0);
            let mut v = crate::theme::hex_points(center, side * (1.06 + f * 0.55), 0.0);
            v.push(v[0]);
            p.add(egui::Shape::line(
                v,
                egui::Stroke::new(
                    1.1,
                    col.gamma_multiply(0.22 * (1.0 - f) * (0.35 + 0.65 * hot) * alpha),
                ),
            ));
        }
        let spin = t * 0.45;
        for seg in 0..3 {
            let base = spin + std::f32::consts::TAU * seg as f32 / 3.0;
            p.add(egui::Shape::line(
                arc_pts(
                    center,
                    side * 1.20,
                    base,
                    base + std::f32::consts::TAU * 0.12,
                    12,
                ),
                egui::Stroke::new(1.6, col.gamma_multiply((0.30 + 0.55 * hot) * alpha)),
            ));
        }
        p.add(egui::Shape::convex_polygon(
            crate::theme::hex_points(center, side, 0.0),
            col.gamma_multiply((0.14 + 0.10 * hot) * alpha),
            egui::Stroke::NONE,
        ));
        let mut edge = crate::theme::hex_points(center, side, 0.0);
        edge.push(edge[0]);
        p.add(egui::Shape::line(
            edge,
            egui::Stroke::new(1.6, col.gamma_multiply((0.55 + 0.45 * hot) * alpha)),
        ));
        p.text(
            center,
            Align2::CENTER_CENTER,
            initials(name),
            FontId::proportional(side * 0.72),
            pal.text.gamma_multiply((0.82 + 0.18 * hot) * alpha),
        );
        if locked {
            let badge = pos2(center.x + side * 0.70, center.y + side * 0.70);
            p.circle_filled(badge, 13.0, pal.bg_deep.gamma_multiply(alpha));
            p.circle_stroke(
                badge,
                13.0,
                egui::Stroke::new(1.0, col.gamma_multiply(0.7 * alpha)),
            );
            mini_glyph(
                p,
                Rect::from_center_size(badge, vec2(13.0, 13.0)),
                1,
                col.gamma_multiply(alpha),
            );
        }
        p.text(
            pos2(r.center().x, r.bottom() - 30.0),
            Align2::CENTER_CENTER,
            name,
            FontId::proportional(15.0),
            pal.text.gamma_multiply((0.80 + 0.20 * hot) * alpha),
        );
        p.text(
            pos2(r.center().x, r.bottom() - 10.0),
            Align2::CENTER_CENTER,
            sub,
            FontId::proportional(10.5),
            pal.faint.gamma_multiply(alpha),
        );
    }
    resp
}

pub fn add_tile(
    ui: &mut egui::Ui,
    pal: &Palette,
    rect: Rect,
    label: &str,
    alpha: f32,
) -> egui::Response {
    let resp = ui.interact(rect, ui.id().with("ptile-add"), Sense::click());
    let hot = ui
        .ctx()
        .animate_bool_with_time(resp.id, resp.hovered(), crate::motion::hover_time());
    let r = rect.translate(vec2(0.0, -6.0 * hot));
    let side = r.width().min(r.height() - 56.0) * 0.5;
    let center = pos2(r.center().x, r.top() + side + 6.0);
    let p = ui.painter();
    let mut edge = crate::theme::hex_points(center, side, 0.0);
    edge.push(edge[0]);
    p.add(egui::Shape::line(
        edge,
        egui::Stroke::new(
            1.4,
            pal.line_hi.gamma_multiply((0.55 + 0.45 * hot) * alpha),
        ),
    ));
    let arm = side * 0.30;
    for (a, b) in [
        (pos2(center.x - arm, center.y), pos2(center.x + arm, center.y)),
        (pos2(center.x, center.y - arm), pos2(center.x, center.y + arm)),
    ] {
        p.line_segment(
            [a, b],
            egui::Stroke::new(2.2, pal.muted.gamma_multiply((0.7 + 0.3 * hot) * alpha)),
        );
    }
    p.text(
        pos2(r.center().x, r.bottom() - 30.0),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(14.0),
        pal.muted.gamma_multiply((0.75 + 0.25 * hot) * alpha),
    );
    resp
}

pub fn panel_card(painter: &egui::Painter, rect: Rect, pal: &Palette) {
    painter.rect_filled(
        rect.translate(vec2(0.0, 6.0)).shrink(3.0),
        18.0,
        Color32::BLACK.gamma_multiply(0.16),
    );
    painter.rect_filled(rect, 18.0, pal.panel.gamma_multiply(0.92));
    painter.rect_stroke(
        rect,
        18.0,
        egui::Stroke::new(1.0, pal.line),
        StrokeKind::Inside,
    );
}

/// وجه بطاقة اللوحة: شارة، عنوان، نصّ، زرّ، رابط
pub type Face = [String; 5];

/// بطاقة اللوحة، وتتحرّك كما تتحرّك بطاقة الشاشة الأولى: تنسحب القديمة
/// إلى جهة وتدخل الجديدة من الأخرى، ويهدأ التأشير عليها بلا قفزة
///
/// ترجع أنها نُقرت وأن المؤشّر فوقها، والثاني يوقف الدوران ما دام يقرؤها
#[allow(clippy::too_many_arguments)]
pub fn promo_card(
    ui: &mut egui::Ui,
    rect: Rect,
    pal: &Palette,
    rtl: bool,
    tint: Color32,
    cur: &Face,
    prev: Option<&Face>,
    phase: f32,
    alpha: f32,
) -> (bool, bool) {
    if alpha <= 0.004 || rect.height() < 34.0 {
        return (false, false);
    }
    let clickable = !cur[3].trim().is_empty() && !cur[4].trim().is_empty();
    let res = ui.interact(
        rect,
        egui::Id::new("promo"),
        if clickable {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let hot = ui
        .ctx()
        .animate_bool_with_time(res.id, clickable && res.hovered(), crate::motion::hover_time());
    if clickable && res.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    {
        let p = ui.painter();
        p.rect_filled(
            rect,
            16.0,
            pal.panel_hi.gamma_multiply((0.55 + 0.25 * hot) * alpha),
        );
        p.rect_stroke(
            rect,
            16.0,
            egui::Stroke::new(
                1.0,
                lerp_color(pal.line, tint.gamma_multiply(0.55), hot).gamma_multiply(alpha),
            ),
            StrokeKind::Inside,
        );
    }

    // الحساب نفسه الذي تنزلق به بطاقة الشاشة الأولى
    let dir = if rtl { -1.0 } else { 1.0 };
    let leaving = 1.0 - (phase / 0.45).clamp(0.0, 1.0);
    let arriving = ((phase - 0.35) / 0.65).clamp(0.0, 1.0);
    let p = ui.painter().with_clip_rect(rect);
    if let Some(old) = prev.filter(|_| leaving > 0.001) {
        promo_face(&p, rect, pal, rtl, tint, old, leaving * alpha, -dir * 18.0 * (1.0 - leaving), 0.0);
    }
    promo_face(
        &p,
        rect,
        pal,
        rtl,
        tint,
        cur,
        arriving * alpha,
        dir * 18.0 * (1.0 - ease_out(arriving)),
        hot,
    );

    (clickable && res.clicked(), res.hovered())
}

#[allow(clippy::too_many_arguments)]
fn promo_face(
    p: &egui::Painter,
    rect: Rect,
    pal: &Palette,
    rtl: bool,
    tint: Color32,
    face: &Face,
    alpha: f32,
    dx: f32,
    hot: f32,
) {
    if alpha <= 0.004 {
        return;
    }
    let [badge, title, body, cta, _] = face;
    let pad = 14.0;
    let x = if rtl { rect.right() - pad } else { rect.left() + pad } + dx;
    let anchor = if rtl { Align2::RIGHT_TOP } else { Align2::LEFT_TOP };
    let mut y = rect.top() + 11.0;

    if !badge.trim().is_empty() {
        p.text(
            pos2(x, y),
            anchor,
            badge,
            FontId::proportional(9.5),
            tint.gamma_multiply(0.95 * alpha),
        );
        y += 14.0;
    }
    p.text(
        pos2(x, y),
        anchor,
        title,
        FontId::proportional(13.5),
        pal.text.gamma_multiply(alpha),
    );
    y += 20.0;

    let has_cta = !cta.trim().is_empty();
    // نصّ اللوحة يُكتب من بعيد وقد يطول، فيُقصّ على ما يتّسع بدل أن يسقط
    let room = rect.bottom() - y - if has_cta { 40.0 } else { 10.0 };
    if !body.trim().is_empty() && room > 10.0 {
        let galley = p.layout(
            body.clone(),
            FontId::proportional(10.5),
            pal.muted.gamma_multiply(0.95 * alpha),
            rect.width() - pad * 2.0,
        );
        let at = if rtl {
            pos2(x - galley.size().x, y)
        } else {
            pos2(x, y)
        };
        // يقف عند آخر سطر يتّسع كاملًا، فلا يظهر نصف سطر مشقوق
        let shown = galley
            .rows
            .iter()
            .map(|r| r.rect().max.y)
            .filter(|&bottom| bottom <= room)
            .fold(0.0, f32::max);
        if shown > 0.0 {
            p.with_clip_rect(Rect::from_min_size(
                pos2(rect.left(), y),
                vec2(rect.width(), shown),
            ))
            .galley(at, galley, pal.muted);
        }
    }

    // الزرّ في مكانه نفسه على كل بطاقة، لا يتبع طول النصّ فيقفز بينها
    if has_cta {
        let g = p.layout_no_wrap(
            cta.clone(),
            FontId::proportional(10.5),
            tint.gamma_multiply(alpha),
        );
        let w = g.size().x + 22.0;
        let r = Rect::from_min_size(
            pos2(if rtl { x - w } else { x }, rect.bottom() - 12.0 - 22.0),
            vec2(w, 22.0),
        );
        p.rect_filled(
            r,
            999.0,
            tint.gamma_multiply((0.14 + 0.12 * hot) * alpha),
        );
        p.galley(r.center() - g.size() * 0.5, g, tint);
    }
}
