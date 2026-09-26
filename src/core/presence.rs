//! نشاط ديسكورد.
//!
//! يتكلم بديل مع برنامج ديسكورد على نفس الجهاز عبر أنبوب محلّي، فلا
//! يخرج من هنا اتصال شبكة ولا يُرسل شيء عن حساباتك. الذي يظهر لأصدقائك
//! سطران ثابتان لا اسم حساب فيهما ولا اسم منصّة.
//!
//! والميزة مطفأة ما لم تُشغَّل من الإعدادات، وإن كان ديسكورد مغلقًا
//! فكل شيء هنا يسقط بصمت ويعاود المحاولة بعد مدّة.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use discord_rich_presence::{DiscordIpc, DiscordIpcClient, activity};

/// معرّف التطبيق في لوحة مطوّري ديسكورد.
const APP_ID: &str = "1553285643512315974";

/// مفاتيح الصور كما هي مرفوعة في Rich Presence Assets، حرفًا بحرف.
/// وصورة الغلاف في لوحة المطوّرين خانة أخرى لا مفتاح لها، فلا تصلح هنا.
const LARGE: &str = "badeel";
/// الشارة الصغيرة على ركن الكبيرة. اتركها فارغة ما لم تُرفع صورة ثانية.
const SMALL: &str = "";

const SITE: &str = "https://ryanathlawi.github.io/badeel-site/";
const CODE: &str = "https://github.com/Ryanathlawi/badeel";

/// لا نحاول الاتصال أكثر من مرة كل هذه المدّة، فديسكورد قد لا يكون مفتوحًا.
const RETRY: Duration = Duration::from_secs(20);

pub struct Presence {
    client: Option<DiscordIpcClient>,
    on: bool,
    /// بداية الجلسة، ليظهر عندهم «منذ كذا».
    since: i64,
    last_try: Option<Instant>,
    /// آخر سطرين أُرسلا، فلا نعيد إرسال نفس الشيء كل إطار.
    shown: Option<(String, String)>,
}

impl Default for Presence {
    fn default() -> Self {
        Self::new()
    }
}

impl Presence {
    pub fn new() -> Self {
        Self {
            client: None,
            on: false,
            since: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or_default(),
            last_try: None,
            shown: None,
        }
    }

    pub fn set_enabled(&mut self, on: bool) {
        if self.on == on {
            return;
        }
        self.on = on;
        if !on {
            self.drop_client();
        } else {
            self.last_try = None;
        }
    }

    fn drop_client(&mut self) {
        if let Some(mut c) = self.client.take() {
            let _ = c.clear_activity();
            let _ = c.close();
        }
        self.shown = None;
    }

    /// يوصل إن أمكن. يرجع صحيحًا لو صار عندنا عميل جاهز.
    fn ensure(&mut self) -> bool {
        if self.client.is_some() {
            return true;
        }
        if let Some(t) = self.last_try
            && t.elapsed() < RETRY
        {
            return false;
        }
        self.last_try = Some(Instant::now());
        let mut c = DiscordIpcClient::new(APP_ID);
        if c.connect().is_ok() {
            self.client = Some(c);
            true
        } else {
            false
        }
    }

    /// السطران اللذان يراهما أصدقاؤك. تكرار نفس النص لا يكلّف شيئًا.
    pub fn show(&mut self, details: &str, state: &str, site: &str, code: &str) {
        if !self.on || !self.ensure() {
            return;
        }
        if self
            .shown
            .as_ref()
            .is_some_and(|(d, s)| d == details && s == state)
        {
            return;
        }

        let payload = activity::Activity::new()
            .details(details)
            .state(state)
            .assets({
                let a = activity::Assets::new().large_image(LARGE).large_text(details);
                if SMALL.is_empty() {
                    a
                } else {
                    a.small_image(SMALL).small_text(state)
                }
            })
            .timestamps(activity::Timestamps::new().start(self.since))
            .buttons(vec![
                activity::Button::new(site, SITE),
                activity::Button::new(code, CODE),
            ]);

        match self.client.as_mut().map(|c| c.set_activity(payload)) {
            Some(Ok(())) => self.shown = Some((details.to_owned(), state.to_owned())),
            // انقطع الأنبوب أو أُغلق ديسكورد: نتخلّص من العميل ونعاود لاحقًا
            _ => {
                self.client = None;
                self.shown = None;
            }
        }
    }
}

impl Drop for Presence {
    fn drop(&mut self) {
        self.drop_client();
    }
}
