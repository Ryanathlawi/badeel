use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::time::Instant;

use eframe::egui::{self, Align, Align2, Color32, FontId, Layout, Rect, RichText, Sense, pos2, vec2};

use crate::core::catalog::{self, Platform};
use crate::core::update::{self, Progress};
use crate::core::vault::{self, VaultKey};
use crate::about;
use crate::core::profile::{self, Book};
use crate::core::{bnet, board, paths, presence, procs, steam, store, switch, transfer};
use crate::i18n::Lang;
use crate::motion::{self, Motion, back_out, ease_out};
use crate::showcase::{self, Showcase};
use crate::theme::{self, Accent, Palette};
use crate::ui;

const APP_KEY: &str = "badeel.settings";

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    pub lang: Lang,
    pub launch_after_switch: bool,
    pub close_after_switch: bool,
    #[serde(default)]
    pub seen_tour: bool,
    #[serde(default = "yes")]
    pub skip_steam_chooser: bool,
    #[serde(default = "yes")]
    pub animations: bool,
    #[serde(default = "yes")]
    pub live_backdrop: bool,
    #[serde(default = "yes")]
    pub showcase: bool,
    #[serde(default = "yes")]
    pub confirm_switch: bool,
    #[serde(default)]
    pub mask_ids: bool,
    #[serde(default)]
    pub auto_lock_min: u32,
    #[serde(default)]
    pub lock_on_minimize: bool,
    #[serde(default = "yes")]
    pub auto_update: bool,
    #[serde(default)]
    pub discord: bool,
    #[serde(default = "yes")]
    pub board: bool,
}

/// كم تبقى بطاقة اللوحة قبل أن تدور، أطول من بطاقة الشاشة الأولى لأنها
/// بجوار قائمة يختار منها المستخدم فلا يليق بها أن تشغله
const PROMO_DWELL: f32 = 9.0;

fn yes() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            lang: Lang::Ar,
            launch_after_switch: true,
            close_after_switch: false,
            seen_tour: false,
            skip_steam_chooser: true,
            animations: true,
            live_backdrop: true,
            showcase: true,
            confirm_switch: true,
            mask_ids: false,
            auto_lock_min: 0,
            lock_on_minimize: false,
            auto_update: true,
            discord: false,
            board: true,
        }
    }
}

pub struct Toast {
    text: String,
    kind: ToastKind,
    born: Instant,
}

#[derive(PartialEq, Clone, Copy)]
pub enum ToastKind {
    Ok,
    Err,
}

pub enum Dialog {
    None,
    Find { query: String, pick: usize },
    Export { a: String, b: String, error: String },
    Import { file: std::path::PathBuf, password: String, error: String },
    AddAccount { name: String, error: String },
    Rename { id: String, name: String },
    Confirm { id: String, name: String },
    Unlock { password: String, error: String },
    SetPassword { a: String, b: String, error: String },
    UpdateFound(update::Available),
    ConfirmSwitch { id: String, name: String },
    NewProfile { name: String, error: String },
    RenameProfile { id: String, name: String },
    DropProfile { id: String, name: String },
}

#[derive(PartialEq, Clone, Copy, Debug)]
enum Route {
    Home,
    Accounts,
    About,
    Settings,
}

struct BusyJob {
    rx: Receiver<JobMsg>,
    label: String,
    started: Instant,
}

enum JobMsg {
    Step(switch::Step),
    Done(Result<String, String>),
}

const TOUR_STEPS: usize = 5;

const FRAME_MS: u64 = 16;

pub struct App {
    settings: Settings,
    pal: Palette,
    key: Option<Arc<VaultKey>>,

    route: Route,
    settings_nav: f32,
    selected: Option<String>,

    platform: &'static Platform,
    accounts: store::Index,
    current_id: Option<String>,
    installed: Vec<bool>,
    counts: Vec<usize>,

    dialog: Dialog,
    tour: Option<usize>,
    tour_t: f32,
    toasts: Vec<Toast>,

    busy: Option<BusyJob>,
    step: Option<switch::Step>,

    update_rx: Option<Receiver<Progress>>,
    update_state: Option<Progress>,
    update_ready: bool,

    book: Book,
    accent: Accent,
    home_in: f32,
    last_input: Instant,
    curtain: f32,
    picking: bool,
    pick_in: f32,
    about_in: f32,
    settings_tab: u8,
    has_pw: bool,
    motion: Motion,
    show: Showcase,
    t0: Instant,
    last_frame: Instant,
    intro: f32,
    list_in: f32,
    search: String,
    refresh_at: Instant,
    presence: presence::Presence,
    board: board::Board,
    board_rx: Option<Receiver<anyhow::Result<board::Board>>>,
    promo: Showcase,
    promo_hot: bool,
    find: Vec<Hit>,
    transfer_rx: Option<Receiver<(bool, Result<String, String>)>>,
    bnet_why: Option<bnet::Empty>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let settings: Settings = cc
            .storage
            .and_then(|s| eframe::get_value(s, APP_KEY))
            .unwrap_or_default();
        let book = profile::load();
        let accent = Accent::from_index(book.current().accent);
        theme::apply(&cc.egui_ctx, accent);
        egui_extras::install_image_loaders(&cc.egui_ctx);

        let locked = vault::state() == vault::State::Locked;
        let key = (!locked)
            .then(|| vault::open(None).ok().map(Arc::new))
            .flatten();

        let installed: Vec<bool> = catalog::PLATFORMS.iter().map(procs::installed).collect();
        let counts: Vec<usize> = catalog::PLATFORMS
            .iter()
            .map(|p| accounts_of(p).accounts.len())
            .collect();
        let tour = (!settings.seen_tour).then_some(0);

        let auto_update = settings.auto_update;
        let mut app = Self {
            pal: theme::palette(accent),
            settings,
            key,
            route: Route::Home,
            settings_nav: 0.0,
            selected: None,
            platform: catalog::PLATFORMS
                .iter()
                .enumerate()
                .max_by_key(|(i, _)| counts[*i] * 2 + usize::from(installed[*i]))
                .map(|(_, p)| p)
                .unwrap_or(&catalog::PLATFORMS[0]),
            accounts: store::Index::default(),
            current_id: None,
            installed,
            counts,
            dialog: if locked {
                Dialog::Unlock {
                    password: String::new(),
                    error: String::new(),
                }
            } else {
                Dialog::None
            },
            tour,
            tour_t: 0.0,
            toasts: Vec::new(),
            busy: None,
            step: None,
            update_rx: auto_update.then(update::spawn_check),
            update_state: None,
            update_ready: false,
            book,
            accent,
            home_in: 0.0,
            last_input: Instant::now(),
            curtain: 0.0,
            picking: false,
            pick_in: 0.0,
            about_in: 0.0,
            settings_tab: 0,
            has_pw: locked,
            motion: Motion {
                enabled: true,
                backdrop: true,
            },
            show: Showcase::default(),
            t0: Instant::now(),
            last_frame: Instant::now(),
            intro: 0.0,
            list_in: 0.0,
            search: String::new(),
            refresh_at: Instant::now(),
            presence: presence::Presence::new(),
            board: board::load(),
            board_rx: None,
            promo: Showcase::default(),
            promo_hot: false,
            find: Vec::new(),
            transfer_rx: None,
            bnet_why: None,
        };
        app.picking = app.book.profiles.len() > 1;
        app.presence.set_enabled(app.settings.discord);
        if app.settings.board {
            app.fetch_board();
        }
        app.reload();
        app
    }

    /// سطرا نشاط ديسكورد. لا اسم حساب فيهما ولا اسم منصّة، فما يظهر
    /// لأصدقائك أن بديل مفتوح عندك لا أكثر.
    ///
    /// وهما بالإنجليزية دائمًا مهما كانت لغة الواجهة، لأن من يقرؤهما
    /// أصدقاؤك في ديسكورد لا أنت.
    fn tell_discord(&mut self) {
        if !self.settings.discord {
            return;
        }
        let state = if self.busy.is_some() || self.step.is_some() {
            "Switching an account"
        } else if self.key.is_none() {
            "Vault locked"
        } else {
            "Vault unlocked and encrypted"
        };
        self.presence
            .show("Game account switcher", state, "Website", "Source");
    }

    /// النافذة بلا إطار من النظام، فحوافها لا تُسحب بنفسها. هذه تتكفّل
    /// بذلك: تتحسّس المؤشر عند الحافة، وتغيّر شكله، وتسلّم ويندوز
    /// عملية التكبير والتصغير بمجرّد الضغط.
    fn edge_resize(ctx: &egui::Context) {
        use egui::{CursorIcon as C, ResizeDirection as D, ViewportCommand};
        const EDGE: f32 = 6.0;

        let rect = ctx.viewport_rect();
        let Some(pos) = ctx.pointer_latest_pos() else {
            return;
        };
        if !rect.contains(pos) {
            return;
        }
        let w = pos.x - rect.left() < EDGE;
        let e = rect.right() - pos.x < EDGE;
        let n = pos.y - rect.top() < EDGE;
        let s = rect.bottom() - pos.y < EDGE;

        let (dir, cur) = match (w, e, n, s) {
            (true, _, true, _) => (D::NorthWest, C::ResizeNwSe),
            (_, true, _, true) => (D::SouthEast, C::ResizeNwSe),
            (_, true, true, _) => (D::NorthEast, C::ResizeNeSw),
            (true, _, _, true) => (D::SouthWest, C::ResizeNeSw),
            (true, ..) => (D::West, C::ResizeHorizontal),
            (_, true, ..) => (D::East, C::ResizeHorizontal),
            (_, _, true, _) => (D::North, C::ResizeVertical),
            (_, _, _, true) => (D::South, C::ResizeVertical),
            _ => return,
        };

        ctx.set_cursor_icon(cur);
        if ctx.input(|i| i.pointer.primary_pressed()) {
            ctx.send_viewport_cmd(ViewportCommand::BeginResize(dir));
        }
    }

    fn pace(&self, ctx: &egui::Context, frame_start: Instant, budget_ms: u64) {
        if budget_ms == 0 {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
            return;
        }
        let budget = std::time::Duration::from_millis(budget_ms);
        let spent = frame_start.elapsed();
        if spent < budget {
            std::thread::sleep(budget - spent);
        }
        ctx.request_repaint();
    }

    fn go(&mut self, route: Route) {
        if self.route == route {
            return;
        }
        self.route = route;
        if route != Route::Settings {
            self.curtain = 1.0;
        }
    }

    fn restyle(&mut self, ctx: &egui::Context) {
        self.accent = Accent::from_index(self.book.current().accent);
        self.pal = theme::palette(self.accent);
        theme::apply(ctx, self.accent);
    }

    /// يجلب اللوحة في خيط مستقل، فلا ينتظرها الفتح ولا يعطّلها انقطاع الشبكة
    fn fetch_board(&mut self) {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(board::refresh());
        });
        self.board_rx = Some(rx);
    }

    fn poll_board(&mut self) {
        let Some(rx) = &self.board_rx else { return };
        match rx.try_recv() {
            Ok(Ok(b)) => {
                // المحفوظ يظهر أوّلًا ثم يصل الجديد بعد لحظة، فإن اختلف دخل
                // بحركة البطاقة نفسها بدل أن يقفز فوق القديم
                if b != self.board {
                    self.board = b;
                    self.promo.t = 0.0;
                    self.promo.prev = usize::MAX;
                }
                self.board_rx = None;
            }
            Ok(Err(e)) => {
                log::warn!("لم تصل اللوحة ({e:#}) — تبقى النسخة المحفوظة");
                self.board_rx = None;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(_) => self.board_rx = None,
        }
    }

    fn reload(&mut self) {
        self.accounts = accounts_of(self.platform);
        self.current_id = switch::current_id(self.platform);
        self.bnet_why = (matches!(self.platform.identity, catalog::Identity::Bnet)
            && self.accounts.accounts.is_empty())
        .then(bnet::why_empty);
        if let Some(bnet::Empty::Unreadable(e)) = &self.bnet_why {
            log::warn!("إعدادات باتل نت لم تُقرأ: {e}");
        }
        let i = catalog::index_of(self.platform.id);
        if let Some(c) = self.counts.get_mut(i) {
            *c = self.accounts.accounts.len();
        }
        if self
            .selected
            .as_ref()
            .is_none_or(|id| self.accounts.get(id).is_none())
        {
            self.selected = self.current_id.clone();
        }
    }

    fn toast(&mut self, text: impl Into<String>, kind: ToastKind) {
        self.toasts.push(Toast {
            text: text.into(),
            kind,
            born: Instant::now(),
        });
    }

    fn open_platform(&mut self, p: &'static Platform) {
        self.curtain = 1.0;
        self.platform = p;
        self.list_in = 0.0;
        self.selected = None;
        self.search.clear();
        self.promo = Showcase::default();
        self.reload();
        self.offer_current();
    }

    /// التصدير والاستيراد يشتقّان مفتاحًا بـ Argon2 ويفكّان ويشفّران كل ملف،
    /// فيعملان في خيط مستقل ولا تتجمّد النافذة
    fn start_transfer(&mut self, import: bool, file: std::path::PathBuf, password: String) {
        let Some(key) = self.key.clone() else { return };
        let lang = self.settings.lang;
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let password = zeroize::Zeroizing::new(password);
            let r = if import {
                transfer::import(&key, &password, &file).map(|m| match (lang.rtl(), m.skipped) {
                    (true, 0) => format!("تمت إضافة {} حساب", m.added),
                    (true, k) => format!("تمت إضافة {} حساب، و{k} موجود أصلًا", m.added),
                    (false, 0) => format!("Added {} accounts", m.added),
                    (false, k) => format!("Added {} accounts, {k} were already here", m.added),
                })
            } else {
                transfer::export(&key, &password, &file).map(|n| {
                    if lang.rtl() {
                        format!("تم التصدير، {n} حساب في الملف")
                    } else {
                        format!("Exported {n} accounts")
                    }
                })
            };
            let _ = tx.send((import, r.map_err(|e| format!("{e:#}"))));
        });
        self.transfer_rx = Some(rx);
        self.toast(
            if import {
                lang.t("جارٍ الاستيراد", "Importing")
            } else {
                lang.t("جارٍ التصدير", "Exporting")
            },
            ToastKind::Ok,
        );
    }

    fn poll_transfer(&mut self) {
        let Some(rx) = &self.transfer_rx else { return };
        let (import, result) = match rx.try_recv() {
            Ok(got) => got,
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(_) => {
                self.transfer_rx = None;
                return;
            }
        };
        self.transfer_rx = None;
        match result {
            Ok(msg) => {
                self.toast(msg, ToastKind::Ok);
                if import {
                    self.counts = catalog::PLATFORMS
                        .iter()
                        .map(|p| accounts_of(p).accounts.len())
                        .collect();
                    self.reload();
                }
            }
            Err(e) => self.toast(e, ToastKind::Err),
        }
    }

    /// باتل نت يُغلق أوّلًا، وإلا كتب عند خروجه قيمته القديمة فوق ما كتبناه
    fn remember_bnet(&mut self) {
        let lang = self.settings.lang;
        let done = procs::close_all(self.platform.exes, self.platform.close)
            .and_then(|_| bnet::remember_names());
        match done {
            Ok(()) => self.toast(
                lang.t(
                    "تم، افتح باتل نت وسجّل دخولك مرة وبيطلع حسابك هنا",
                    "Done. Open Battle.net and sign in once, and your account shows up here",
                ),
                ToastKind::Ok,
            ),
            Err(e) => self.toast(format!("{e:#}"), ToastKind::Err),
        }
        self.reload();
    }

    /// يجمع حسابات كل المنصّات المثبّتة مرّة عند الفتح، لا في كل إطار
    fn open_find(&mut self) {
        let installed = self.installed.clone();
        self.find = catalog::PLATFORMS
            .iter()
            .enumerate()
            .filter(|(i, _)| installed.get(*i).copied().unwrap_or(false))
            .flat_map(|(_, p)| {
                let live = switch::current_id(p);
                accounts_of(p).accounts.into_iter().map(move |acc| Hit {
                    live: live.as_deref() == Some(acc.id.as_str()),
                    plat: p,
                    acc,
                })
            })
            .collect();
        self.dialog = Dialog::Find {
            query: String::new(),
            pick: 0,
        };
    }

    /// يفتح منصّة الحساب ويحدّده، ويبدّل له إن لم يكن هو المسجَّل دخوله
    fn go_to_hit(&mut self, plat: &'static Platform, id: String, live: bool) {
        self.go(Route::Accounts);
        if plat.id != self.platform.id {
            self.open_platform(plat);
        }
        self.selected = Some(id.clone());
        if !live {
            self.ask_switch(id);
        }
    }

    /// ستيم وباتل نت يحفظان قائمة حساباتهما فيقرأها بديل وحده، وبقية المنصّات
    /// لا تحفظ إلا جلسة واحدة بلا اسم، فأقرب ما يشبه ذلك أن يرى بديل الجلسة
    /// القائمة عند أول زيارة ويطلب لها اسمًا بدل أن ينتظر المستخدم يبحث عن الزر
    fn offer_current(&mut self) {
        if self.platform.identity.own_list()
            || self.key.is_none()
            || !matches!(self.dialog, Dialog::None)
            || !self.accounts.accounts.is_empty()
            || !switch::session_live(self.platform)
        {
            return;
        }
        self.dialog = Dialog::AddAccount {
            name: String::new(),
            error: String::new(),
        };
    }

    fn ask_switch(&mut self, account_id: String) {
        if !self.settings.confirm_switch {
            self.start_switch(account_id);
            return;
        }
        let name = self
            .accounts
            .get(&account_id)
            .map(|a| a.name.clone())
            .unwrap_or_default();
        self.dialog = Dialog::ConfirmSwitch {
            id: account_id,
            name,
        };
    }

    fn start_switch(&mut self, account_id: String) {
        // التبديل يُغلق المنصّة بالقوة، ولو كانت مباراة جارية خرج منها
        // اللاعب وأخذ عقوبة، فيُوقف قبل أن يمسّ شيئًا
        if let Some(g) = procs::game_running(self.platform) {
            let rtl = self.settings.lang.rtl();
            let msg = if rtl {
                format!("{} شغّالة الآن، أغلقها أولًا", g.name(true))
            } else {
                format!("{} is running — close it first", g.name(false))
            };
            self.toast(msg, ToastKind::Err);
            return;
        }
        let Some(key) = self.key.clone() else {
            self.toast(
                self.settings.lang.t("افتح القفل أولًا", "Unlock first"),
                ToastKind::Err,
            );
            return;
        };
        let p = self.platform;
        let launch = self.settings.launch_after_switch;
        let skip_chooser = self.settings.skip_steam_chooser;
        let name = self
            .accounts
            .get(&account_id)
            .map(|a| a.name.clone())
            .unwrap_or_default();
        let (tx, rx) = std::sync::mpsc::channel();
        let tx2 = tx.clone();
        std::thread::spawn(move || {
            let r = switch::switch_to(p, &account_id, launch, skip_chooser, &key, |s| {
                let _ = tx2.send(JobMsg::Step(s));
            });
            let _ = tx.send(JobMsg::Done(match r {
                Ok(()) => Ok(name),
                Err(e) => Err(format!("{e:#}")),
            }));
        });
        self.busy = Some(BusyJob {
            rx,
            label: self.settings.lang.t("جارٍ التبديل", "Switching").into(),
            started: Instant::now(),
        });
        self.step = Some(switch::Step::Closing);
    }

    fn add_current(&mut self, name: String) {
        let Some(key) = self.key.clone() else { return };
        let p = self.platform;
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let r = switch::add_current(p, &name, &key);
            let _ = tx.send(JobMsg::Done(match r {
                Ok(a) => Ok(a.name),
                Err(e) => Err(format!("{e:#}")),
            }));
        });
        self.busy = Some(BusyJob {
            rx,
            label: self.settings.lang.t("جارٍ الحفظ", "Saving").into(),
            started: Instant::now(),
        });
        self.step = Some(switch::Step::Saving);
    }

    fn poll_jobs(&mut self, ctx: &egui::Context) {
        let mut finished = None;
        if let Some(job) = &self.busy {
            while let Ok(msg) = job.rx.try_recv() {
                match msg {
                    JobMsg::Step(s) => self.step = Some(s),
                    JobMsg::Done(r) => finished = Some(r),
                }
            }
            ctx.request_repaint();
        }
        if let Some(result) = finished {
            self.busy = None;
            self.step = None;
            match result {
                Ok(name) => {
                    let msg = format!(
                        "{} {name}",
                        self.settings.lang.t("تم التبديل إلى", "Switched to")
                    );
                    self.toast(msg, ToastKind::Ok);
                    self.reload();
                    if self.settings.close_after_switch {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
                Err(e) => self.toast(e, ToastKind::Err),
            }
        }

        let mut update_msg = None;
        if let Some(rx) = &self.update_rx {
            while let Ok(p) = rx.try_recv() {
                update_msg = Some(p);
            }
        }
        if let Some(p) = update_msg {
            match &p {
                Progress::Found(u) => {
                    self.dialog = Dialog::UpdateFound(u.clone());
                    self.update_state = Some(p);
                }
                Progress::Ready => {
                    self.update_ready = true;
                    self.update_state = Some(p);
                    self.toast(
                        self.settings
                            .lang
                            .t("التحديث جاهز — أعد التشغيل", "Update ready — restart"),
                        ToastKind::Ok,
                    );
                }
                Progress::Failed(e) => {
                    log::warn!("update: {e}");
                    self.update_state = None;
                }
                Progress::UpToDate => self.update_state = None,
                _ => self.update_state = Some(p),
            }
            ctx.request_repaint();
        }

        if self.busy.is_none() && self.route == Route::Accounts && Instant::now() > self.refresh_at {
            self.refresh_at = Instant::now() + std::time::Duration::from_secs(4);
            self.current_id = switch::current_id(self.platform);
        }
    }

    fn guard(&mut self, ctx: &egui::Context) {
        let busy = ctx.input(|i| {
            i.pointer.has_pointer() && i.pointer.velocity().length() > 0.5
                || !i.events.is_empty()
                || i.pointer.any_down()
        });
        if busy {
            self.last_input = Instant::now();
        }
        if self.key.is_none() || !matches!(self.dialog, Dialog::None) {
            return;
        }
        let minimized = ctx.input(|i| i.viewport().minimized.unwrap_or(false));
        let idle = self.last_input.elapsed().as_secs();
        let by_idle =
            self.settings.auto_lock_min > 0 && idle >= self.settings.auto_lock_min as u64 * 60;
        let by_min = self.settings.lock_on_minimize && minimized;
        if (by_idle || by_min) && self.has_pw {
            self.key = None;
            self.dialog = Dialog::Unlock {
                password: String::new(),
                error: String::new(),
            };
        }
    }

    fn hotkeys(&mut self, ui: &egui::Ui) {
        if !matches!(self.dialog, Dialog::None) || self.tour.is_some() || self.picking {
            return;
        }
        if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::K)) {
            self.open_find();
            return;
        }
        // من يكتب في خانة لا يقصد اختصارًا، وكان حرف l في بحث الحسابات يقفل
        // الخزنة وحرف s يفتح الإعدادات ورقم ينقل المنصّة
        if ui.ctx().text_edit_focused() {
            return;
        }
        let (esc, digit, gear, lock) = ui.input(|i| {
            let mut d = None;
            for (n, k) in [
                (1usize, egui::Key::Num1),
                (2, egui::Key::Num2),
                (3, egui::Key::Num3),
                (4, egui::Key::Num4),
                (5, egui::Key::Num5),
                (6, egui::Key::Num6),
                (7, egui::Key::Num7),
                (8, egui::Key::Num8),
            ] {
                if i.key_pressed(k) {
                    d = Some(n);
                }
            }
            (
                i.key_pressed(egui::Key::Escape),
                d,
                i.key_pressed(egui::Key::S),
                i.key_pressed(egui::Key::L),
            )
        });
        if esc {
            match self.route {
                Route::Settings | Route::Accounts | Route::About => self.go(Route::Home),
                Route::Home => ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close),
            }
        }
        if let Some(n) = digit {
            if let Some(p) = catalog::PLATFORMS.get(n - 1) {
                self.go(Route::Accounts);
                if p.id != self.platform.id {
                    self.open_platform(p);
                } else {
                    self.list_in = 0.0;
                }
            }
        }
        if gear {
            self.go(if self.route == Route::Settings {
                Route::Home
            } else {
                Route::Settings
            });
        }
        if lock && self.key.is_some() {
            self.key = None;
            self.dialog = Dialog::Unlock {
                password: String::new(),
                error: String::new(),
            };
        }
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, APP_KEY, &self.settings);
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        theme::palette(self.accent).bg.to_normalized_gamma_f32()
    }

    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = root.ctx().clone();
        let ctx = &ctx;
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;
        let t = (now - self.t0).as_secs_f32();
        self.pal = theme::palette(self.accent);

        let mo = Motion {
            enabled: self.settings.animations,
            backdrop: self.settings.animations && self.settings.live_backdrop,
        };
        self.motion = mo;
        motion::set_enabled(mo.enabled);

        let set_target = if self.route == Route::Settings { 1.0 } else { 0.0 };
        self.settings_nav = mo.step(self.settings_nav, set_target, dt, motion::SPEED_PANEL);
        self.intro = mo.step(self.intro, 1.0, dt, motion::SPEED_INTRO);
        self.list_in = mo.step(self.list_in, 1.0, dt, motion::SPEED_LIST);
        let home_target = if self.route == Route::Home { 1.0 } else { 0.0 };
        self.home_in = mo.step(self.home_in, home_target, dt, motion::SPEED_PANEL);
        let about_target = if self.route == Route::About { 1.0 } else { 0.0 };
        self.about_in = mo.step(self.about_in, about_target, dt, motion::SPEED_PANEL);
        let pick_target = if self.picking { 1.0 } else { 0.0 };
        self.pick_in = mo.step(self.pick_in, pick_target, dt, motion::SPEED_PANEL);
        self.show.tick(dt, mo);
        if self.route == Route::Accounts && self.settings.board {
            if self.promo_hot {
                self.promo.hold = 0.0;
            }
            let n = board::pool(&self.board, self.platform.id).len();
            self.promo.tick_n(dt, mo, n, PROMO_DWELL);
        }
        self.tell_discord();
        if self.curtain > 0.0 {
            self.curtain = if mo.enabled {
                (self.curtain - dt / 0.78).max(0.0)
            } else {
                0.0
            };
        }

        self.poll_jobs(ctx);
        self.poll_board();
        self.poll_transfer();
        self.hotkeys(root);
        self.guard(ctx);

        let settling = self.settings_nav != set_target
            || self.home_in != home_target
            || self.about_in != about_target
            || self.pick_in != pick_target
            || self.intro < 1.0
            || self.list_in < 1.0
            || self.busy.is_some()
            || !self.toasts.is_empty()
            || self.show.moving()
            || self.promo.moving()
            || self.curtain > 0.0
            || self.tour.is_some();
        let (focused, pointer, minimized) = ctx.input(|i| {
            (
                i.focused,
                i.pointer.has_pointer(),
                i.viewport().minimized.unwrap_or(false),
            )
        });
        let budget = if minimized {
            0
        } else if settling || (pointer && focused) {
            FRAME_MS
        } else if mo.enabled {
            FRAME_MS * 2
        } else {
            0
        };
        Self::edge_resize(ctx);
        self.pace(ctx, now, budget);
        let bt = if mo.backdrop { t } else { 0.0 };
        theme::draw_backdrop(root.painter(), root.max_rect(), &self.pal, bt, mo.backdrop);

        self.titlebar(root);
        self.header(root, t);
        self.footer(root);
        self.rail(root, t);

        let content = root.available_rect_before_wrap();
        let on_home = self.route == Route::Home
            || self.route == Route::About
            || self.home_in > 0.02
            || self.about_in > 0.02;
        if !on_home {
            self.list_panel(root, t);
            self.showcase_panel(root, t);
        }

        let stage = root.available_rect_before_wrap();
        let home_a = ease_out(self.home_in);
        let about_a = ease_out(self.about_in);
        if home_a > 0.004 {
            self.home_screen(root, stage, t, home_a * (1.0 - about_a));
        }
        if about_a > 0.004 {
            self.about_screen(root, stage, t, about_a);
        }
        if home_a < 0.996 && about_a < 0.004 {
            self.stage(root, stage, t, 1.0 - home_a);
        }
        if self.curtain > 0.0015 {
            let c = self.curtain;
            let veil = (c / 0.34).clamp(0.0, 1.0);
            let mark_a = ((c - 0.30) / 0.26).clamp(0.0, 1.0);
            let p = root.painter();
            p.rect_filled(content, 0.0, self.pal.bg.gamma_multiply(0.94 * veil));
            let r = (content.height() * 0.09).clamp(26.0, 40.0);
            ui::loader(
                p,
                pos2(content.center().x, content.center().y - r * 0.4),
                r,
                &self.pal,
                t,
                ease_out(mark_a),
            );
            p.text(
                pos2(content.center().x, content.center().y + r * 2.6),
                Align2::CENTER_CENTER,
                self.settings.lang.t("لحظة…", "one moment…"),
                FontId::proportional(11.5),
                self.pal.faint.gamma_multiply(ease_out(mark_a)),
            );
        }

        let set_a = ease_out(self.settings_nav);
        if set_a > 0.004 {
            self.settings_screen(ctx, set_a);
        }
        let pick_a = ease_out(self.pick_in);
        if pick_a > 0.004 {
            self.picker(ctx, t, pick_a);
        }

        self.overlay(ctx, t);
        self.tour_overlay(ctx, t);
        self.dialogs(ctx, self.settings.lang.rtl());
    }
}

impl App {
    fn titlebar(&mut self, root: &mut egui::Ui) {
        let pal = self.pal;
        let lang = self.settings.lang;
        let rtl = lang.rtl();
        let mut minimize = false;
        let mut maximize = false;
        let mut close = false;

        egui::Panel::top("titlebar")
            .exact_size(32.0)
            .resizable(false)
            .frame(egui::Frame::NONE)
            .show(root, |ui| {
                let rect = ui.max_rect();
                ui.painter().rect_filled(rect, 0.0, pal.bg_deep);

                let drag = ui.interact(
                    rect,
                    egui::Id::new("titledrag"),
                    Sense::click_and_drag(),
                );
                if drag.drag_started() {
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                if drag.double_clicked() {
                    let max = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Maximized(!max));
                }

                let title = |ui: &mut egui::Ui| {
                    let l = if rtl {
                        Layout::right_to_left(Align::Center)
                    } else {
                        Layout::left_to_right(Align::Center)
                    };
                    ui.with_layout(l, |ui| {
                        ui.add_space(10.0);
                        let (r, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
                        ui::brand_mark(ui.painter(), r, &pal, 0.0);
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(lang.t(
                                "بديل — مبدّل حسابات الألعاب",
                                "badeel — game account switcher",
                            ))
                            .size(11.0)
                            .color(pal.muted),
                        );
                    });
                };
                let buttons = |ui: &mut egui::Ui| {
                    let l = if rtl {
                        Layout::left_to_right(Align::Center)
                    } else {
                        Layout::right_to_left(Align::Center)
                    };
                    ui.with_layout(l, |ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        if ui::titlebar_button(ui, &pal, 2).clicked() {
                            close = true;
                        }
                        if ui::titlebar_button(ui, &pal, 1).clicked() {
                            maximize = true;
                        }
                        if ui::titlebar_button(ui, &pal, 0).clicked() {
                            minimize = true;
                        }
                    });
                };

                ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                    let sides = egui::Sides::new().height(32.0);
                    if rtl {
                        sides.show(ui, buttons, title);
                    } else {
                        sides.show(ui, title, buttons);
                    }
                });
            });

        if minimize {
            root.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        }
        if maximize {
            let max = root
                .ctx()
                .input(|i| i.viewport().maximized.unwrap_or(false));
            root.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Maximized(!max));
        }
        if close {
            root.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn header(&mut self, root: &mut egui::Ui, t: f32) {
        let pal = self.pal;
        let lang = self.settings.lang;
        let rtl = lang.rtl();
        let locked = self.key.is_none();
        let total = self.accounts.accounts.len();
        let plat = self.platform.name(rtl);
        let live = self
            .current_id
            .as_ref()
            .and_then(|id| self.accounts.get(id))
            .map(|a| a.name.clone());
        let update_ready = self.update_ready;
        let found = match self.update_state.clone() {
            Some(Progress::Found(u)) => Some(u),
            _ => None,
        };
        let mut want_lock = false;
        let mut want_update = false;
        let mut want_restart = false;

        egui::Panel::top("header")
            .exact_size(60.0)
            .resizable(false)
            .frame(egui::Frame::NONE)
            .show(root, |ui| {
                let rect = ui.max_rect();
                ui.painter()
                    .rect_filled(rect, 0.0, pal.bg_deep.gamma_multiply(0.55));

                let brand = |ui: &mut egui::Ui| {
                    let l = if rtl {
                        Layout::right_to_left(Align::Center)
                    } else {
                        Layout::left_to_right(Align::Center)
                    };
                    ui.with_layout(l, |ui| {
                        let (r, _) = ui.allocate_exact_size(vec2(38.0, 38.0), Sense::hover());
                        ui::brand_mark(ui.painter(), r, &pal, t);
                        ui.add_space(9.0);
                        ui.label(
                            RichText::new(lang.t("بديل", "badeel"))
                                .size(19.0)
                                .strong(),
                        );
                    });
                };
                let chips = |ui: &mut egui::Ui| {
                    let l = if rtl {
                        Layout::left_to_right(Align::Center)
                    } else {
                        Layout::right_to_left(Align::Center)
                    };
                    ui.with_layout(l, |ui| {
                        if update_ready {
                            if ui::pill(
                                ui,
                                &pal,
                                lang.t("أعد التشغيل للتحديث", "Restart to update"),
                                pal.live,
                                t,
                            )
                            .clicked()
                            {
                                want_restart = true;
                            }
                        } else if let Some(u) = &found {
                            let txt = format!("{} {}", lang.t("تحديث", "Update"), u.version);
                            if ui::pill(ui, &pal, &txt, pal.warn, t).clicked() {
                                want_update = true;
                            }
                        }
                        let vtxt = format!(
                            "{} · v{}",
                            lang.t("النسخة العربية", "English build"),
                            update::current_version()
                        );
                        ui::status_chip(ui, &pal, &vtxt, pal.accent, t);

                        let vault_txt = if locked {
                            lang.t("الخزنة · مقفلة", "Vault · locked")
                        } else {
                            lang.t("الخزنة · مفتوحة ومشفّرة", "Vault · unlocked")
                        };
                        if ui::status_chip(
                            ui,
                            &pal,
                            vault_txt,
                            if locked { pal.warn } else { pal.live },
                            t,
                        )
                        .clicked()
                        {
                            want_lock = true;
                        }

                        let acc_txt = match &live {
                            Some(n) => format!("{plat} · {n}"),
                            None => format!(
                                "{plat} · {}",
                                if lang.rtl() {
                                    format!("{total} حساب")
                                } else {
                                    format!("{total} accounts")
                                }
                            ),
                        };
                        ui::status_chip(ui, &pal, &acc_txt, platform_color(self.platform.id), t);
                    });
                };

                ui.scope_builder(
                    egui::UiBuilder::new().max_rect(rect.shrink2(vec2(18.0, 11.0))),
                    |ui| {
                        let sides = egui::Sides::new().height(38.0);
                        if rtl {
                            sides.show(ui, chips, brand);
                        } else {
                            sides.show(ui, brand, chips);
                        }
                    },
                );
            });

        if want_lock {
            if self.key.is_some() {
                self.key = None;
            }
            self.dialog = Dialog::Unlock {
                password: String::new(),
                error: String::new(),
            };
        }
        if want_restart {
            let _ = update::restart();
            root.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if want_update {
            if let Some(u) = found {
                self.dialog = Dialog::UpdateFound(u);
            }
        }
    }

    fn footer(&mut self, root: &mut egui::Ui) {
        let pal = self.pal;
        let lang = self.settings.lang;
        let rtl = lang.rtl();

        egui::Panel::bottom("footer")
            .exact_size(32.0)
            .resizable(false)
            .frame(egui::Frame::NONE)
            .show(root, |ui| {
                let rect = ui.max_rect();
                ui.painter().rect_filled(rect, 0.0, pal.bg_deep);
                ui.painter().line_segment(
                    [pos2(rect.left(), rect.top()), pos2(rect.right(), rect.top())],
                    egui::Stroke::new(1.0, pal.line.gamma_multiply(0.7)),
                );

                let keys = |ui: &mut egui::Ui| {
                    let l = if rtl {
                        Layout::left_to_right(Align::Center)
                    } else {
                        Layout::right_to_left(Align::Center)
                    };
                    ui.with_layout(l, |ui| {
                        ui.spacing_mut().item_spacing.x = 12.0;
                        ui::key_chip(ui, &pal, "esc", lang.t("إغلاق", "close"));
                        ui::key_chip(ui, &pal, "Ctrl K", lang.t("بحث", "find"));
                        ui::key_chip(ui, &pal, "1-7", lang.t("المنصّة", "platform"));
                        ui::key_chip(ui, &pal, "S", lang.t("الإعدادات", "settings"));
                        ui::key_chip(ui, &pal, "L", lang.t("قفل الخزنة", "lock"));
                    });
                };
                let credit = |ui: &mut egui::Ui| {
                    let l = if rtl {
                        Layout::right_to_left(Align::Center)
                    } else {
                        Layout::left_to_right(Align::Center)
                    };
                    ui.with_layout(l, |ui| {
                        ui.label(
                            RichText::new(lang.t(
                                "بديل · تأسيس وتطوير: ريان الأثلاوي ومؤيد المطيري · GPL-3.0",
                                "badeel · founded by Ryan Athlawi & Moayad Almutairi · GPL-3.0",
                            ))
                            .size(10.0)
                            .color(pal.faint),
                        );
                    });
                };

                ui.scope_builder(
                    egui::UiBuilder::new().max_rect(rect.shrink2(vec2(14.0, 6.0))),
                    |ui| {
                        let sides = egui::Sides::new().height(20.0);
                        if rtl {
                            sides.show(ui, keys, credit);
                        } else {
                            sides.show(ui, credit, keys);
                        }
                    },
                );
            });
    }

    fn rail(&mut self, root: &mut egui::Ui, t: f32) {
        let pal = self.pal;
        let lang = self.settings.lang;
        let rtl = lang.rtl();
        let cur = self.platform.id;
        let on_settings = self.route == Route::Settings;
        let on_accounts = self.route == Route::Accounts;
        let on_about = self.route == Route::About;
        let mut want_home = false;
        let mut pick: Option<&'static Platform> = None;
        let mut want_settings = false;
        let mut want_about = false;
        let mut want_lang = false;

        let panel = if rtl {
            egui::Panel::left("rail")
        } else {
            egui::Panel::right("rail")
        };
        panel
            .exact_size(62.0)
            .resizable(false)
            .frame(egui::Frame::NONE)
            .show(root, |ui| {
                let rect = ui.max_rect();
                let edge = if rtl { rect.right() } else { rect.left() };
                ui.painter().line_segment(
                    [pos2(edge, rect.top()), pos2(edge, rect.bottom())],
                    egui::Stroke::new(1.0, pal.line.gamma_multiply(0.5)),
                );

                let rows = catalog::PLATFORMS.len() as f32 + 1.0;
                let room = rect.height() - 24.0 - 168.0;
                let side = ((room / rows) - 8.0).clamp(30.0, 46.0);
                let space = ((room - rows * side) / rows).clamp(3.0, 8.0);
                ui.scope_builder(
                    egui::UiBuilder::new().max_rect(rect.shrink2(vec2(8.0, 12.0))),
                    |ui| {
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = space;
                            if ui::rail_icon(ui, &pal, 1, self.route == Route::Home, side)
                                .clicked()
                            {
                                want_home = true;
                            }
                            for (i, p) in catalog::PLATFORMS.iter().enumerate() {
                                if ui::rail_button(
                                    ui,
                                    &pal,
                                    p.id,
                                    platform_color(p.id),
                                    p.id == cur && on_accounts,
                                    self.installed[i],
                                    self.counts[i],
                                    side,
                                    t,
                                )
                                .clicked()
                                {
                                    pick = Some(p);
                                }
                            }
                        });

                        let bottom = Rect::from_min_size(
                            pos2(rect.left() + 8.0, rect.bottom() - 168.0),
                            vec2(46.0, 158.0),
                        );
                        ui.scope_builder(egui::UiBuilder::new().max_rect(bottom), |ui| {
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 8.0;
                                if ui::rail_icon(ui, &pal, 2, on_about, 46.0).clicked() {
                                    want_about = true;
                                }
                                if ui::rail_icon(ui, &pal, 0, on_settings, 46.0).clicked() {
                                    want_settings = true;
                                }
                                if ui::icon_button(ui, &pal, lang.label(), 46.0).clicked() {
                                    want_lang = true;
                                }
                            });
                        });
                    },
                );
            });

        if want_home {
            self.go(Route::Home);
        }
        if let Some(p) = pick {
            self.go(Route::Accounts);
            if p.id != self.platform.id {
                self.open_platform(p);
            } else {
                self.list_in = 0.0;
            }
        }
        if want_about {
            self.go(if on_about { Route::Home } else { Route::About });
        }
        if want_settings {
            self.go(if on_settings {
                Route::Home
            } else {
                Route::Settings
            });
        }
        if want_lang {
            self.settings.lang = self.settings.lang.toggled();
        }
    }

    fn list_panel(&mut self, root: &mut egui::Ui, t: f32) {
        let pal = self.pal;
        let lang = self.settings.lang;
        let rtl = lang.rtl();
        let plat_id = self.platform.id;
        let plat_name = self.platform.name(rtl);
        let total = self.accounts.accounts.len();
        let mut want_add = false;
        let mut action: Option<(String, u8)> = None;
        let mut search = std::mem::take(&mut self.search);
        let faces: Vec<ui::Face> = if self.settings.board {
            board::pool(&self.board, plat_id)
                .into_iter()
                .map(|c| {
                    [
                        c.badge.t(rtl).to_string(),
                        c.title.t(rtl).to_string(),
                        c.body.t(rtl).to_string(),
                        c.cta.t(rtl).to_string(),
                        c.url.clone(),
                    ]
                })
                .collect()
        } else {
            Vec::new()
        };
        let (promo_i, promo_prev, promo_t) = (self.promo.i, self.promo.prev, self.promo.t);
        let mut promo_hot = false;
        let mut open_link: Option<String> = None;

        let panel = if rtl {
            egui::Panel::right("list")
        } else {
            egui::Panel::left("list")
        };
        panel
            .exact_size(340.0)
            .resizable(false)
            .frame(egui::Frame::NONE)
            .show(root, |ui| {
                let outer = ui.max_rect().shrink2(vec2(12.0, 12.0));
                ui::panel_card(ui.painter(), outer, &pal);

                ui.scope_builder(
                    egui::UiBuilder::new().max_rect(outer.shrink2(vec2(14.0, 14.0))),
                    |ui| {
                        let head = |ui: &mut egui::Ui| {
                            let l = if rtl {
                                Layout::right_to_left(Align::Center)
                            } else {
                                Layout::left_to_right(Align::Center)
                            };
                            ui.with_layout(l, |ui| {
                                ui.label(RichText::new(plat_name).size(16.0).strong());
                                ui.add_space(8.0);
                                let g = ui.painter().layout_no_wrap(
                                    format!("{total}"),
                                    FontId::proportional(10.5),
                                    pal.muted,
                                );
                                let (r, _) = ui.allocate_exact_size(
                                    vec2(g.size().x + 20.0, 22.0),
                                    Sense::hover(),
                                );
                                ui.painter().rect_filled(
                                    r,
                                    999.0,
                                    platform_color(plat_id).gamma_multiply(0.18),
                                );
                                ui.painter().text(
                                    r.center(),
                                    Align2::CENTER_CENTER,
                                    format!("{total}"),
                                    FontId::proportional(10.5),
                                    pal.text,
                                );
                            });
                        };
                        let head_right = |ui: &mut egui::Ui| {
                            let l = if rtl {
                                Layout::left_to_right(Align::Center)
                            } else {
                                Layout::right_to_left(Align::Center)
                            };
                            ui.with_layout(l, |ui| {
                                ui.label(
                                    RichText::new(lang.t("الحسابات", "Accounts"))
                                        .size(10.5)
                                        .color(pal.faint),
                                );
                            });
                        };
                        let sides = egui::Sides::new().height(26.0);
                        if rtl {
                            sides.show(ui, head_right, head);
                        } else {
                            sides.show(ui, head, head_right);
                        }

                        ui.add_space(10.0);
                        if total > 5 {
                            ui::search_box(
                                ui,
                                &pal,
                                rtl,
                                &mut search,
                                lang.t("ابحث عن حساب", "Search"),
                            );
                            ui.add_space(8.0);
                        }

                        let accounts: Vec<store::Account> = self
                            .accounts
                            .sorted()
                            .into_iter()
                            .filter(|a| {
                                let q = search.trim().to_lowercase();
                                q.is_empty()
                                    || a.name.to_lowercase().contains(&q)
                                    || a.note.to_lowercase().contains(&q)
                            })
                            .cloned()
                            .collect();

                        let btn_h = 46.0;
                        let avail = ui.available_rect_before_wrap();
                        // اللوحة تأخذ شريحتها من الأسفل، ولا تظهر إلا إن بقي
                        // للقائمة متّسع فلا تزحمها على الشاشات القصيرة
                        let card = !faces.is_empty() && avail.height() > 320.0;
                        let card_h = if card { 134.0 } else { 0.0 };
                        let list_rect = Rect::from_min_size(
                            avail.min,
                            vec2(
                                avail.width(),
                                (avail.height() - btn_h - 14.0 - card_h).max(60.0),
                            ),
                        );
                        ui.scope_builder(egui::UiBuilder::new().max_rect(list_rect), |ui| {
                            if accounts.is_empty() {
                                let r = ui.max_rect();
                                ui.painter().text(
                                    r.center(),
                                    Align2::CENTER_CENTER,
                                    lang.t("لا حسابات محفوظة بعد", "No saved accounts yet"),
                                    FontId::proportional(12.0),
                                    pal.faint,
                                );
                                return;
                            }
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    ui.spacing_mut().item_spacing.y = 8.0;
                                    for (i, acc) in accounts.iter().enumerate() {
                                        let appear =
                                            (self.list_in * 3.4 - i as f32 * 0.16).clamp(0.0, 1.0);
                                        let is_live =
                                            self.current_id.as_deref() == Some(&acc.id);
                                        let selected =
                                            self.selected.as_deref() == Some(&acc.id);
                                        if let Some(a) = ui::account_row(
                                            ui, &pal, rtl, acc, plat_id, is_live, selected,
                                            appear, t, lang,
                                        ) {
                                            action = Some((acc.id.clone(), a));
                                        }
                                    }
                                });
                        });

                        if card {
                            let n = faces.len();
                            let cur = &faces[promo_i.min(n - 1)];
                            let prev = (promo_prev != promo_i && promo_prev < n)
                                .then(|| &faces[promo_prev]);
                            // تدخل بعد أن تبدأ الحسابات بالظهور، صاعدة قليلًا كما تدخل هي
                            let appear = ease_out((self.list_in * 3.4 - 0.5).clamp(0.0, 1.0));
                            let r = Rect::from_min_size(
                                pos2(
                                    outer.left() + 14.0,
                                    outer.bottom() - 14.0 - btn_h - card_h
                                        + (1.0 - appear) * 10.0,
                                ),
                                vec2(outer.width() - 28.0, card_h - 10.0),
                            );
                            let (clicked, hot) = ui::promo_card(
                                ui,
                                r,
                                &pal,
                                rtl,
                                platform_color(plat_id),
                                cur,
                                prev,
                                promo_t,
                                appear,
                            );
                            promo_hot = hot;
                            if clicked {
                                open_link = Some(cur[4].clone());
                            }
                        }

                        let brect = Rect::from_min_size(
                            pos2(outer.left() + 14.0, outer.bottom() - 14.0 - btn_h),
                            vec2(outer.width() - 28.0, btn_h),
                        );
                        ui.scope_builder(egui::UiBuilder::new().max_rect(brect), |ui| {
                            ui.with_layout(
                                Layout::centered_and_justified(egui::Direction::LeftToRight),
                                |ui| {
                                    if ui::solid_button(
                                        ui,
                                        &pal,
                                        lang.t("أضف الحساب الحالي", "Add current account"),
                                        pal.accent_deep,
                                    )
                                    .clicked()
                                    {
                                        want_add = true;
                                    }
                                },
                            );
                        });
                    },
                );
            });

        self.search = search;
        self.promo_hot = promo_hot;
        if let Some(url) = open_link {
            let _ = std::process::Command::new("explorer.exe").arg(url).spawn();
        }
        if want_add {
            self.dialog = Dialog::AddAccount {
                name: String::new(),
                error: String::new(),
            };
        }
        if let Some((id, what)) = action {
            match what {
                0 => {
                    self.selected = Some(id);
                    self.go(Route::Accounts);
                }
                1 => self.ask_switch(id),
                _ => {
                    let name = self
                        .accounts
                        .get(&id)
                        .map(|a| a.name.clone())
                        .unwrap_or_default();
                    self.dialog = Dialog::Confirm { id, name };
                }
            }
        }
    }

    fn showcase_panel(&mut self, root: &mut egui::Ui, t: f32) {
        if !self.settings.showcase {
            return;
        }
        let avail = root.available_rect_before_wrap();
        if avail.width() < 800.0 {
            return;
        }
        let pal = self.pal;
        let lang = self.settings.lang;
        let alpha = ease_out(self.intro);
        let mo = self.motion;
        let show = &self.show;
        let mut jump = None;

        let panel = if lang.rtl() {
            egui::Panel::left("showcase")
        } else {
            egui::Panel::right("showcase")
        };
        panel
            .exact_size(308.0)
            .resizable(false)
            .frame(egui::Frame::NONE)
            .show(root, |ui| {
                let card = ui.max_rect().shrink2(vec2(12.0, 12.0));
                jump = showcase::panel(ui, card, &pal, lang, show, t, alpha);
            });

        if let Some(i) = jump {
            self.show.go(i, mo);
        }
    }

    fn use_profile(&mut self, id: &str, ctx: &egui::Context) {
        if id != self.book.active {
            if let Err(e) = profile::activate(&mut self.book, id) {
                self.toast(format!("{e:#}"), ToastKind::Err);
                return;
            }
            self.restyle(ctx);
            self.counts = catalog::PLATFORMS
                .iter()
                .map(|p| accounts_of(p).accounts.len())
                .collect();
            self.installed = catalog::PLATFORMS.iter().map(procs::installed).collect();
            self.selected = None;
            self.search.clear();
            self.reload();
        }
        self.picking = false;
        self.route = Route::Home;
        self.home_in = 0.0;
        self.list_in = 0.0;
        self.intro = 0.0;
        self.curtain = 1.0;
    }

    fn picker(&mut self, ctx: &egui::Context, t: f32, alpha: f32) {
        let pal = self.pal;
        let lang = self.settings.lang;
        let rtl = lang.rtl();
        let people: Vec<(String, String, u8, usize)> = self
            .book
            .profiles
            .iter()
            .map(|p| {
                let name = if p.name.trim().is_empty() {
                    lang.t("الملف الرئيسي", "Main profile").to_string()
                } else {
                    p.name.clone()
                };
                let saved = paths::root_of(&p.id)
                    .join("accounts")
                    .read_dir()
                    .map(|d| d.flatten().count())
                    .unwrap_or(0);
                (p.id.clone(), name, p.accent, saved)
            })
            .collect();
        let mut chosen: Option<String> = None;
        let mut want_new = false;

        egui::Area::new(egui::Id::new("picker"))
            .order(egui::Order::Middle)
            .fixed_pos(pos2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.set_opacity(alpha);
                if alpha < 0.90 {
                    ui.disable();
                }
                let screen = ctx.viewport_rect();
                let area = Rect::from_min_max(pos2(screen.left(), screen.top() + 32.0), screen.max);
                ui.interact(area, egui::Id::new("pickerscrim"), Sense::click());
                theme::draw_backdrop(ui.painter(), area, &pal, t, self.motion.backdrop);
                ui.painter()
                    .rect_filled(area, 0.0, pal.bg_deep.gamma_multiply(0.42));

                let n = people.len() + 1;
                let cols = (n.min(5)).max(1);
                let rows = n.div_ceil(cols);
                let tw = ((area.width() - 80.0) / cols as f32).clamp(120.0, 190.0);
                let th = (tw + 62.0).min((area.height() - 190.0) / rows as f32 + 62.0);
                let gap = 20.0;
                let grid_w = tw * cols as f32 + gap * (cols - 1) as f32;
                let grid_h = th * rows as f32 + gap * (rows - 1) as f32;
                let top = area.center().y - grid_h * 0.5 + 24.0;

                let p = ui.painter();
                let mark = Rect::from_center_size(
                    pos2(area.center().x, top - 116.0),
                    vec2(40.0, 40.0),
                );
                ui::brand_mark(p, mark, &pal, t);
                p.text(
                    pos2(area.center().x, top - 72.0),
                    Align2::CENTER_CENTER,
                    lang.t("من يستخدم بديل؟", "Who is using badeel?"),
                    FontId::proportional(24.0),
                    pal.text,
                );
                p.text(
                    pos2(area.center().x, top - 44.0),
                    Align2::CENTER_CENTER,
                    lang.t(
                        "لكل ملف حساباته ولونه وإعداداته الخاصة",
                        "Each profile keeps its own accounts, colour and settings",
                    ),
                    FontId::proportional(12.0),
                    pal.muted,
                );

                for (i, (id, name, acc, saved)) in people.iter().enumerate() {
                    let (gr, gc) = (i / cols, i % cols);
                    let x = if rtl {
                        area.center().x + grid_w * 0.5 - tw - gc as f32 * (tw + gap)
                    } else {
                        area.center().x - grid_w * 0.5 + gc as f32 * (tw + gap)
                    };
                    let cell = Rect::from_min_size(
                        pos2(x, top + gr as f32 * (th + gap)),
                        vec2(tw, th),
                    );
                    let sub = if rtl {
                        format!("{saved} منصّة")
                    } else {
                        format!("{saved} platforms")
                    };
                    if ui::profile_tile(
                        ui,
                        &pal,
                        cell,
                        id,
                        name,
                        &sub,
                        theme::palette(Accent::from_index(*acc)).accent,
                        false,
                        t,
                        1.0,
                    )
                    .clicked()
                    {
                        chosen = Some(id.clone());
                    }
                }

                let i = people.len();
                let (gr, gc) = (i / cols, i % cols);
                let x = if rtl {
                    area.center().x + grid_w * 0.5 - tw - gc as f32 * (tw + gap)
                } else {
                    area.center().x - grid_w * 0.5 + gc as f32 * (tw + gap)
                };
                let cell = Rect::from_min_size(
                    pos2(x, top + gr as f32 * (th + gap)),
                    vec2(tw, th),
                );
                if ui::add_tile(
                    ui,
                    &pal,
                    cell,
                    lang.t("ملف جديد", "New profile"),
                    1.0,
                )
                .clicked()
                {
                    want_new = true;
                }

                ui.painter().text(
                    pos2(area.center().x, area.bottom() - 26.0),
                    Align2::CENTER_CENTER,
                    lang.t(
                        "تأسيس وتطوير: ريان الأثلاوي ومؤيد المطيري",
                        "Founded and built by Ryan Athlawi and Moayad Almutairi",
                    ),
                    FontId::proportional(10.5),
                    pal.faint,
                );
            });

        if let Some(id) = chosen {
            self.use_profile(&id, ctx);
        }
        if want_new {
            self.dialog = Dialog::NewProfile {
                name: String::new(),
                error: String::new(),
            };
        }
    }

    fn about_screen(&mut self, root: &mut egui::Ui, area: Rect, t: f32, alpha: f32) {
        if alpha <= 0.004 || area.width() < 80.0 {
            return;
        }
        let pal = self.pal;
        let lang = self.settings.lang;
        let version = update::current_version().to_string();
        let hit = about::page(
            root,
            area.shrink2(vec2(10.0, 6.0)),
            &pal,
            lang,
            &version,
            t,
            self.about_in,
            alpha,
        );
        if let Some((who, what)) = hit {
            let person = &about::PEOPLE[who];
            let link = if what == 0 {
                person.support
            } else {
                person.contact
            };
            if link.is_empty() {
                self.toast(
                    lang.t(
                        "الرابط ما ضُبط بعد — قريبًا",
                        "Link not set yet - coming soon",
                    ),
                    ToastKind::Ok,
                );
            } else {
                let _ = std::process::Command::new("explorer.exe").arg(link).spawn();
            }
        }
    }

    fn home_screen(&mut self, root: &mut egui::Ui, area: Rect, t: f32, alpha: f32) {
        let pal = self.pal;
        let lang = self.settings.lang;
        let rtl = lang.rtl();
        if alpha <= 0.004 || area.width() < 80.0 || area.height() < 120.0 {
            return;
        }
        let counts = self.counts.clone();
        let installed = self.installed.clone();
        let saved: usize = counts.iter().sum();
        let who = self.book.current().name.clone();
        let safe = self.key.is_some();
        let mo = self.motion;
        let mut pick: Option<&'static Platform> = None;
        let mut jump = None;

        let e = ease_out(self.home_in);
        let st = |i: f32| ease_out(((self.home_in * 2.1) - i * 0.12).clamp(0.0, 1.0));

        let inner = area.shrink2(vec2(20.0, 8.0));
        let h = inner.height();
        let cw = inner.width().min(1020.0);
        let cx = inner.center().x;
        let left = cx - cw * 0.5;
        let gap = 12.0;

        let credits_h = 22.0;
        let hero_h = (h * 0.40).clamp(164.0, 252.0);
        let lab_h = 20.0;
        let plat_h = (h * 0.22).clamp(92.0, 136.0);
        let n = catalog::PLATFORMS.len();
        let one_row = (cw - gap * (n - 1) as f32) / n as f32 >= 124.0;
        let cols = if one_row { n } else { n.div_ceil(2) };
        let grid_rows = n.div_ceil(cols);
        let tw = (cw - gap * (cols - 1) as f32) / cols as f32;
        let th = if grid_rows > 1 {
            (plat_h - gap) * 0.5 + 18.0
        } else {
            plat_h
        };
        let plat_band = th * grid_rows as f32 + gap * (grid_rows - 1) as f32;

        let mut used = hero_h + gap + lab_h + 6.0 + plat_band + credits_h + gap;
        let strip_h = 92.0;
        let with_strip = h - used >= strip_h + gap + 4.0;
        if with_strip {
            used += strip_h + gap;
        }
        let mut y = inner.top() + ((h - used) * 0.34).max(0.0) + (1.0 - e) * 16.0;

        let hero = Rect::from_min_size(pos2(left, y), vec2(cw, hero_h));
        y += hero_h + gap;

        let a0 = st(0.0);
        ui::glass_card(root.painter(), hero, &pal, pal.accent, a0 * alpha);

        let pad = 22.0;
        let em_r = (hero_h * 0.22).clamp(30.0, 46.0);
        let wide = cw >= 660.0;
        let lead = if rtl {
            hero.right() - pad
        } else {
            hero.left() + pad
        };
        let em_c = if wide {
            pos2(
                if rtl {
                    lead - em_r * 1.68
                } else {
                    lead + em_r * 1.68
                },
                hero.center().y,
            )
        } else {
            pos2(hero.center().x, hero.top() + pad + em_r * 1.25)
        };
        ui::emblem(root.painter(), em_c, em_r, &pal, t, safe, st(1.0) * alpha);

        let stats_w = if wide && cw >= 860.0 { 150.0 } else { 0.0 };
        let text_w = if wide {
            hero.width() - pad * 2.0 - em_r * 3.6 - 20.0 - stats_w
        } else {
            hero.width() - pad * 2.0
        };
        let tx = if wide {
            if rtl {
                lead - em_r * 3.36 - 20.0
            } else {
                lead + em_r * 3.36 + 20.0
            }
        } else {
            hero.center().x
        };
        let anchor = if !wide {
            Align2::CENTER_CENTER
        } else if rtl {
            Align2::RIGHT_CENTER
        } else {
            Align2::LEFT_CENTER
        };

        let chips_h = 28.0;
        let block_h = if wide { 100.0 } else { 96.0 };
        let mut ty = if wide {
            hero.center().y - block_h * 0.5 + 12.0
        } else {
            em_c.y + em_r * 1.30 + 16.0
        };

        {
            let p = root.painter();
            let a1 = st(2.0);
            let hour = crate::core::clock::local_hour();
            let hello = if who.trim().is_empty() {
                crate::core::clock::greeting(hour, rtl).to_string()
            } else {
                format!("{}، {who}", crate::core::clock::greeting(hour, rtl))
            };
            p.text(
                pos2(tx, ty + (1.0 - a1) * 8.0),
                anchor,
                &hello,
                FontId::proportional(13.5),
                pal.accent_2.gamma_multiply(a1 * alpha),
            );
            ty += 26.0;

            let a2 = st(3.0);
            let head = lang.t("أهلًا بك في بديل", "Welcome to badeel");
            let size = if wide { 26.0 } else { 23.0 };
            if wide {
                let g = p.layout_no_wrap(
                    head.to_owned(),
                    FontId::proportional(size),
                    pal.text.gamma_multiply(a2 * alpha),
                );
                let gx = if rtl { tx - g.size().x } else { tx };
                ui::shimmer_text(
                    p,
                    pos2(gx + g.size().x * 0.5, ty + (1.0 - a2) * 8.0),
                    head,
                    size,
                    pal.text.gamma_multiply(a2 * alpha),
                    pal.accent_2,
                    g.size().x,
                    t,
                    a2 * alpha,
                );
            } else {
                ui::shimmer_text(
                    p,
                    pos2(tx, ty + (1.0 - a2) * 8.0),
                    head,
                    size,
                    pal.text.gamma_multiply(a2 * alpha),
                    pal.accent_2,
                    text_w.min(420.0),
                    t,
                    a2 * alpha,
                );
            }
            ty += 26.0;

            let a3 = st(4.0);
            let tag = if rtl {
                format!(
                    "حساباتك كلّها في نافذة واحدة · {saved} محفوظ على هذا الجهاز"
                )
            } else {
                format!("Every account in one window · {saved} stored on this PC")
            };
            p.text(
                pos2(tx, ty + (1.0 - a3) * 8.0),
                anchor,
                tag,
                FontId::proportional(12.0),
                pal.muted.gamma_multiply(a3 * alpha),
            );
            ty += 24.0;

            let a4 = st(5.0);
            let vault_txt = if safe {
                lang.t("الخزنة مفتوحة", "Vault open")
            } else {
                lang.t("الخزنة مقفلة", "Vault locked")
            };
            let chips: [(u8, &str, Color32); 4] = [
                (
                    if safe { 0 } else { 1 },
                    vault_txt,
                    if safe { pal.live } else { pal.warn },
                ),
                (1, "AES-256-GCM", pal.live),
                (
                    2,
                    lang.t("هذا الجهاز فقط", "This PC only"),
                    pal.live,
                ),
                (
                    3,
                    lang.t("بلا تتبّع", "No tracking"),
                    pal.accent_2,
                ),
            ];
            let widths: Vec<f32> = chips
                .iter()
                .map(|(_, txt, _)| ui::chip_width(p, &pal, txt))
                .collect();
            let mut keep = chips.len();
            while keep > 1 {
                let total: f32 =
                    widths[..keep].iter().sum::<f32>() + 8.0 * (keep - 1) as f32;
                if total <= text_w {
                    break;
                }
                keep -= 1;
            }
            let total: f32 = widths[..keep].iter().sum::<f32>() + 8.0 * (keep - 1) as f32;
            let mut chx = if !wide {
                tx - total * 0.5
            } else if rtl {
                tx - total
            } else {
                tx
            };
            for i in 0..keep {
                let ai = st(5.0 + i as f32 * 0.55);
                let (kind, txt, col) = chips[i];
                let r = Rect::from_min_size(
                    pos2(chx, ty - chips_h * 0.5 + (1.0 - ai) * 8.0),
                    vec2(widths[i], chips_h),
                );
                ui::assure_chip(p, r, &pal, rtl, kind, txt, col, ai * a4 * alpha);
                chx += widths[i] + 8.0;
            }
        }

        if stats_w > 0.0 {
            let a5 = st(7.0);
            let p = root.painter();
            let sx = if rtl {
                hero.left() + pad
            } else {
                hero.right() - pad
            };
            let live = catalog::PLATFORMS
                .iter()
                .enumerate()
                .filter(|(i, _)| installed[*i])
                .count();
            let rows: [(String, &str, Color32); 3] = [
                (
                    saved.to_string(),
                    lang.t("حساب محفوظ", "accounts stored"),
                    pal.accent,
                ),
                (
                    format!("{live}/{}", catalog::PLATFORMS.len()),
                    lang.t("منصّة مثبّتة", "platforms found"),
                    pal.accent_2,
                ),
                (
                    if who.trim().is_empty() {
                        lang.t("الرئيسي", "main").to_string()
                    } else {
                        who.clone()
                    },
                    lang.t("الملف النشط", "active profile"),
                    pal.live,
                ),
            ];
            let block_top = hero.center().y - 62.0;
            for (i, (value, label, col)) in rows.into_iter().enumerate() {
                let ai = st(7.0 + i as f32 * 0.5);
                let r = Rect::from_min_size(
                    pos2(
                        if rtl { sx } else { sx - stats_w },
                        block_top + i as f32 * 44.0 + (1.0 - ai) * 8.0,
                    ),
                    vec2(stats_w, 40.0),
                );
                ui::stat_line(p, r, &pal, rtl, &value, label, col, ai * a5 * alpha);
            }
        }

        let lab = Rect::from_min_size(pos2(left, y), vec2(cw, lab_h));
        y += lab_h + 6.0;
        ui::band_label(
            root.painter(),
            lab,
            &pal,
            rtl,
            lang.t("منصّاتك", "YOUR PLATFORMS"),
        );

        for (i, plat) in catalog::PLATFORMS.iter().enumerate() {
            let ai = st(8.0 + i as f32 * 0.4);
            let (gr, gc) = (i / cols, i % cols);
            let x = if rtl {
                left + cw - tw - gc as f32 * (tw + gap)
            } else {
                left + gc as f32 * (tw + gap)
            };
            let cell = Rect::from_min_size(
                pos2(x, y + gr as f32 * (th + gap) + (1.0 - ai) * 16.0),
                vec2(tw, th),
            );
            let sub = if !installed[i] {
                lang.t("غير مثبّت", "not installed").to_string()
            } else if rtl {
                format!("{} حساب", counts[i])
            } else {
                format!("{} accounts", counts[i])
            };
            if ui::home_platform(
                root,
                &pal,
                cell,
                plat.id,
                platform_color(plat.id),
                plat.name(rtl),
                &sub,
                installed[i],
                ai * alpha,
            )
            .clicked()
            {
                pick = Some(plat);
            }
        }
        y += plat_band + gap;

        if with_strip {
            let ai = st(12.0);
            let cell = Rect::from_min_size(pos2(left, y + (1.0 - ai) * 12.0), vec2(cw, strip_h));
            jump = showcase::strip(root, cell, &pal, rtl, &self.show, t, ai * alpha);
            y += strip_h + gap;
        }

        let credits = Rect::from_min_size(pos2(left, y), vec2(cw, credits_h));
        root.painter().text(
            credits.center(),
            Align2::CENTER_CENTER,
            lang.t(
                "تأسيس وتطوير: ريان الأثلاوي ومؤيد المطيري · مفتوح المصدر GPL-3.0",
                "Founded and built by Ryan Athlawi and Moayad Almutairi · open source, GPL-3.0",
            ),
            FontId::proportional(10.5),
            pal.faint.gamma_multiply(st(13.0) * alpha),
        );

        if let Some(i) = jump {
            self.show.go(i, mo);
        }
        if let Some(plat) = pick {
            self.go(Route::Accounts);
            if plat.id != self.platform.id {
                self.open_platform(plat);
            } else {
                self.list_in = 0.0;
            }
        }
    }

    fn stage(&mut self, root: &mut egui::Ui, area: Rect, t: f32, alpha: f32) {
        let pal = self.pal;
        let lang = self.settings.lang;
        let rtl = lang.rtl();
        if alpha <= 0.004 || area.width() < 60.0 {
            return;
        }
        let plat_id = self.platform.id;
        let color = platform_color(plat_id);

        let shown = self
            .selected
            .as_ref()
            .and_then(|id| self.accounts.get(id))
            .or_else(|| {
                self.current_id
                    .as_ref()
                    .and_then(|id| self.accounts.get(id))
            })
            .cloned();

        let mut want_switch = false;
        let mut want_launch = false;
        let mut want_avatar = false;
        let mut want_rename = false;
        let mut want_forget = false;

        let inner = area.shrink2(vec2(18.0, 14.0));
        {
            let p = root.painter();
            for k in (1..=7).rev() {
                let f = k as f32 / 7.0;
                p.circle_filled(
                    pos2(inner.center().x, inner.center().y - inner.height() * 0.08),
                    inner.width().max(inner.height()) * 0.42 * f,
                    color.gamma_multiply(0.020 * (1.0 - f) * alpha),
                );
            }
        }

        let Some(acc) = shown else {
            let none_saved = self.accounts.accounts.is_empty();
            let card = Rect::from_center_size(
                inner.center(),
                vec2(inner.width().min(480.0), 214.0),
            );
            ui::glass_card(root.painter(), card, &pal, color, alpha);
            let icon = Rect::from_center_size(
                pos2(inner.center().x, inner.center().y - 66.0),
                vec2(56.0, 56.0),
            );
            ui::platform_icon(root, icon, plat_id, color, alpha * 0.85);
            // باتل نت الفارغ يقول السبب، فالصفحة الساكتة كانت تترك المستخدم لا يدري
            let why = self.bnet_why.clone().filter(|_| none_saved).map(|w| match w {
                bnet::Empty::NoConfig => (
                    lang.t("باتل نت ما انفتح عندك للحين", "Battle.net hasn't been opened on this PC yet"),
                    lang.t(
                        "افتح باتل نت وسجّل دخولك مرة، وبعدها ارجع هنا",
                        "Open Battle.net, sign in once, then come back",
                    ),
                    false,
                ),
                bnet::Empty::Unreadable(_) => (
                    lang.t("ملف باتل نت خربان وما قدرت أقراه", "Battle.net's settings file is damaged"),
                    lang.t(
                        "افتح باتل نت وسجّل دخولك مرة عشان يصلّحه، وبعدها ارجع هنا",
                        "Open Battle.net and sign in once so it repairs it, then come back",
                    ),
                    false,
                ),
                bnet::Empty::NotRemembered => (
                    lang.t("باتل نت ما يحفظ إيميلاتك", "Battle.net isn't saving your emails"),
                    lang.t(
                        "اضغط الزر، وبعدها افتح باتل نت وسجّل دخولك مرة",
                        "Press the button, then open Battle.net and sign in once",
                    ),
                    true,
                ),
                bnet::Empty::NothingSaved => (
                    lang.t("باتل نت ما حفظ ولا حساب للحين", "Battle.net hasn't saved an account yet"),
                    lang.t(
                        "افتح باتل نت وسجّل دخولك مرة، وبيطلع حسابك هنا",
                        "Open Battle.net, sign in once, and your account shows up here",
                    ),
                    false,
                ),
            });
            let p = root.painter();
            p.text(
                pos2(inner.center().x, inner.center().y - 6.0),
                Align2::CENTER_CENTER,
                if let Some((title, _, _)) = &why {
                    title
                } else if none_saved {
                    lang.t(
                        "ما فيه حسابات محفوظة هنا بعد",
                        "No accounts saved here yet",
                    )
                } else {
                    lang.t(
                        "اختر حسابًا من القائمة",
                        "Pick an account from the list",
                    )
                },
                FontId::proportional(16.5),
                pal.text.gamma_multiply(alpha * 0.85),
            );
            p.text(
                pos2(inner.center().x, inner.center().y + 22.0),
                Align2::CENTER_CENTER,
                if let Some((_, hint, _)) = &why {
                    hint
                } else if none_saved && self.platform.identity.own_list() {
                    lang.t(
                        "سجّل دخولك في المنصّة كالمعتاد وبديل يتعرّف على حساباتك وحده",
                        "Sign in on the platform as usual and badeel finds your accounts by itself",
                    )
                } else if none_saved {
                    lang.t(
                        "سجّل دخولك في المنصّة كالمعتاد، ثم اضغط «أضف الحساب الحالي»",
                        "Sign in on the platform as usual, then press Add current account",
                    )
                } else {
                    lang.t(
                        "اضغط أي حساب لتشوف تفاصيله",
                        "Click any account to see its details",
                    )
                },
                FontId::proportional(12.0),
                pal.muted.gamma_multiply(alpha * 0.9),
            );
            if why.as_ref().is_some_and(|(_, _, fixable)| *fixable) {
                let btn = Rect::from_center_size(
                    pos2(inner.center().x, inner.center().y + 64.0),
                    vec2(190.0, 38.0),
                );
                let clicked = root
                    .scope_builder(egui::UiBuilder::new().max_rect(btn), |ui| {
                        ui.with_layout(
                            Layout::centered_and_justified(egui::Direction::LeftToRight),
                            |ui| {
                                ui::solid_button(
                                    ui,
                                    &pal,
                                    lang.t("فعّل حفظ الإيميلات", "Save my emails"),
                                    pal.accent_deep,
                                )
                                .clicked()
                            },
                        )
                        .inner
                    })
                    .inner;
                if clicked {
                    self.remember_bnet();
                }
            }
            return;
        };

        let is_live = self.current_id.as_deref() == Some(acc.id.as_str());
        let e = ease_out(self.list_in);
        let av_r = (inner.height() * 0.13).clamp(38.0, 64.0);
        let tail = if is_live { 274.0 } else { 256.0 };
        let block = 2.0 * av_r + tail;
        let pad = ((inner.height() - block) * 0.40).clamp(10.0, 60.0);
        let cy = inner.top() + pad + av_r + (1.0 - e) * 14.0;
        let cx = inner.center().x;

        let card = Rect::from_min_max(
            pos2(cx - inner.width().min(608.0) * 0.5, cy - av_r - 28.0),
            pos2(
                cx + inner.width().min(608.0) * 0.5,
                (cy + av_r + tail + 12.0).min(inner.bottom()),
            ),
        );
        ui::glass_card(root.painter(), card, &pal, color, alpha);

        ui::avatar(
            root,
            pos2(cx, cy),
            av_r,
            &acc,
            plat_id,
            &pal,
            is_live,
            t,
            alpha,
        );

        let base = cy + av_r;
        let p = root.painter();
        p.text(
            pos2(cx, base + 30.0),
            Align2::CENTER_CENTER,
            &acc.name,
            FontId::proportional(25.0),
            pal.text.gamma_multiply(alpha),
        );
        let hide = self.settings.mask_ids;
        let sub = if acc.note.is_empty() {
            self.platform.name(rtl).to_string()
        } else {
            format!("{} · {}", self.platform.name(rtl), mask(&acc.note, hide))
        };
        p.text(
            pos2(cx, base + 54.0),
            Align2::CENTER_CENTER,
            sub,
            FontId::proportional(12.0),
            pal.muted.gamma_multiply(alpha),
        );

        if is_live {
            let label = lang.t("مسجّل دخوله الآن", "signed in right now");
            let g = p.layout_no_wrap(label.to_string(), FontId::proportional(10.5), pal.live);
            let r = Rect::from_center_size(pos2(cx, base + 82.0), vec2(g.size().x + 34.0, 23.0));
            p.rect_filled(r, 999.0, pal.live.gamma_multiply(0.16 * alpha));
            let dot_x = if rtl { r.right() - 13.0 } else { r.left() + 13.0 };
            p.circle_filled(
                pos2(dot_x, r.center().y),
                3.0,
                pal.live
                    .gamma_multiply(alpha * (0.5 + 0.5 * theme::pulse(t, 2.4))),
            );
            p.text(
                pos2(r.center().x + if rtl { -7.0 } else { 7.0 }, r.center().y),
                Align2::CENTER_CENTER,
                label,
                FontId::proportional(10.5),
                pal.live.gamma_multiply(alpha),
            );
        }

        let primary = if is_live {
            lang.t("شغّل المنصّة", "Launch platform")
        } else {
            lang.t("بدّل إلى هذا الحساب", "Switch to this account")
        };
        let second = if is_live {
            lang.t("أعد تركيب الجلسة", "Re-apply session")
        } else {
            lang.t("شغّل بلا تبديل", "Launch as is")
        };
        let wide = |s: &str, pad: f32| {
            p.layout_no_wrap(s.to_string(), FontId::proportional(13.0), pal.text)
                .size()
                .x
                + pad
        };
        let (w1, w2) = (wide(primary, 40.0), wide(second, 32.0));
        let prow = Rect::from_center_size(
            pos2(cx, base + if is_live { 126.0 } else { 108.0 }),
            vec2(w1 + w2 + 10.0, 44.0),
        );
        root.scope_builder(egui::UiBuilder::new().max_rect(prow), |ui| {
            let l = if rtl {
                Layout::right_to_left(Align::Center)
            } else {
                Layout::left_to_right(Align::Center)
            };
            ui.with_layout(l, |ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                if ui::solid_button(ui, &pal, primary, pal.accent_deep).clicked() {
                    if is_live {
                        want_launch = true;
                    } else {
                        want_switch = true;
                    }
                }
                if ui::ghost_button_sized(ui, &pal, second, vec2(w2, 44.0)).clicked() {
                    if is_live {
                        want_switch = true;
                    } else {
                        want_launch = true;
                    }
                }
            });
        });

        let chip = vec2(106.0, 36.0);
        let srow = Rect::from_center_size(
            pos2(cx, prow.bottom() + 28.0),
            vec2(chip.x * 3.0 + 20.0, chip.y),
        );
        root.scope_builder(egui::UiBuilder::new().max_rect(srow), |ui| {
            let l = if rtl {
                Layout::right_to_left(Align::Center)
            } else {
                Layout::left_to_right(Align::Center)
            };
            ui.with_layout(l, |ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                if ui::ghost_button_sized(ui, &pal, lang.t("الصورة", "Picture"), chip).clicked() {
                    want_avatar = true;
                }
                if ui::ghost_button_sized(ui, &pal, lang.t("التسمية", "Rename"), chip).clicked() {
                    want_rename = true;
                }
                if ui::ghost_button_sized(ui, &pal, lang.t("حذف", "Forget"), chip).clicked() {
                    want_forget = true;
                }
            });
        });

        let strip_top = srow.bottom() + 22.0;
        if inner.bottom() - strip_top >= 62.0 {
            let full = inner.width().min(530.0);
            let tw = (full - 30.0) / 4.0;
            let cells: [(&str, String, Option<Color32>); 4] = [
                (
                    lang.t("آخر استخدام", "Last used"),
                    ago(lang, acc.last_used),
                    None,
                ),
                (
                    lang.t("مرات التبديل", "Switches"),
                    acc.uses.to_string(),
                    None,
                ),
                (
                    lang.t("المعرّف", "Identifier"),
                    mask(&acc.id, hide),
                    None,
                ),
                (
                    lang.t("الحماية", "Protection"),
                    lang.t("مشفّر · AES-256", "AES-256 sealed").to_string(),
                    Some(pal.live),
                ),
            ];
            let p = root.painter();
            for (i, (label, value, col)) in cells.into_iter().enumerate() {
                let r = Rect::from_min_size(
                    pos2(cx - full * 0.5 + (tw + 10.0) * i as f32, strip_top),
                    vec2(tw, 58.0),
                );
                let slide = (1.0 - ease_out((self.list_in * 2.2 - i as f32 * 0.12).clamp(0.0, 1.0)))
                    * 10.0;
                ui::stat_tile(
                    p,
                    r.translate(vec2(0.0, slide)),
                    &pal,
                    label,
                    &value,
                    col,
                    alpha,
                );
            }
        }

        let id = acc.id.clone();
        if want_switch {
            self.ask_switch(id.clone());
        }
        if want_launch {
            if let Err(e) = procs::launch(self.platform, &[]) {
                self.toast(format!("{e:#}"), ToastKind::Err);
            }
        }
        if want_avatar {
            self.pick_avatar(&id);
        }
        if want_rename {
            self.dialog = Dialog::Rename {
                id: id.clone(),
                name: acc.name.clone(),
            };
        }
        if want_forget {
            self.dialog = Dialog::Confirm {
                id,
                name: acc.name.clone(),
            };
        }
    }

    fn settings_screen(&mut self, ctx: &egui::Context, alpha: f32) {
        let pal = self.pal;
        let lang = self.settings.lang;
        let rtl = lang.rtl();
        let t = self.t0.elapsed().as_secs_f32();
        let mut want_close = false;
        let mut tab = self.settings_tab;

        let mut toggle_launch = false;
        let mut toggle_close = false;
        let mut toggle_chooser = false;
        let mut toggle_confirm = false;
        let mut toggle_anim = false;
        let mut toggle_backdrop = false;
        let mut toggle_showcase = false;
        let mut toggle_mask = false;
        let mut toggle_discord = false;
        let mut toggle_board = false;
        let mut toggle_minlock = false;
        let mut toggle_autoupd = false;
        let mut want_lang = false;
        let mut want_password = false;
        let mut want_remove_password = false;
        let mut want_folder = false;
        let mut want_export = false;
        let mut want_import = false;
        let mut want_tour = false;
        let mut want_check = false;
        let mut want_about = false;
        let mut cycle_lock = false;
        let mut new_profile = false;
        let mut rename_profile = false;
        let mut drop_profile = false;
        let mut want_picker = false;
        let mut pick_profile: Option<String> = None;
        let mut pick_accent: Option<u8> = None;

        let has_password = self.has_pw;
        let profiles: Vec<(String, String, u8)> = self
            .book
            .profiles
            .iter()
            .map(|p| (p.id.clone(), p.name.clone(), p.accent))
            .collect();
        let active_id = self.book.active.clone();
        let active_name = self.book.current().name.clone();
        let accent_now = self.accent.index();
        let many = profiles.len() > 1;
        let lock_label = match self.settings.auto_lock_min {
            0 => lang.t("مطفأ", "off").to_string(),
            n if rtl => format!("{n} د"),
            n => format!("{n} min"),
        };
        let swatch: Vec<Color32> = Accent::ALL
            .iter()
            .map(|a| theme::palette(*a).accent)
            .collect();
        let sections: [(u8, &str, &str); 6] = [
            (
                0,
                lang.t("الملفات الشخصية", "Profiles"),
                lang.t(
                    "لكل شخص على هذا الجهاز حساباته ولونه ووضعه الخاص",
                    "Each person on this PC gets their own accounts, colour and mode",
                ),
            ),
            (
                1,
                lang.t("المظهر والحركة", "Look and motion"),
                lang.t(
                    "الوضع واللون واللغة وسرعة الحركة",
                    "Mode, colour, language and how much the interface moves",
                ),
            ),
            (
                2,
                lang.t("التبديل", "Switching"),
                lang.t(
                    "ما الذي يحدث بالضبط عند كل تبديل حساب",
                    "Exactly what happens on every account switch",
                ),
            ),
            (
                3,
                lang.t("الأمان", "Security"),
                lang.t(
                    "التشفير والقفل وما يظهر على الشاشة",
                    "Encryption, locking, and what shows on screen",
                ),
            ),
            (
                4,
                lang.t("التحديثات", "Updates"),
                lang.t(
                    "كل ما يتصل بالإنترنت، وما يكلّم إلا GitHub",
                    "Everything that goes online, and it only talks to GitHub",
                ),
            ),
            (
                5,
                lang.t("عن البرنامج", "About"),
                lang.t(
                    "من بناه، وبأي رخصة، وأين مصدره",
                    "Who built it, under what licence, and where the source is",
                ),
            ),
        ];

        egui::Area::new(egui::Id::new("settingsheet"))
            .order(egui::Order::Middle)
            .fixed_pos(pos2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.set_opacity(alpha);
                if alpha < 0.90 {
                    ui.disable();
                }
                let screen = ctx.viewport_rect();
                if ui
                    .interact(screen, egui::Id::new("settingscrim"), Sense::click())
                    .clicked()
                {
                    want_close = true;
                }
                ui.painter()
                    .rect_filled(screen, 0.0, pal.bg_deep.gamma_multiply(0.82));

                let cw = (screen.width() - 88.0).clamp(360.0, 900.0);
                let ch = (screen.height() - 74.0).max(300.0);
                let card = Rect::from_center_size(
                    pos2(
                        screen.center().x,
                        screen.center().y + 12.0 + (1.0 - alpha) * 22.0,
                    ),
                    vec2(cw, ch),
                );
                ui::glass_card(ui.painter(), card, &pal, pal.accent, 1.0);

                let head = Rect::from_min_size(
                    pos2(card.left() + 22.0, card.top() + 16.0),
                    vec2(card.width() - 44.0, 38.0),
                );
                ui.scope_builder(egui::UiBuilder::new().max_rect(head), |ui| {
                    let title = |ui: &mut egui::Ui| {
                        let l = if rtl {
                            Layout::right_to_left(Align::Center)
                        } else {
                            Layout::left_to_right(Align::Center)
                        };
                        ui.with_layout(l, |ui| {
                            let (r, _) =
                                ui.allocate_exact_size(vec2(22.0, 22.0), Sense::hover());
                            ui::nav_glyph(ui.painter(), r, 1, pal.accent);
                            ui.add_space(9.0);
                            ui.label(
                                RichText::new(lang.t("الإعدادات", "Settings"))
                                    .size(17.5)
                                    .strong(),
                            );
                        });
                    };
                    let done = |ui: &mut egui::Ui| {
                        let l = if rtl {
                            Layout::left_to_right(Align::Center)
                        } else {
                            Layout::right_to_left(Align::Center)
                        };
                        ui.with_layout(l, |ui| {
                            if ui::ghost_button(ui, &pal, lang.t("تم", "Done")).clicked() {
                                want_close = true;
                            }
                        });
                    };
                    let sides = egui::Sides::new().height(38.0);
                    if rtl {
                        sides.show(ui, done, title);
                    } else {
                        sides.show(ui, title, done);
                    }
                });

                let body_top = card.top() + 66.0;
                let nav_w = 194.0;
                let nav = if rtl {
                    Rect::from_min_max(
                        pos2(card.right() - 14.0 - nav_w, body_top),
                        pos2(card.right() - 14.0, card.bottom() - 16.0),
                    )
                } else {
                    Rect::from_min_max(
                        pos2(card.left() + 14.0, body_top),
                        pos2(card.left() + 14.0 + nav_w, card.bottom() - 16.0),
                    )
                };
                ui.painter().rect_filled(
                    nav.expand2(vec2(4.0, 6.0)),
                    16.0,
                    pal.bg_deep.gamma_multiply(0.45),
                );
                ui.scope_builder(
                    egui::UiBuilder::new().max_rect(nav.shrink2(vec2(8.0, 8.0))),
                    |ui| {
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 4.0;
                            for (kind, label, _) in sections {
                                if ui::nav_item(ui, &pal, rtl, kind, label, tab == kind)
                                    .clicked()
                                {
                                    tab = kind;
                                }
                            }
                        });
                    },
                );

                let line_x = if rtl {
                    nav.left() - 12.0
                } else {
                    nav.right() + 12.0
                };
                ui.painter().line_segment(
                    [
                        pos2(line_x, body_top + 4.0),
                        pos2(line_x, card.bottom() - 22.0),
                    ],
                    egui::Stroke::new(1.0, pal.line),
                );

                let pane = if rtl {
                    Rect::from_min_max(
                        pos2(card.left() + 24.0, body_top),
                        pos2(line_x - 20.0, card.bottom() - 18.0),
                    )
                } else {
                    Rect::from_min_max(
                        pos2(line_x + 20.0, body_top),
                        pos2(card.right() - 24.0, card.bottom() - 18.0),
                    )
                };
                let current = sections[tab.min(5) as usize];
                ui::pane_header(
                    ui.painter(),
                    Rect::from_min_size(pane.min, vec2(pane.width(), 50.0)),
                    &pal,
                    rtl,
                    current.1,
                    current.2,
                    pal.accent,
                );

                let body = Rect::from_min_max(
                    pos2(pane.left(), pane.top() + 62.0),
                    pane.max,
                );
                ui.scope_builder(egui::UiBuilder::new().max_rect(body), |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .id_salt(("settingpane", tab))
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing.y = 9.0;
                            match tab {
                                0 => {
                                    let (row, _) = ui.allocate_exact_size(
                                        vec2(ui.available_width(), 40.0),
                                        Sense::hover(),
                                    );
                                    ui.scope_builder(
                                        egui::UiBuilder::new().max_rect(row),
                                        |ui| {
                                            let l = if rtl {
                                                Layout::right_to_left(Align::Center)
                                            } else {
                                                Layout::left_to_right(Align::Center)
                                            };
                                            ui.with_layout(l, |ui| {
                                                ui.spacing_mut().item_spacing.x = 8.0;
                                                for (id, name, acc) in &profiles {
                                                    let label = if name.trim().is_empty() {
                                                        lang.t("الملف الرئيسي", "Main profile")
                                                            .to_string()
                                                    } else {
                                                        name.clone()
                                                    };
                                                    let col = theme::palette(
                                                        Accent::from_index(*acc),
                                                    )
                                                    .accent;
                                                    if ui::profile_chip(
                                                        ui,
                                                        &pal,
                                                        &label,
                                                        col,
                                                        *id == active_id,
                                                    )
                                                    .clicked()
                                                    {
                                                        pick_profile = Some(id.clone());
                                                    }
                                                }
                                                if profiles.len() < 12
                                                    && ui::icon_button(ui, &pal, "+", 38.0)
                                                        .clicked()
                                                {
                                                    new_profile = true;
                                                }
                                            });
                                        },
                                    );
                                    if ui::setting_row(
                                        ui,
                                        &pal,
                                        rtl,
                                        lang.t("اسم الملف الحالي", "Current profile name"),
                                        if active_name.trim().is_empty() {
                                            lang.t(
                                                "ما له اسم بعد — سمّه باسمك",
                                                "Unnamed yet - give it your name",
                                            )
                                        } else {
                                            &active_name
                                        },
                                        lang.t("تسمية", "Rename"),
                                    ) {
                                        rename_profile = true;
                                    }
                                    ui::info_row(
                                        ui,
                                        &pal,
                                        rtl,
                                        lang.t("حسابات هذا الملف", "Accounts in this profile"),
                                        &if rtl {
                                            format!("{} حساب", self.counts.iter().sum::<usize>())
                                        } else {
                                            format!(
                                                "{} accounts",
                                                self.counts.iter().sum::<usize>()
                                            )
                                        },
                                    );
                                    if ui::setting_row(
                                        ui,
                                        &pal,
                                        rtl,
                                        lang.t("غيّر الملف الشخصي", "Switch profile"),
                                        lang.t(
                                            "يرجّعك لشاشة اختيار الملفات",
                                            "Takes you back to the profile picker",
                                        ),
                                        lang.t("اعرض", "Show"),
                                    ) {
                                        want_picker = true;
                                    }
                                    if many
                                        && ui::setting_row(
                                            ui,
                                            &pal,
                                            rtl,
                                            lang.t("احذف هذا الملف", "Delete this profile"),
                                            lang.t(
                                                "يمسح حساباته المحفوظة هنا فقط، ولا يمس المنصّات",
                                                "Erases only the accounts saved here, never the platforms",
                                            ),
                                            lang.t("حذف", "Delete"),
                                        )
                                    {
                                        drop_profile = true;
                                    }
                                }
                                1 => {
                                    if let Some(i) = ui::swatches(
                                        ui,
                                        &pal,
                                        rtl,
                                        lang.t("لون الواجهة", "Interface colour"),
                                        Accent::from_index(accent_now).name(rtl),
                                        &swatch,
                                        accent_now,
                                    ) {
                                        pick_accent = Some(i);
                                    }
                                    if ui::setting_row(
                                        ui,
                                        &pal,
                                        rtl,
                                        lang.t("اللغة", "Language"),
                                        lang.t(
                                            "العربية بدعم كامل للاتجاه من اليمين",
                                            "Arabic with full right-to-left support",
                                        ),
                                        lang.label(),
                                    ) {
                                        want_lang = true;
                                    }
                                    if ui::toggle(
                                        ui,
                                        &pal,
                                        rtl,
                                        lang.t("الحركات والانتقالات", "Animations"),
                                        lang.t(
                                            "أطفئها لو تبي الواجهة فورية بلا أي حركة",
                                            "Turn off for an instant, motionless interface",
                                        ),
                                        self.settings.animations,
                                    ) {
                                        toggle_anim = true;
                                    }
                                    if ui::toggle(
                                        ui,
                                        &pal,
                                        rtl,
                                        lang.t("خلفية متحركة", "Living backdrop"),
                                        lang.t(
                                            "شبكة الخلايا والهالات تتنفّس خلف الواجهة",
                                            "The hex field and halos breathing behind the app",
                                        ),
                                        self.settings.live_backdrop,
                                    ) {
                                        toggle_backdrop = true;
                                    }
                                    if ui::toggle(
                                        ui,
                                        &pal,
                                        rtl,
                                        lang.t("لوحة المميزات", "Feature board"),
                                        lang.t(
                                            "اللوحة الجانبية في شاشة الحسابات",
                                            "The side board on the accounts screen",
                                        ),
                                        self.settings.showcase,
                                    ) {
                                        toggle_showcase = true;
                                    }
                                }
                                2 => {
                                    if ui::toggle(
                                        ui,
                                        &pal,
                                        rtl,
                                        lang.t("اسألني قبل كل تبديل", "Ask before every switch"),
                                        lang.t(
                                            "يمنع الضغطة الغلط من إغلاق لعبتك",
                                            "Stops a stray click from closing your game",
                                        ),
                                        self.settings.confirm_switch,
                                    ) {
                                        toggle_confirm = true;
                                    }
                                    if ui::toggle(
                                        ui,
                                        &pal,
                                        rtl,
                                        lang.t(
                                            "شغّل المنصّة بعد التبديل",
                                            "Launch platform after switching",
                                        ),
                                        lang.t(
                                            "يفتح اللانشر مباشرة بالحساب الجديد",
                                            "Opens the launcher right away",
                                        ),
                                        self.settings.launch_after_switch,
                                    ) {
                                        toggle_launch = true;
                                    }
                                    if ui::toggle(
                                        ui,
                                        &pal,
                                        rtl,
                                        lang.t(
                                            "أغلق بديل بعد التبديل",
                                            "Close badeel after switching",
                                        ),
                                        lang.t(
                                            "يختفي فور نجاح التبديل",
                                            "Disappears once the switch succeeds",
                                        ),
                                        self.settings.close_after_switch,
                                    ) {
                                        toggle_close = true;
                                    }
                                    if ui::toggle(
                                        ui,
                                        &pal,
                                        rtl,
                                        lang.t(
                                            "تخطَّ قائمة اختيار الحساب في ستيم",
                                            "Skip Steam's account chooser",
                                        ),
                                        lang.t(
                                            "بدونه يسألك ستيم عن الحساب في كل تشغيل",
                                            "Without it Steam asks on every start",
                                        ),
                                        self.settings.skip_steam_chooser,
                                    ) {
                                        toggle_chooser = true;
                                    }
                                    ui::info_row(
                                        ui,
                                        &pal,
                                        rtl,
                                        lang.t("عند الفشل", "On failure"),
                                        lang.t(
                                            "يرجع كل ملف إلى مكانه تلقائيًا",
                                            "Every file is rolled back automatically",
                                        ),
                                    );
                                }
                                3 => {
                                    ui::info_row(
                                        ui,
                                        &pal,
                                        rtl,
                                        lang.t("التشفير", "Encryption"),
                                        "AES-256-GCM · Argon2id · DPAPI",
                                    );
                                    ui::info_row(
                                        ui,
                                        &pal,
                                        rtl,
                                        lang.t("الارتباط", "Binding"),
                                        lang.t(
                                            "مربوط بحساب ويندوز وبهذا الجهاز",
                                            "Bound to this Windows account and PC",
                                        ),
                                    );
                                    if ui::setting_row(
                                        ui,
                                        &pal,
                                        rtl,
                                        lang.t("كلمة سر الخزنة", "Vault password"),
                                        lang.t(
                                            "طبقة ثانية فوق تشفير ويندوز",
                                            "A second layer over Windows encryption",
                                        ),
                                        if has_password {
                                            lang.t("تغيير", "Change")
                                        } else {
                                            lang.t("تفعيل", "Enable")
                                        },
                                    ) {
                                        want_password = true;
                                    }
                                    if has_password
                                        && ui::setting_row(
                                            ui,
                                            &pal,
                                            rtl,
                                            lang.t("إزالة كلمة السر", "Remove password"),
                                            lang.t(
                                                "تبقى الحسابات مشفّرة بحماية ويندوز",
                                                "Accounts stay encrypted with Windows protection",
                                            ),
                                            lang.t("إزالة", "Remove"),
                                        )
                                    {
                                        want_remove_password = true;
                                    }
                                    if ui::setting_row(
                                        ui,
                                        &pal,
                                        rtl,
                                        lang.t("القفل التلقائي", "Auto-lock"),
                                        lang.t(
                                            "يقفل الخزنة إذا طال سكونك — يحتاج كلمة سر",
                                            "Locks the vault after idle time - needs a password",
                                        ),
                                        &lock_label,
                                    ) {
                                        cycle_lock = true;
                                    }
                                    if ui::toggle(
                                        ui,
                                        &pal,
                                        rtl,
                                        lang.t("اقفل عند التصغير", "Lock when minimised"),
                                        lang.t(
                                            "أول ما تصغّر النافذة تنقفل الخزنة",
                                            "The vault locks the moment the window is minimised",
                                        ),
                                        self.settings.lock_on_minimize,
                                    ) {
                                        toggle_minlock = true;
                                    }
                                    if ui::toggle(
                                        ui,
                                        &pal,
                                        rtl,
                                        lang.t(
                                            "اخف معرّفات الحسابات",
                                            "Hide account identifiers",
                                        ),
                                        lang.t(
                                            "يستر الأرقام والأسماء التقنية وقت البث أو التسجيل",
                                            "Masks IDs and logins while streaming or recording",
                                        ),
                                        self.settings.mask_ids,
                                    ) {
                                        toggle_mask = true;
                                    }
                                    if ui::toggle(
                                        ui,
                                        &pal,
                                        rtl,
                                        lang.t(
                                            "أظهر بديل في نشاط ديسكورد",
                                            "Show badeel in your Discord activity",
                                        ),
                                        lang.t(
                                            "سطران عامّان بلا اسم حساب ولا منصّة، ولا يخرج اتصال من البرنامج",
                                            "Two generic lines, no account or platform name, and no connection leaves the app",
                                        ),
                                        self.settings.discord,
                                    ) {
                                        toggle_discord = true;
                                    }
                                    if ui::setting_row(
                                        ui,
                                        &pal,
                                        rtl,
                                        lang.t("مجلد البيانات", "Data folder"),
                                        lang.t(
                                            "حسابات هذا الملف الشخصي، مشفّرة",
                                            "This profile's accounts, encrypted",
                                        ),
                                        lang.t("افتح", "Open"),
                                    ) {
                                        want_folder = true;
                                    }
                                    if ui::setting_row(
                                        ui,
                                        &pal,
                                        rtl,
                                        lang.t("انقل حساباتك لجهاز ثاني", "Move accounts to another PC"),
                                        lang.t(
                                            "ملف واحد مقفل بكلمة سر تختارها، يفتح على أي جهاز",
                                            "One file locked with a password you pick, opens on any PC",
                                        ),
                                        lang.t("تصدير", "Export"),
                                    ) {
                                        want_export = true;
                                    }
                                    if ui::setting_row(
                                        ui,
                                        &pal,
                                        rtl,
                                        lang.t("استورد من ملف نقل", "Import a transfer file"),
                                        lang.t(
                                            "يضيف الحسابات الجديدة ولا يمسّ الموجودة",
                                            "Adds new accounts and leaves the ones here alone",
                                        ),
                                        lang.t("استيراد", "Import"),
                                    ) {
                                        want_import = true;
                                    }
                                }
                                4 => {
                                    if ui::toggle(
                                        ui,
                                        &pal,
                                        rtl,
                                        lang.t(
                                            "افحص التحديثات تلقائيًا",
                                            "Check for updates automatically",
                                        ),
                                        lang.t(
                                            "يسأل GitHub عن نسخة جديدة ولا يرسل له شي عنك",
                                            "Asks GitHub for a new version and sends nothing about you",
                                        ),
                                        self.settings.auto_update,
                                    ) {
                                        toggle_autoupd = true;
                                    }
                                    if ui::toggle(
                                        ui,
                                        &pal,
                                        rtl,
                                        lang.t("لوحة أثلاوي", "Athlawi's board"),
                                        lang.t(
                                            "بطاقة فيها كلمة أو برنامج ثاني لنا، تجي من موقعنا على GitHub ولا يطلع من جهازك شي",
                                            "A card with a word or another of our apps, fetched from our site on GitHub, nothing leaves your PC",
                                        ),
                                        self.settings.board,
                                    ) {
                                        toggle_board = true;
                                    }
                                    ui::info_row(
                                        ui,
                                        &pal,
                                        rtl,
                                        lang.t("الإصدار الحالي", "Current version"),
                                        update::current_version(),
                                    );
                                    if ui::setting_row(
                                        ui,
                                        &pal,
                                        rtl,
                                        lang.t("افحص الآن", "Check now"),
                                        lang.t(
                                            "يبحث عن إصدار أحدث ويخبرك",
                                            "Looks for a newer release and tells you",
                                        ),
                                        lang.t("افحص", "Check"),
                                    ) {
                                        want_check = true;
                                    }
                                }
                                _ => {
                                    ui::info_row(
                                        ui,
                                        &pal,
                                        rtl,
                                        lang.t("التأسيس والتطوير", "Founded and built by"),
                                        lang.t(
                                            "ريان الأثلاوي ومؤيد المطيري",
                                            "Ryan Athlawi and Moayad Almutairi",
                                        ),
                                    );
                                    ui::info_row(ui, &pal, rtl, lang.t("الرخصة", "Licence"), "GPL-3.0");
                                    ui::info_row(
                                        ui,
                                        &pal,
                                        rtl,
                                        lang.t("المصدر", "Source"),
                                        "github.com/Ryanathlawi/badeel",
                                    );
                                    if ui::setting_row(
                                        ui,
                                        &pal,
                                        rtl,
                                        lang.t("صفحة الفريق والقصة", "Team and story"),
                                        lang.t(
                                            "من بنى بديل، ولماذا بُني أصلًا",
                                            "Who built badeel, and why it exists at all",
                                        ),
                                        lang.t("افتح", "Open"),
                                    ) {
                                        want_about = true;
                                    }
                                    if ui::setting_row(
                                        ui,
                                        &pal,
                                        rtl,
                                        lang.t("الجولة التعريفية", "Product tour"),
                                        lang.t("اعرضها من جديد", "Show it again"),
                                        lang.t("ابدأ", "Start"),
                                    ) {
                                        want_tour = true;
                                    }
                                }
                            }
                            ui.add_space(8.0);
                        });
                });
                let _ = t;
            });

        self.settings_tab = tab;
        if want_picker {
            self.route = Route::Home;
            self.picking = true;
        }
        if want_close {
            self.go(Route::Home);
        }
        if want_about {
            self.go(Route::About);
        }
        if toggle_launch {
            self.settings.launch_after_switch = !self.settings.launch_after_switch;
        }
        if toggle_close {
            self.settings.close_after_switch = !self.settings.close_after_switch;
        }
        if toggle_confirm {
            self.settings.confirm_switch = !self.settings.confirm_switch;
        }
        if toggle_chooser {
            self.settings.skip_steam_chooser = !self.settings.skip_steam_chooser;
            if !self.settings.skip_steam_chooser {
                if let Some(r) = steam::root() {
                    let _ = steam::set_user_chooser(&r, true);
                }
            }
        }
        if toggle_anim {
            self.settings.animations = !self.settings.animations;
        }
        if toggle_backdrop {
            self.settings.live_backdrop = !self.settings.live_backdrop;
        }
        if toggle_showcase {
            self.settings.showcase = !self.settings.showcase;
        }
        if toggle_discord {
            self.settings.discord = !self.settings.discord;
            self.presence.set_enabled(self.settings.discord);
        }
        if toggle_board {
            self.settings.board = !self.settings.board;
            if self.settings.board && self.board_rx.is_none() {
                self.fetch_board();
            }
        }
        if toggle_mask {
            self.settings.mask_ids = !self.settings.mask_ids;
        }
        if toggle_minlock {
            self.settings.lock_on_minimize = !self.settings.lock_on_minimize;
        }
        if toggle_autoupd {
            self.settings.auto_update = !self.settings.auto_update;
        }
        if cycle_lock {
            self.settings.auto_lock_min = match self.settings.auto_lock_min {
                0 => 1,
                1 => 5,
                5 => 15,
                15 => 30,
                _ => 0,
            };
        }
        if want_lang {
            self.settings.lang = self.settings.lang.toggled();
        }
        if let Some(a) = pick_accent {
            let id = self.book.active.clone();
            if let Some(pr) = self.book.profiles.iter_mut().find(|p| p.id == id) {
                pr.accent = a;
            }
            let _ = profile::save(&self.book);
            self.restyle(ctx);
        }
        if let Some(id) = pick_profile {
            if id != self.book.active {
                match profile::activate(&mut self.book, &id) {
                    Ok(()) => {
                        self.restyle(ctx);
                        self.counts = catalog::PLATFORMS
                            .iter()
                            .map(|p| accounts_of(p).accounts.len())
                            .collect();
                        self.selected = None;
                        self.search.clear();
                        self.list_in = 0.0;
                        self.reload();
                        let who = self.book.current().name.clone();
                        self.toast(
                            format!("{} {who}", lang.t("الملف النشط:", "Active profile:")),
                            ToastKind::Ok,
                        );
                    }
                    Err(e) => self.toast(format!("{e:#}"), ToastKind::Err),
                }
            }
        }
        if new_profile {
            self.dialog = Dialog::NewProfile {
                name: String::new(),
                error: String::new(),
            };
        }
        if rename_profile {
            self.dialog = Dialog::RenameProfile {
                id: self.book.active.clone(),
                name: self.book.current().name.clone(),
            };
        }
        if drop_profile {
            let name = self.book.current().name.clone();
            self.dialog = Dialog::DropProfile {
                id: self.book.active.clone(),
                name,
            };
        }
        if want_password {
            self.dialog = Dialog::SetPassword {
                a: String::new(),
                b: String::new(),
                error: String::new(),
            };
        }
        if want_remove_password {
            self.has_pw = false;
            match vault::set_password(None, None) {
                Ok(()) => self.toast(
                    lang.t("أُزيلت كلمة السر", "Password removed"),
                    ToastKind::Ok,
                ),
                Err(e) => self.toast(format!("{e:#}"), ToastKind::Err),
            }
        }
        if want_export {
            self.dialog = Dialog::Export {
                a: String::new(),
                b: String::new(),
                error: String::new(),
            };
        }
        if want_import {
            let lang = self.settings.lang;
            if let Some(file) = rfd::FileDialog::new()
                .add_filter(lang.t("ملف نقل بديل", "badeel transfer file"), &["badeel"])
                .set_title(lang.t("اختر ملف النقل", "Pick the transfer file"))
                .pick_file()
            {
                self.dialog = Dialog::Import {
                    file,
                    password: String::new(),
                    error: String::new(),
                };
            }
        }
        if want_folder {
            let dir = paths::profile_root();
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::process::Command::new("explorer.exe").arg(&dir).spawn();
        }
        if want_tour {
            self.tour = Some(0);
            self.tour_t = 0.0;
            self.go(Route::Home);
        }
        if want_check {
            self.update_rx = Some(update::spawn_check());
            self.toast(
                lang.t("أفحص التحديثات…", "Checking for updates…"),
                ToastKind::Ok,
            );
        }
    }

    fn pick_avatar(&mut self, id: &str) {
        let lang = self.settings.lang;
        let picked = rfd::FileDialog::new()
            .add_filter(lang.t("صورة", "Image"), &["png", "jpg", "jpeg", "webp", "gif"])
            .set_title(lang.t("اختر صورة للحساب", "Pick an account picture"))
            .pick_file();
        let Some(src) = picked else { return };
        let dir = paths::profile_root().join("avatars");
        let _ = std::fs::create_dir_all(&dir);
        let ext = src
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_else(|| "png".into());
        let dest = dir.join(format!(
            "{}-{}.{ext}",
            self.platform.id,
            paths::sanitize(id)
        ));
        match std::fs::copy(&src, &dest) {
            Ok(_) => {
                if let Some(acc) = self.accounts.accounts.iter_mut().find(|a| a.id == id) {
                    acc.avatar = Some(dest.to_string_lossy().to_string());
                }
                let _ = store::save(self.platform.id, &self.accounts);
                self.toast(lang.t("تم تغيير الصورة", "Picture updated"), ToastKind::Ok);
            }
            Err(e) => self.toast(format!("{e}"), ToastKind::Err),
        }
    }

    fn overlay(&mut self, ctx: &egui::Context, t: f32) {
        let pal = self.pal;
        let lang = self.settings.lang;

        if let (Some(job), Some(step)) = (&self.busy, &self.step) {
            let label = match step {
                switch::Step::Closing => lang.t("أُغلق المنصّة…", "Closing the platform…"),
                switch::Step::Saving => lang.t("أحفظ الحساب الحالي…", "Saving current account…"),
                switch::Step::Restoring => lang.t("أركّب الحساب…", "Restoring account…"),
                switch::Step::Launching => lang.t("أشغّل…", "Launching…"),
                switch::Step::Done => lang.t("تم", "Done"),
            };
            let secs = job.started.elapsed().as_secs();
            let line = if secs >= 3 {
                format!("{label}  ({secs}s)")
            } else {
                label.to_string()
            };
            let hint = if secs >= 6 && matches!(step, switch::Step::Closing) {
                lang.t(
                    "المنصّة تأخذ وقتها في الإغلاق — سأغلقها بالقوة بعد قليل",
                    "The platform is slow to close — it will be forced shortly",
                )
            } else {
                ""
            };
            ui::progress_overlay(ctx, &pal, &job.label, &line, hint, t);
        }

        self.toasts.retain(|x| x.born.elapsed().as_secs_f32() < 4.2);
        let toasts: Vec<(String, ToastKind, f32)> = self
            .toasts
            .iter()
            .map(|x| (x.text.clone(), x.kind, x.born.elapsed().as_secs_f32()))
            .collect();
        ui::toasts(ctx, &pal, &toasts);
    }

    fn tour_overlay(&mut self, ctx: &egui::Context, t: f32) {
        let Some(step) = self.tour else { return };
        let pal = self.pal;
        let lang = self.settings.lang;
        let rtl = lang.rtl();

        let steps: [(&str, &str); TOUR_STEPS] = [
            (
                lang.t("أهلًا بك في بديل", "Welcome to badeel"),
                lang.t(
                    "مبدّل حسابات لسبع منصّات، ستيم وباتل نت ورايوت وإيبك ويوبيسوفت وروكستار وجوج، بلا كلمات سر وبلا إعادة تسجيل دخول في كل مرة، من تأسيس ريان الأثلاوي ومؤيد المطيري",
                    "One switcher for Steam, Battle.net, Riot, Epic, Ubisoft, Rockstar and GOG. No passwords, no signing in again. Founded by Ryan Athlawi and Moayad Almutairi.",
                ),
            ),
            (
                lang.t("اختر منصّتك", "Pick your platform"),
                lang.t(
                    "الشاشة الأولى تعرض منصّاتك المثبّتة وعدد الحسابات المحفوظة في كل واحدة، واضغط الرقم للاختيار السريع",
                    "The first screen shows your installed platforms and how many accounts you saved in each. Press its number to jump.",
                ),
            ),
            (
                lang.t("احفظ حسابك الحالي", "Save the account you are on"),
                lang.t(
                    "سجّل دخولك في المنصّة كالمعتاد مرة واحدة، ثم اضغط «أضف الحساب الحالي» وسمِّه، وكرّر ذلك لكل حساب",
                    "Sign in on the platform once as usual, then press Add current account and name it. Repeat per account.",
                ),
            ),
            (
                lang.t("بدّل بضغطة", "Switch in one click"),
                lang.t(
                    "بديل يغلق المنصّة، يحفظ جلستك الحالية، يركّب الحساب المطلوب، ثم يشغّلها من جديد، وإن تعثّر شيء رجع كل شيء كما كان",
                    "badeel closes the platform, saves your current session, restores the one you picked and starts it again. If anything fails, everything goes back as it was.",
                ),
            ),
            (
                lang.t("كل شي مشفّر", "Everything is encrypted"),
                lang.t(
                    "جلساتك تُحفظ مشفّرة ومربوطة بحساب ويندوز وبجهازك، ونسخها إلى جهاز آخر لا تفتح، وتقدر تضيف كلمة سر فوقها من الإعدادات",
                    "Your sessions are stored encrypted and bound to this Windows account and PC — a copy on another machine will not open. Add a password on top from Settings.",
                ),
            ),
        ];

        let mut next = false;
        let mut prev = false;
        let mut done = false;
        let mut skipped = false;

        egui::Area::new(egui::Id::new("tour"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos2(0.0, 0.0))
            .show(ctx, |ui| {
                let screen = ctx.viewport_rect();
                ui.painter()
                    .rect_filled(screen, 0.0, pal.bg_deep.gamma_multiply(0.86));

                let e = back_out(self.tour_t);
                let card = Rect::from_center_size(
                    pos2(screen.center().x, screen.center().y + (1.0 - e) * 26.0),
                    vec2(470.0_f32.min(screen.width() - 60.0), 268.0),
                );
                let p = ui.painter();
                p.rect_filled(
                    card.translate(vec2(0.0, 16.0)),
                    24.0,
                    Color32::BLACK.gamma_multiply(0.40),
                );
                p.rect_filled(card, 24.0, pal.panel);
                p.rect_stroke(
                    card,
                    24.0,
                    egui::Stroke::new(1.0, pal.line_hi),
                    egui::StrokeKind::Inside,
                );

                let mark = Rect::from_center_size(
                    pos2(card.center().x, card.top() + 48.0),
                    vec2(52.0, 52.0),
                );
                ui::brand_mark(p, mark, &pal, t);

                let (title, body) = steps[step.min(TOUR_STEPS - 1)];
                p.text(
                    pos2(card.center().x, card.top() + 96.0),
                    Align2::CENTER_CENTER,
                    title,
                    FontId::proportional(19.0),
                    pal.text,
                );

                let text_rect = Rect::from_min_size(
                    pos2(card.left() + 30.0, card.top() + 118.0),
                    vec2(card.width() - 60.0, 86.0),
                );
                ui.scope_builder(egui::UiBuilder::new().max_rect(text_rect), |ui| {
                    ui.with_layout(
                        Layout::top_down(if rtl { Align::RIGHT } else { Align::LEFT }),
                        |ui| {
                            ui.label(
                                RichText::new(body)
                                    .size(12.5)
                                    .color(pal.muted),
                            );
                        },
                    );
                });

                for i in 0..TOUR_STEPS {
                    let on = i == step;
                    let x = card.center().x - (TOUR_STEPS as f32 - 1.0) * 7.0 + i as f32 * 14.0;
                    ui.painter().circle_filled(
                        pos2(x, card.bottom() - 62.0),
                        if on { 4.0 } else { 3.0 },
                        if on {
                            pal.accent
                        } else {
                            pal.line_hi.gamma_multiply(0.7)
                        },
                    );
                }

                let row = Rect::from_min_size(
                    pos2(card.left() + 26.0, card.bottom() - 52.0),
                    vec2(card.width() - 52.0, 42.0),
                );
                ui.scope_builder(egui::UiBuilder::new().max_rect(row), |ui| {
                    let last = step + 1 >= TOUR_STEPS;
                    let nav_side = |ui: &mut egui::Ui| {
                        let l = if rtl {
                            Layout::right_to_left(Align::Center)
                        } else {
                            Layout::left_to_right(Align::Center)
                        };
                        ui.with_layout(l, |ui| {
                            if ui::solid_button(
                                ui,
                                &pal,
                                if last {
                                    lang.t("يلا نبدأ", "Let's go")
                                } else {
                                    lang.t("التالي", "Next")
                                },
                                pal.accent_deep,
                            )
                            .clicked()
                            {
                                if last {
                                    done = true;
                                } else {
                                    next = true;
                                }
                            }
                            if step > 0
                                && ui::ghost_button(ui, &pal, lang.t("رجوع", "Back")).clicked()
                            {
                                prev = true;
                            }
                        });
                    };
                    let skip_side = |ui: &mut egui::Ui| {
                        let l = if rtl {
                            Layout::left_to_right(Align::Center)
                        } else {
                            Layout::right_to_left(Align::Center)
                        };
                        ui.with_layout(l, |ui| {
                            if ui::ghost_button(ui, &pal, lang.t("تخطّي", "Skip")).clicked() {
                                skipped = true;
                            }
                        });
                    };
                    let sides = egui::Sides::new().height(40.0);
                    if rtl {
                        sides.show(ui, skip_side, nav_side);
                    } else {
                        sides.show(ui, nav_side, skip_side);
                    }
                });
            });

        if next {
            self.tour = Some(step + 1);
            self.tour_t = 0.0;
        }
        if prev {
            self.tour = Some(step.saturating_sub(1));
            self.tour_t = 0.0;
        }
        if done || skipped {
            self.tour = None;
            self.settings.seen_tour = true;
        }
    }

    fn dialogs(&mut self, ctx: &egui::Context, rtl: bool) {
        let pal = self.pal;
        let lang = self.settings.lang;
        let mut close = false;

        let mut found: Option<(&'static Platform, String, bool)> = None;
        let mut export_pw: Option<String> = None;
        let mut import_job: Option<(std::path::PathBuf, String)> = None;
        match &mut self.dialog {
            Dialog::None => {}
            Dialog::Export { a, b, error } => {
                let mut confirm = false;
                let modal = egui::Modal::new(egui::Id::new("export")).show(ctx, |ui| {
                    ui.set_width(380.0);
                    ui::dialog_title(ui, &pal, rtl, lang.t("انقل حساباتك", "Move your accounts"));
                    ui.label(
                        RichText::new(lang.t(
                            "اختر كلمة سر تفتح بها الملف على الجهاز الثاني، وما نقدر نسترجعها لو نسيتها",
                            "Pick a password to open the file on the other PC. It can't be recovered if you forget it.",
                        ))
                        .size(11.5)
                        .color(pal.text),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(lang.t(
                            "تنتقل الأسماء والصور والجلسات، وبعض المنصّات تربط الجلسة بجهازها فقد تطلب تسجيل الدخول مرة على الجهاز الجديد",
                            "Names, pictures and sessions all move. Some platforms tie a session to the PC, so they may ask you to sign in once on the new one.",
                        ))
                        .size(10.5)
                        .color(pal.muted),
                    );
                    ui.add_space(10.0);
                    ui.add(
                        egui::TextEdit::singleline(a)
                            .password(true)
                            .hint_text(lang.t("كلمة السر، 8 أحرف أو أكثر", "password, 8 characters or more"))
                            .desired_width(f32::INFINITY),
                    );
                    ui.add(
                        egui::TextEdit::singleline(b)
                            .password(true)
                            .hint_text(lang.t("أعد كتابتها", "type it again"))
                            .desired_width(f32::INFINITY),
                    );
                    if !error.is_empty() {
                        ui.label(RichText::new(error.as_str()).size(11.0).color(pal.danger));
                    }
                    ui.add_space(12.0);
                    let l = if rtl {
                        Layout::right_to_left(Align::Center)
                    } else {
                        Layout::left_to_right(Align::Center)
                    };
                    ui.with_layout(l, |ui| {
                        if ui::solid_button(ui, &pal, lang.t("صدّر", "Export"), pal.accent_deep)
                            .clicked()
                            || ui.input(|i| i.key_pressed(egui::Key::Enter))
                        {
                            confirm = true;
                        }
                        if ui::ghost_button(ui, &pal, lang.t("إلغاء", "Cancel")).clicked() {
                            close = true;
                        }
                    });
                });
                if confirm {
                    if a.chars().count() < 8 {
                        *error = lang
                            .t("كلمة السر لازم تكون 8 أحرف أو أكثر", "Use at least 8 characters")
                            .to_string();
                    } else if a != b {
                        *error = lang
                            .t("الكلمتان مو متطابقتين", "The two don't match")
                            .to_string();
                    } else {
                        export_pw = Some(a.clone());
                        close = true;
                    }
                }
                if modal.should_close() {
                    close = true;
                }
            }
            Dialog::Import { file, password, error } => {
                let mut confirm = false;
                let name = file
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let modal = egui::Modal::new(egui::Id::new("import")).show(ctx, |ui| {
                    ui.set_width(380.0);
                    ui::dialog_title(ui, &pal, rtl, lang.t("استيراد الحسابات", "Import accounts"));
                    ui.label(RichText::new(name.as_str()).size(11.0).color(pal.accent));
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(lang.t(
                            "الحسابات الموجودة هنا ما تُمسّ، يُضاف الجديد بس",
                            "Accounts already here stay as they are, only new ones are added.",
                        ))
                        .size(11.0)
                        .color(pal.muted),
                    );
                    ui.add_space(10.0);
                    ui.add(
                        egui::TextEdit::singleline(password)
                            .password(true)
                            .hint_text(lang.t("كلمة سر الملف", "the file's password"))
                            .desired_width(f32::INFINITY),
                    );
                    if !error.is_empty() {
                        ui.label(RichText::new(error.as_str()).size(11.0).color(pal.danger));
                    }
                    ui.add_space(12.0);
                    let l = if rtl {
                        Layout::right_to_left(Align::Center)
                    } else {
                        Layout::left_to_right(Align::Center)
                    };
                    ui.with_layout(l, |ui| {
                        if ui::solid_button(ui, &pal, lang.t("استورد", "Import"), pal.accent_deep)
                            .clicked()
                            || ui.input(|i| i.key_pressed(egui::Key::Enter))
                        {
                            confirm = true;
                        }
                        if ui::ghost_button(ui, &pal, lang.t("إلغاء", "Cancel")).clicked() {
                            close = true;
                        }
                    });
                });
                if confirm {
                    if password.is_empty() {
                        *error = lang.t("اكتب كلمة سر الملف", "Enter the file's password").to_string();
                    } else {
                        import_job = Some((file.clone(), password.clone()));
                        close = true;
                    }
                }
                if modal.should_close() {
                    close = true;
                }
            }
            Dialog::Find { query, pick } => {
                let t = self.t0.elapsed().as_secs_f32();
                let hits = rank(&self.find, query);
                let n = hits.len();
                let (up, down, enter) = ctx.input(|i| {
                    (
                        i.key_pressed(egui::Key::ArrowUp),
                        i.key_pressed(egui::Key::ArrowDown),
                        i.key_pressed(egui::Key::Enter),
                    )
                });
                if n > 0 {
                    if down {
                        *pick = (*pick + 1) % n;
                    }
                    if up {
                        *pick = (*pick + n - 1) % n;
                    }
                    *pick = (*pick).min(n - 1);
                }
                let mut chosen = (enter && n > 0).then_some(*pick);
                let before = query.clone();
                let modal = egui::Modal::new(egui::Id::new("find")).show(ctx, |ui| {
                    ui.set_width(460.0);
                    ui::dialog_title(
                        ui,
                        &pal,
                        rtl,
                        lang.t("ابحث في كل حساباتك", "Find any account"),
                    );
                    ui.add(
                        egui::TextEdit::singleline(query)
                            .hint_text(lang.t(
                                "اسم الحساب أو المنصّة",
                                "An account or platform name",
                            ))
                            .desired_width(f32::INFINITY),
                    )
                    .request_focus();
                    ui.add_space(10.0);
                    if hits.is_empty() {
                        ui.label(
                            RichText::new(lang.t(
                                "ما لقيت حساب بهذا الاسم",
                                "No account matches that",
                            ))
                            .color(pal.muted),
                        );
                    }
                    ui.spacing_mut().item_spacing.y = 6.0;
                    for (i, h) in hits.iter().enumerate() {
                        let act = ui::account_row(
                            ui, &pal, rtl, &h.acc, h.plat.id, h.live, i == *pick, 1.0, t, lang,
                        );
                        if act.is_some_and(|a| a != 2) {
                            chosen = Some(i);
                        }
                    }
                });
                if *query != before {
                    *pick = 0;
                }
                if let Some(h) = chosen.and_then(|i| hits.get(i)) {
                    found = Some((h.plat, h.acc.id.clone(), h.live));
                    close = true;
                }
                if modal.should_close() {
                    close = true;
                }
            }
            Dialog::AddAccount { name, error } => {
                let mut confirm = false;
                let modal = egui::Modal::new(egui::Id::new("add")).show(ctx, |ui| {
                    ui.set_width(360.0);
                    ui::dialog_title(
                        ui,
                        &pal,
                        rtl,
                        lang.t("إضافة الحساب الحالي", "Add current account"),
                    );
                    ui.label(
                        RichText::new(lang.t(
                            "يُحفظ الحساب المسجَّل دخوله الآن، مشفّرًا على جهازك",
                            "Saves the account signed in right now, encrypted on your PC.",
                        ))
                        .size(11.5)
                        .color(pal.muted),
                    );
                    ui.add_space(10.0);
                    let r = ui.add(
                        egui::TextEdit::singleline(name)
                            .hint_text(lang.t("اسم الحساب", "account name"))
                            .desired_width(f32::INFINITY),
                    );
                    r.request_focus();
                    if !error.is_empty() {
                        ui.label(RichText::new(error.as_str()).size(11.0).color(pal.danger));
                    }
                    ui.add_space(12.0);
                    let l = if rtl {
                        Layout::right_to_left(Align::Center)
                    } else {
                        Layout::left_to_right(Align::Center)
                    };
                    ui.with_layout(l, |ui| {
                        if ui::solid_button(ui, &pal, lang.t("حفظ", "Save"), pal.accent_deep)
                            .clicked()
                            || ui.input(|i| i.key_pressed(egui::Key::Enter))
                        {
                            confirm = true;
                        }
                        if ui::ghost_button(ui, &pal, lang.t("إلغاء", "Cancel")).clicked() {
                            close = true;
                        }
                    });
                });
                if modal.should_close() {
                    close = true;
                }
                if confirm {
                    let n = name.trim().to_string();
                    if n.is_empty() {
                        *error = lang.t("اكتب اسمًا", "Enter a name").to_string();
                    } else {
                        self.add_current(n);
                        close = true;
                    }
                }
            }
            Dialog::Rename { id, name } => {
                let id = id.clone();
                let mut new_name = name.clone();
                let mut confirm = false;
                let modal = egui::Modal::new(egui::Id::new("rename")).show(ctx, |ui| {
                    ui.set_width(340.0);
                    ui::dialog_title(ui, &pal, rtl, lang.t("إعادة التسمية", "Rename"));
                    ui.add(egui::TextEdit::singleline(&mut new_name).desired_width(f32::INFINITY));
                    ui.add_space(12.0);
                    let l = if rtl {
                        Layout::right_to_left(Align::Center)
                    } else {
                        Layout::left_to_right(Align::Center)
                    };
                    ui.with_layout(l, |ui| {
                        if ui::solid_button(ui, &pal, lang.t("حفظ", "Save"), pal.accent_deep)
                            .clicked()
                            || ui.input(|i| i.key_pressed(egui::Key::Enter))
                        {
                            confirm = true;
                        }
                        if ui::ghost_button(ui, &pal, lang.t("إلغاء", "Cancel")).clicked() {
                            close = true;
                        }
                    });
                });
                if modal.should_close() {
                    close = true;
                }
                *name = new_name.clone();
                if confirm {
                    if let Some(acc) = self.accounts.accounts.iter_mut().find(|a| a.id == id) {
                        acc.name = new_name.trim().to_string();
                    }
                    let _ = store::save(self.platform.id, &self.accounts);
                    close = true;
                }
            }
            Dialog::Confirm { id, name } => {
                let (id, name) = (id.clone(), name.clone());
                let mut confirm = false;
                let modal = egui::Modal::new(egui::Id::new("confirm")).show(ctx, |ui| {
                    ui.set_width(350.0);
                    ui::dialog_title(ui, &pal, rtl, lang.t("حذف الحساب", "Forget account"));
                    ui.label(
                        RichText::new(format!(
                            "{} «{name}». {}",
                            lang.t("سيُحذف من بديل", "Removes it from badeel"),
                            lang.t(
                                "الحساب نفسه لا يُمسّ — تُحذف نسخته المحفوظة هنا فقط",
                                "The account itself is untouched — only the copy saved here."
                            )
                        ))
                        .size(12.0)
                        .color(pal.muted),
                    );
                    ui.add_space(12.0);
                    let l = if rtl {
                        Layout::right_to_left(Align::Center)
                    } else {
                        Layout::left_to_right(Align::Center)
                    };
                    ui.with_layout(l, |ui| {
                        if ui::solid_button(ui, &pal, lang.t("حذف", "Forget"), pal.danger).clicked()
                        {
                            confirm = true;
                        }
                        if ui::ghost_button(ui, &pal, lang.t("إلغاء", "Cancel")).clicked() {
                            close = true;
                        }
                    });
                });
                if modal.should_close() {
                    close = true;
                }
                if confirm {
                    if let Err(e) = switch::forget(self.platform, &id) {
                        self.toast(format!("{e:#}"), ToastKind::Err);
                    }
                    self.reload();
                    close = true;
                }
            }
            Dialog::Unlock { password, error } => {
                let mut try_open = false;
                egui::Modal::new(egui::Id::new("unlock")).show(ctx, |ui| {
                    ui.set_width(340.0);
                    ui::dialog_title(ui, &pal, rtl, lang.t("الخزنة مقفلة", "Vault locked"));
                    ui.label(
                        RichText::new(lang.t(
                            "حساباتك مشفّرة، أدخل كلمة السر لفتحها",
                            "Your accounts are encrypted. Enter the password.",
                        ))
                        .size(11.5)
                        .color(pal.muted),
                    );
                    ui.add_space(10.0);
                    let r = ui.add(
                        egui::TextEdit::singleline(password)
                            .password(true)
                            .desired_width(f32::INFINITY),
                    );
                    r.request_focus();
                    if !error.is_empty() {
                        ui.label(RichText::new(error.as_str()).size(11.0).color(pal.danger));
                    }
                    ui.add_space(12.0);
                    if ui::solid_button(ui, &pal, lang.t("فتح", "Unlock"), pal.accent_deep).clicked()
                        || ui.input(|i| i.key_pressed(egui::Key::Enter))
                    {
                        try_open = true;
                    }
                });
                if try_open {
                    match vault::open(Some(password.as_str())) {
                        Ok(k) => {
                            self.key = Some(Arc::new(k));
                            close = true;
                        }
                        Err(e) => *error = format!("{e:#}"),
                    }
                }
            }
            Dialog::SetPassword { a, b, error } => {
                let mut confirm = false;
                let modal = egui::Modal::new(egui::Id::new("setpw")).show(ctx, |ui| {
                    ui.set_width(350.0);
                    ui::dialog_title(ui, &pal, rtl, lang.t("كلمة سر الخزنة", "Vault password"));
                    ui.label(
                        RichText::new(lang.t(
                            "لا يمكن استرجاعها — احفظها في مكان آمن",
                            "It cannot be recovered — keep it somewhere safe.",
                        ))
                        .size(11.5)
                        .color(pal.muted),
                    );
                    ui.add_space(10.0);
                    ui.add(
                        egui::TextEdit::singleline(a)
                            .password(true)
                            .hint_text(lang.t("كلمة السر", "password"))
                            .desired_width(f32::INFINITY),
                    );
                    ui.add(
                        egui::TextEdit::singleline(b)
                            .password(true)
                            .hint_text(lang.t("تأكيد", "confirm"))
                            .desired_width(f32::INFINITY),
                    );
                    if !error.is_empty() {
                        ui.label(RichText::new(error.as_str()).size(11.0).color(pal.danger));
                    }
                    ui.add_space(12.0);
                    let l = if rtl {
                        Layout::right_to_left(Align::Center)
                    } else {
                        Layout::left_to_right(Align::Center)
                    };
                    ui.with_layout(l, |ui| {
                        if ui::solid_button(ui, &pal, lang.t("تفعيل", "Enable"), pal.accent_deep)
                            .clicked()
                        {
                            confirm = true;
                        }
                        if ui::ghost_button(ui, &pal, lang.t("إلغاء", "Cancel")).clicked() {
                            close = true;
                        }
                    });
                });
                if modal.should_close() {
                    close = true;
                }
                if confirm {
                    if a != b {
                        *error = lang.t("غير متطابقتين", "They do not match").to_string();
                    } else if a.chars().count() < 5 {
                        *error = lang
                            .t("خمسة أحرف على الأقل", "At least five characters")
                            .to_string();
                    } else {
                        match vault::set_password(None, Some(a.as_str())) {
                            Ok(()) => {
                                self.has_pw = true;
                                self.toast(
                                    lang.t("فُعّلت كلمة السر", "Password enabled"),
                                    ToastKind::Ok,
                                );
                                close = true;
                            }
                            Err(e) => *error = format!("{e:#}"),
                        }
                    }
                }
            }
            Dialog::ConfirmSwitch { id, name } => {
                let (id, name) = (id.clone(), name.clone());
                let mut go = false;
                let plat = self.platform.name(rtl);
                let modal = egui::Modal::new(egui::Id::new("goswitch")).show(ctx, |ui| {
                    ui.set_width(380.0);
                    ui::dialog_title(ui, &pal, rtl, lang.t("تأكيد التبديل", "Confirm the switch"));
                    ui.label(
                        RichText::new(format!(
                            "{} «{name}». {} {plat} {}",
                            lang.t("سيُبدّل إلى", "Switching to"),
                            lang.t("سيُغلق", "This closes"),
                            lang.t("ويُعاد تشغيله", "and starts it again."),
                        ))
                        .size(12.0)
                        .color(pal.muted),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(lang.t(
                            "لو عندك لعبة شغّالة أغلقها أولًا",
                            "If a game is running, close it first.",
                        ))
                        .size(11.0)
                        .color(pal.warn),
                    );
                    ui.add_space(12.0);
                    let l = if rtl {
                        Layout::right_to_left(Align::Center)
                    } else {
                        Layout::left_to_right(Align::Center)
                    };
                    ui.with_layout(l, |ui| {
                        if ui::solid_button(ui, &pal, lang.t("بدّل الآن", "Switch now"), pal.accent_deep)
                            .clicked()
                        {
                            go = true;
                        }
                        if ui::ghost_button(ui, &pal, lang.t("إلغاء", "Cancel")).clicked() {
                            close = true;
                        }
                    });
                });
                if modal.should_close() {
                    close = true;
                }
                if go {
                    self.start_switch(id);
                    close = true;
                }
            }
            Dialog::NewProfile { name, error } => {
                let mut confirm = false;
                let mut typed = name.clone();
                let modal = egui::Modal::new(egui::Id::new("newprof")).show(ctx, |ui| {
                    ui.set_width(360.0);
                    ui::dialog_title(ui, &pal, rtl, lang.t("ملف شخصي جديد", "New profile"));
                    ui.label(
                        RichText::new(lang.t(
                            "ملف مستقل بحساباته ولونه وإعدادات مظهره — مناسب لو أكثر من شخص يستخدم الجهاز",
                            "A separate space with its own accounts, colour and look — for when more than one person uses this PC.",
                        ))
                        .size(11.5)
                        .color(pal.muted),
                    );
                    ui.add_space(10.0);
                    let r = ui.add(
                        egui::TextEdit::singleline(&mut typed)
                            .hint_text(lang.t("اسم الملف", "profile name"))
                            .desired_width(f32::INFINITY),
                    );
                    r.request_focus();
                    if !error.is_empty() {
                        ui.label(RichText::new(error.as_str()).size(11.0).color(pal.danger));
                    }
                    ui.add_space(12.0);
                    let l = if rtl {
                        Layout::right_to_left(Align::Center)
                    } else {
                        Layout::left_to_right(Align::Center)
                    };
                    ui.with_layout(l, |ui| {
                        if ui::solid_button(ui, &pal, lang.t("إنشاء", "Create"), pal.accent_deep)
                            .clicked()
                            || ui.input(|i| i.key_pressed(egui::Key::Enter))
                        {
                            confirm = true;
                        }
                        if ui::ghost_button(ui, &pal, lang.t("إلغاء", "Cancel")).clicked() {
                            close = true;
                        }
                    });
                });
                *name = typed.clone();
                if modal.should_close() {
                    close = true;
                }
                if confirm {
                    let n = typed.trim().to_string();
                    if n.is_empty() {
                        *error = lang.t("اكتب اسمًا", "Enter a name").to_string();
                    } else {
                        let accent = (self.book.profiles.len() as u8) % 8;
                        match profile::create(&mut self.book, &n, accent) {
                            Ok(id) => {
                                self.use_profile(&id, ctx);
                                self.toast(
                                    lang.t("أُنشئ الملف", "Profile created"),
                                    ToastKind::Ok,
                                );
                                close = true;
                            }
                            Err(e) => *error = format!("{e:#}"),
                        }
                    }
                }
            }
            Dialog::RenameProfile { id, name } => {
                let id = id.clone();
                let mut typed = name.clone();
                let mut confirm = false;
                let modal = egui::Modal::new(egui::Id::new("renprof")).show(ctx, |ui| {
                    ui.set_width(340.0);
                    ui::dialog_title(ui, &pal, rtl, lang.t("اسم الملف الشخصي", "Profile name"));
                    let r = ui.add(
                        egui::TextEdit::singleline(&mut typed).desired_width(f32::INFINITY),
                    );
                    r.request_focus();
                    ui.add_space(12.0);
                    let l = if rtl {
                        Layout::right_to_left(Align::Center)
                    } else {
                        Layout::left_to_right(Align::Center)
                    };
                    ui.with_layout(l, |ui| {
                        if ui::solid_button(ui, &pal, lang.t("حفظ", "Save"), pal.accent_deep)
                            .clicked()
                            || ui.input(|i| i.key_pressed(egui::Key::Enter))
                        {
                            confirm = true;
                        }
                        if ui::ghost_button(ui, &pal, lang.t("إلغاء", "Cancel")).clicked() {
                            close = true;
                        }
                    });
                });
                *name = typed.clone();
                if modal.should_close() {
                    close = true;
                }
                if confirm {
                    if let Some(pr) = self.book.profiles.iter_mut().find(|p| p.id == id) {
                        pr.name = typed.trim().to_string();
                    }
                    let _ = profile::save(&self.book);
                    close = true;
                }
            }
            Dialog::DropProfile { id, name } => {
                let (id, name) = (id.clone(), name.clone());
                let mut confirm = false;
                let modal = egui::Modal::new(egui::Id::new("dropprof")).show(ctx, |ui| {
                    ui.set_width(360.0);
                    ui::dialog_title(ui, &pal, rtl, lang.t("حذف الملف الشخصي", "Delete profile"));
                    ui.label(
                        RichText::new(format!(
                            "«{name}» — {}",
                            lang.t(
                                "تُمسح الحسابات المحفوظة فيه نهائيًا، وحساباتك في المنصّات لا تُمسّ",
                                "Its saved accounts are erased for good. Your accounts on the platforms are untouched."
                            )
                        ))
                        .size(12.0)
                        .color(pal.muted),
                    );
                    ui.add_space(12.0);
                    let l = if rtl {
                        Layout::right_to_left(Align::Center)
                    } else {
                        Layout::left_to_right(Align::Center)
                    };
                    ui.with_layout(l, |ui| {
                        if ui::solid_button(ui, &pal, lang.t("حذف", "Delete"), pal.danger).clicked() {
                            confirm = true;
                        }
                        if ui::ghost_button(ui, &pal, lang.t("إلغاء", "Cancel")).clicked() {
                            close = true;
                        }
                    });
                });
                if modal.should_close() {
                    close = true;
                }
                if confirm {
                    match profile::remove(&mut self.book, &id) {
                        Ok(()) => {
                            self.restyle(ctx);
                            self.counts = catalog::PLATFORMS
                                .iter()
                                .map(|p| accounts_of(p).accounts.len())
                                .collect();
                            self.selected = None;
                            self.list_in = 0.0;
                            self.reload();
                            self.toast(lang.t("حُذف الملف", "Profile deleted"), ToastKind::Ok);
                        }
                        Err(e) => self.toast(format!("{e:#}"), ToastKind::Err),
                    }
                    close = true;
                }
            }
            Dialog::UpdateFound(u) => {
                let u = u.clone();
                let mut go = false;
                let modal = egui::Modal::new(egui::Id::new("update")).show(ctx, |ui| {
                    ui.set_width(400.0);
                    ui::dialog_title(
                        ui,
                        &pal,
                        rtl,
                        lang.t("يوجد تحديث جديد", "An update is available"),
                    );
                    ui.label(
                        RichText::new(format!(
                            "{} {} ← {}",
                            lang.t("الإصدار", "Version"),
                            update::current_version(),
                            u.version
                        ))
                        .size(12.5)
                        .color(pal.accent),
                    );
                    if !u.notes.trim().is_empty() {
                        ui.add_space(8.0);
                        egui::ScrollArea::vertical().max_height(170.0).show(ui, |ui| {
                            ui.label(
                                RichText::new(update::plain_notes(&u.notes))
                                    .size(11.5)
                                    .color(pal.muted),
                            );
                        });
                    }
                    ui.add_space(12.0);
                    let l = if rtl {
                        Layout::right_to_left(Align::Center)
                    } else {
                        Layout::left_to_right(Align::Center)
                    };
                    ui.with_layout(l, |ui| {
                        if ui::solid_button(
                            ui,
                            &pal,
                            lang.t("حدّث الآن", "Update now"),
                            pal.accent_deep,
                        )
                        .clicked()
                        {
                            go = true;
                        }
                        if ui::ghost_button(ui, &pal, lang.t("لاحقًا", "Later")).clicked() {
                            close = true;
                        }
                    });
                });
                if modal.should_close() {
                    close = true;
                }
                if go {
                    self.update_rx = Some(update::spawn_install(u));
                    close = true;
                }
            }
        }

        if close {
            self.dialog = Dialog::None;
            self.find.clear();
        }
        if let Some((plat, id, live)) = found {
            self.go_to_hit(plat, id, live);
        }
        if let Some(pw) = export_pw {
            let lang = self.settings.lang;
            if let Some(dest) = rfd::FileDialog::new()
                .add_filter(lang.t("ملف نقل بديل", "badeel transfer file"), &["badeel"])
                .set_title(lang.t("وين تحفظ ملف النقل؟", "Where should the transfer file go?"))
                .set_file_name("badeel-accounts.badeel")
                .save_file()
            {
                self.start_transfer(false, dest, pw);
            }
        }
        if let Some((file, pw)) = import_job {
            self.start_transfer(true, file, pw);
        }
    }
}

/// سجلّ حسابات منصّة كما يجب أن يظهر: المحفوظ، ومعه ما تحفظه المنصّة نفسها
/// في قائمتها ولم يدخل السجلّ بعد، ويُكتب السجلّ إن زاد فيه شيء
///
/// تستعمله صفحة المنصّة والبحث الشامل معًا، فلا يرى أحدهما غير ما يراه الآخر
fn accounts_of(p: &'static Platform) -> store::Index {
    let mut index = store::load(p.id);
    let mut fresh = false;
    match p.identity {
        catalog::Identity::Steam => {
            for a in steam::accounts().unwrap_or_default() {
                if index.get(&a.id64).is_some() {
                    continue;
                }
                let name = if a.persona.is_empty() {
                    a.login.clone()
                } else {
                    a.persona.clone()
                };
                index.upsert(store::Account {
                    id: a.id64,
                    name,
                    note: a.login,
                    avatar: None,
                    last_used: 0,
                    uses: 0,
                });
                fresh = true;
            }
            for acc in index.accounts.iter_mut() {
                if acc.avatar.is_none() {
                    acc.avatar =
                        steam::avatar_path(&acc.id).map(|p| p.to_string_lossy().to_string());
                    fresh |= acc.avatar.is_some();
                }
            }
        }
        catalog::Identity::Bnet => {
            for email in bnet::accounts() {
                if index.get(&email).is_some() {
                    continue;
                }
                let name = email.split('@').next().unwrap_or(&email).to_string();
                index.upsert(store::Account {
                    id: email.clone(),
                    name,
                    note: email,
                    avatar: None,
                    last_used: 0,
                    uses: 0,
                });
                fresh = true;
            }
        }
        _ => {}
    }
    if fresh {
        let _ = store::save(p.id, &index);
    }
    index
}

/// سطر في البحث الشامل: حساب ومنصّته، وهل هو المسجَّل دخوله الآن فيها
struct Hit {
    plat: &'static Platform,
    acc: store::Account,
    live: bool,
}

/// كم سطرًا يعرض البحث، فلا تطول النافذة على الشاشات القصيرة
const FIND_ROWS: usize = 6;

/// ما يطابق البحث مرتّبًا: من يبدأ اسمه بما كُتب أوّلًا ثم من يحتويه في اسمه
/// أو ملاحظته أو اسم منصّته بأيّ اللغتين، وبين المتساويين الأحدث استخدامًا،
/// والبحث الفارغ يعرض أحدث الحسابات استخدامًا في كل المنصّات
fn rank<'a>(hits: &'a [Hit], query: &str) -> Vec<&'a Hit> {
    let q = query.trim().to_lowercase();
    let mut out: Vec<(u8, &Hit)> = hits
        .iter()
        .filter_map(|h| {
            if q.is_empty() {
                return Some((1, h));
            }
            let name = h.acc.name.to_lowercase();
            if name.starts_with(&q) {
                return Some((0, h));
            }
            let hay = [
                name.as_str(),
                &h.acc.note.to_lowercase(),
                h.plat.name_ar,
                &h.plat.name_en.to_lowercase(),
                h.plat.id,
            ]
            .join(" ");
            hay.contains(&q).then_some((1, h))
        })
        .collect();
    out.sort_by_key(|(tier, h)| (*tier, std::cmp::Reverse(h.acc.last_used)));
    out.into_iter().map(|(_, h)| h).take(FIND_ROWS).collect()
}

fn mask(text: &str, on: bool) -> String {
    if !on || text.is_empty() {
        return text.to_string();
    }
    let n = text.chars().count();
    let keep = if n > 6 { 3 } else { 1 };
    let dots: String = std::iter::repeat_n('\u{2022}', n.saturating_sub(keep).min(10)).collect();
    let tail: String = text.chars().skip(n - keep).collect();
    format!("{dots}{tail}")
}

fn ago(lang: Lang, when: u64) -> String {
    if when == 0 {
        return lang.t("لم يُستخدم بعد", "not yet").to_string();
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let d = now.saturating_sub(when);
    let rtl = lang.rtl();
    if d < 120 {
        return lang.t("قبل قليل", "moments ago").to_string();
    }
    let (n, unit) = if d < 3600 {
        (d / 60, if rtl { "دقيقة" } else { "min" })
    } else if d < 86_400 {
        (d / 3600, if rtl { "ساعة" } else { "h" })
    } else if d < 2_592_000 {
        (d / 86_400, if rtl { "يوم" } else { "d" })
    } else {
        (d / 2_592_000, if rtl { "شهر" } else { "mo" })
    };
    if rtl {
        format!("قبل {n} {unit}")
    } else {
        format!("{n}{unit} ago")
    }
}

pub fn platform_color(id: &str) -> Color32 {
    let rgb = Color32::from_rgb;
    match id {
        "steam" => rgb(120, 200, 246),
        "battlenet" => rgb(64, 160, 255),
        "riot" => rgb(235, 85, 85),
        "epic" => rgb(222, 222, 232),
        "ubisoft" => rgb(108, 190, 255),
        "rockstar" => rgb(249, 175, 30),
        "gog" => rgb(180, 120, 245),
        _ => rgb(150, 130, 255),
    }
}

#[cfg(test)]
mod find_tests {
    use super::*;

    fn hit(pid: &str, name: &str, last: u64) -> Hit {
        Hit {
            plat: &catalog::PLATFORMS[catalog::index_of(pid)],
            acc: store::Account {
                id: format!("{pid}:{name}"),
                name: name.into(),
                note: String::new(),
                avatar: None,
                last_used: last,
                uses: 0,
            },
            live: false,
        }
    }

    fn names(v: Vec<&Hit>) -> Vec<String> {
        v.iter().map(|h| h.acc.name.clone()).collect()
    }

    #[test]
    fn a_name_that_starts_with_the_query_comes_before_one_that_only_contains_it() {
        let hits = vec![hit("steam", "xryan", 50), hit("riot", "ryan", 10), hit("epic", "other", 99)];
        assert_eq!(names(rank(&hits, "RY")), vec!["ryan", "xryan"]);
    }

    #[test]
    fn a_platform_name_in_either_language_finds_its_accounts() {
        let hits = vec![hit("battlenet", "a", 1), hit("steam", "b", 2)];
        assert_eq!(names(rank(&hits, "باتل")), vec!["a"]);
        assert_eq!(names(rank(&hits, "battle")), vec!["a"]);
    }

    #[test]
    fn an_empty_query_lists_the_most_recently_used_first_and_stays_short() {
        let hits: Vec<Hit> = (0..10).map(|i| hit("steam", &format!("n{i}"), i)).collect();
        let r = rank(&hits, "  ");
        assert_eq!(r.len(), FIND_ROWS);
        assert_eq!(r[0].acc.name, "n9");
    }
}
