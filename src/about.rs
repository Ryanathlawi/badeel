use eframe::egui::{self, Align2, Color32, FontId, Rect, Sense, pos2, vec2};

use crate::i18n::Lang;
use crate::theme::{Palette, ease_out};
use crate::ui;

const PORTRAITS: [&[u8]; 2] = [
    include_bytes!(concat!(env!("OUT_DIR"), "/person0.png")),
    include_bytes!(concat!(env!("OUT_DIR"), "/person1.png")),
];

fn portrait(ui: &egui::Ui, who: usize) -> Option<egui::TextureId> {
    let bytes = *PORTRAITS.get(who)?;
    if bytes.is_empty() {
        return None;
    }
    let source = egui::ImageSource::Bytes {
        uri: format!("bytes://badeel-person-{who}.png").into(),
        bytes: egui::load::Bytes::Static(bytes),
    };
    let poll = source.load(
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

pub struct Person {
    pub initials: &'static str,
    pub name: (&'static str, &'static str),
    pub role: (&'static str, &'static str),
    pub bio: (&'static str, &'static str),
    pub tags: [(&'static str, &'static str); 3],
    pub hue: u8,
    pub support: &'static str,
    pub contact: &'static str,
}

pub const PEOPLE: [Person; 2] = [
    Person {
        initials: "RA",
        name: ("ريان الأثلاوي", "Ryan Athlawi"),
        role: (
            "المؤسس · هندسة النظام والتصميم",
            "Founder · systems engineering and design",
        ),
        bio: (
            "مؤسس بديل والمهندس الذي بناه سطرًا سطرًا، المحرّك الذرّي وطبقة التشفير ونظام الاسترجاع الذي لا يترك ملفًا في غير مكانه والواجهة بكل بكسل فيها، لا يشحن كودًا لا يفهمه ولا يستعير حلًّا لا يقدر أن يدافع عنه، سُرق حسابه مرّة فرفض أن يتكرّر ذلك لأحد غيره، وبنى مع مؤيد البرنامج الذي كان يتمنّى وجوده ذلك اليوم، وقاعدته الوحيدة أن ما لا تستطيع قراءته لا تستطيع ائتمانه.",
            "Founder of badeel and the engineer who built it line by line: the atomic engine, the encryption layer, the rollback system that never leaves a file out of place, and every pixel of the interface. He ships no code he does not understand and borrows no solution he cannot defend. His own account was stolen once; he refused to let that happen to anyone else, and built the program he wished had existed that day. His only rule: what you cannot read, you cannot trust.",
        ),
        tags: [
            ("المحرّك", "Engine"),
            ("التشفير", "Encryption"),
            ("التصميم", "Design"),
        ],
        hue: 0,
        support: "https://www.paypal.com/paypalme/RayanAthlawi",
        contact: "https://discord.gg/H8sq6Uc3kA",
    },
    Person {
        initials: "MA",
        name: ("مؤيد المطيري", "Moayad Almutairi"),
        role: (
            "المؤسس المشارك · المعمار والتجربة والجودة",
            "Co-founder · architecture, experience and quality",
        ),
        bio: (
            "الشريك المؤسس والعقل الثاني خلف بديل، كان في المشروع قبل أن يكون له اسم، ومنه جاءت قاعدة أن لكل خطوة في التبديل طريق رجوع وهي اليوم أقوى ما في المحرّك، ومن يده خرج ترتيب الشاشات وكل زر في مكانه والواجهة العربية من اليمين إلى اليسار بحق لا بترجمة مقلوبة، ولا تصل نسخة إلى المستخدمين قبل أن يمشي عليها بيده على أجهزة ومنصّات حقيقية، ومرجعه في كل قرار سؤال واحد، هل هذا واضح وآمن لمن يفتح البرنامج لأول مرة؟",
            "Co-founder and the second mind behind badeel, in this project before it had a name. The rule that every step of a switch must have a way back was his, and it is now the strongest thing in the engine. The order of the screens, where every button sits, and an interface that is genuinely right to left rather than a flipped translation all came out of his hands, and no build reaches users before he has walked it himself on real machines and real platforms. Every call comes back to one question: is this clear and safe for someone opening it for the first time?",
        ),
        tags: [
            ("المعمار", "Architecture"),
            ("التجربة", "Experience"),
            ("الجودة", "Quality"),
        ],
        hue: 1,
        support: "",
        contact: "",
    },
];

pub fn story(rtl: bool) -> [&'static str; 16] {
    if rtl {
        [
            "ما بدأت فكرة بديل في اجتماع ولا على ورقة بيضاء، بدأت في ليلة عادية جدًا فتحت فيها حسابي فلم أجده",
            "في البداية ظننتها مشكلة في الاتصال فأعدت المحاولة مرتين وثلاثًا، ثم جاءت الرسالة التي يعرفها كل من مرّ بهذا، كلمة السر غير صحيحة، وحين طلبت استعادتها اكتشفت أن البريد المرتبط بالحساب لم يعد بريدي، سنوات من اللعب ومن المشتريات ومن الأصدقاء انتقلت كلها إلى شخص لا أعرف عنه شيئًا وفي أقل من عشر دقائق",
            "قضيت الأسبوع الذي بعده في تذاكر الدعم، رسائل تُغلق تلقائيًا وردود جاهزة كُتبت للجميع ولم تُكتب لأحد، وانتظار طويل ينتهي بلا شيء، ومع كل رسالة كنت أعيد على نفسي السؤال نفسه، من أين دخلوا؟",
            "والجواب كان أمامي طوال الوقت، قبل ذلك بأسابيع كنت أستخدم أداة صغيرة لتبديل الحسابات، من تلك الأدوات المنتشرة التي يفتحها آلاف اللاعبين كل يوم وهم مطمئنون، مجانية وسريعة وتوفّر عليك عناء تسجيل الخروج والدخول في كل مرة، وهذا بالضبط سبب انتشارها",
            "وهنا فهمت المشكلة الحقيقية، هذه الأدوات لا تطلب منك كلمة سر ولا رمز تحقق، بل تطلب شيئًا أثمن من الاثنين معًا وهو جلستك المفتوحة، ذلك الملف الصغير الذي يقول للمنصّة إنك أنت، ثم تحفظه في مكان لا تراه وبصيغة لا تقرأها وتفعل به ما لا تعرفه",
            "وما دام الكود مغلقًا فلا أحد يستطيع أن يثبت ماذا يجري لتلك الملفات بعد أن تسلّمها، لا أنت ولا أي شخص آخر، أنت لا توقّع على شيء، بل تسلّم مفاتيح بيتك لأحدهم لأنه قال لك إنه سيحرسها",
            "بحثت وقتها عن بديل أثق به، وجدت أدوات كثيرة لكن أغلبها يطلب صلاحيات المدير على الجهاز كله، وبعضها يحفظ الجلسات بلا تشفير يُذكر، وأكثرها لا يعرف العربية أصلًا، ولم أجد واحدًا أستطيع أن أفتح كوده وأقرأ بنفسي ماذا يفعل بملفاتي",
            "فقرّرت أن أكتبه، لا لأنني أردت أن أبني برنامجًا، بل لأنني أردت أن أنام مرتاحًا وأنا أعرف أين تذهب جلستي بالضبط",
            "وبنيته على قاعدة واحدة لم أتنازل عنها في أي سطر، أنت لا يجب أن تثق بنا، كل شيء في بديل مفتوح ومقروء، وكل ملف تحفظه يُشفّر بمفتاح لا يوجد خارج جهازك، وكل خطوة في التبديل لها طريق رجوع لو تعثّرت، فإن أردت أن تتأكد فافتح الكود، وإن أردت أكثر من ذلك فابنِ النسخة بنفسك وقارنها بالتي ننشرها",
            "ولم أمشِ هذا الطريق وحدي، ومؤيد المطيري لم يأتِ في آخره ليجرّب البرنامج ويقول رأيه فيه، مؤيد كان معي قبل أن يكون للبرنامج اسم، من الليلة التي حكيت له فيها ما صار وأنا ما زلت غاضبًا، فما قال لي خلّها وانسها، قال لي طيب وش نسوي",
            "ومن تلك الليلة صار الطريق طريقنا نحن الاثنين، وجلسنا ليالي طويلة نرسم على الورق كيف يشتغل هذا الشيء قبل أن يُكتب منه سطر واحد، أنا أمسك المحرّك وهو يمسك المعمار من الجهة الثانية، يسأل عن الحالة التي لم تخطر لي، ويوقفني عند كل قرار سريع ليقول لي طيب وش يصير لو انقطعت الكهرباء هنا بالضبط",
            "وفكرة أن لكل خطوة في التبديل طريق رجوع كانت فكرته هو، أنا كنت أبني سبع خطوات تمشي للأمام فقط، وهو الذي قال إن الخطوة التي لا تعرف كيف ترجع لا تستحق أن تُكتب، فأعدنا بناء المحرّك كله على هذا الأساس، وهو اليوم أقوى ما في بديل وأكثر ما أنام مرتاحًا بسببه",
            "والوجه الذي تراه للبرنامج وجهه هو، ترتيب الشاشات وأين يقع كل زر وماذا يرى المستخدم في أول ثانية وماذا لا يجب أن يراه أبدًا، كل هذا خرج من يده، وهو الذي أصرّ أن تكون الواجهة عربية من اليمين إلى اليسار بحق لا بترجمة مقلوبة على عجل، وهو الذي ردّ عليّ عشرات التصاميم التي كنت أراها جميلة وقال لي إنها جميلة في عيني أنا وحدي لأنني أنا الذي بنيتها",
            "وهو العين التي لا يمر من أمامها شيء، فلا يخرج إصدار من بديل قبل أن يمشي عليه بيده على أجهزة حقيقية ومنصّات حقيقية وحسابات حقيقية، وقد ردّ عليّ إصدارات كاملة قبل ساعات من نشرها لأن خطوة واحدة فيها كانت غامضة على من يفتح البرنامج لأول مرة، وكان على حق في كل مرة",
            "نختلف كثيرًا وأحسن ما في بديل خرج من ذلك الاختلاف، لأن كل ميزة فيه مرّت على رأسين لا على رأس واحد، وهذا هو الفرق بين أداة كتبها شخص لنفسه وبين برنامج بناه اثنان لغيرهما",
            "بديل اليوم مفتوح المصدر بالكامل تحت رخصة GPL-3.0، لا نطلب منك أن تثق بنا، نطلب منك أن تقرأ، وإن وجدت فيه ما لا يعجبك فأخبرنا، فهذه الأداة كُتبت أصلًا لأن أحدًا لم يخبرني قبل أن أخسر حسابي.",
        ]
    } else {
        [
            "The idea for badeel did not start in a meeting, and it did not start on a blank page. It started on a very ordinary night, when I opened my account and it was not there.",
            "At first I thought it was the connection, so I tried again, twice, three times. Then came the line that anyone who has been through this knows by heart: incorrect password. When I asked to recover it, I found that the email on the account was no longer my email. Years of playing, of purchases, of friends, all of it handed to someone I know nothing about, in under ten minutes.",
            "I spent the week that followed inside support tickets. Threads that closed themselves, replies written for everyone and for nobody, and long waits that ended with nothing. And with every message I kept asking myself the same question: where did they come in from?",
            "The answer had been in front of me the whole time. A few weeks earlier I had been using a small account switcher, one of those widely used tools that thousands of players open every day feeling perfectly safe. Free, fast, and it saves you from signing out and back in every single time, which is exactly why it spread.",
            "That is when I understood the real problem. A tool like that never asks you for a password or a verification code. It asks for something worth more than both of them together: your open session, the small file that tells the platform you are you. Then it keeps that file somewhere you cannot see, in a format you cannot read, and does things with it that you never find out about.",
            "And as long as the source stays closed, nobody can prove what happens to those files once you have handed them over. Not you, and not anyone else. You are not signing an agreement, you are giving the keys to your house to someone because he told you he would look after them.",
            "So I went looking for an alternative I could trust. There were plenty of tools. Most of them wanted administrator rights over the whole machine, some stored sessions with no encryption worth the name, and almost none of them spoke Arabic at all. Not one of them let me open its source and read, for myself, what it was doing with my files.",
            "So I decided to write it. Not because I wanted to build a program, but because I wanted to sleep at night knowing exactly where my session goes.",
            "And I built it on one rule I have not given up on in a single line: you should not have to trust us. Everything in badeel is open and readable, every file it saves is encrypted with a key that exists nowhere outside your own machine, and every step of a switch has a way back if it stumbles. If you want to be certain, read the source, and if you want more than that, build it yourself and compare your build with the one we publish.",
            "And I did not walk this road alone. Moayad Almutairi did not turn up at the end of it to try the program and offer an opinion. Moayad was with me before the program had a name, from the night I told him what had happened while I was still angry about it. He did not tell me to let it go. He asked me what we were going to do about it.",
            "From that night the road became ours, both of us. We spent long nights drawing on paper how this thing should work before a single line of it existed. I would hold the engine and he would hold the architecture from the other side, asking about the case that had never occurred to me, stopping me at every quick decision to ask what happens if the power cuts out right here.",
            "The idea that every step of a switch must have a way back was his. I was building seven steps that only walked forward, and he was the one who said that a step which does not know how to come back does not deserve to be written. So we rebuilt the entire engine on that principle, and today it is the strongest thing in badeel and the part that lets me sleep.",
            "And the face you see on the program is his face. The order of the screens, where every button sits, what a user sees in the first second and what they must never see, all of that came out of his hands. He is the one who insisted the interface be genuinely Arabic, right to left by design and not a translation flipped over in a hurry, and he is the one who sent back dozens of designs I thought were beautiful and told me they were beautiful in my eyes alone, because I was the one who had built them.",
            "He is also the eye nothing gets past. No release leaves badeel before he has walked through it himself on real machines, real platforms and real accounts. He has sent whole builds back hours before they were due to go out because one step in them was unclear to someone opening the program for the first time, and he was right every single time.",
            "We disagree often, and the best of badeel came out of that disagreement, because every feature in it passed through two heads instead of one. That is the difference between a tool somebody wrote for himself and a program two people built for everyone else.",
            "badeel today is fully open source under the GPL-3.0 licence. We are not asking you to trust us, we are asking you to read. And if you find something in it you do not like, tell us, because this tool was written in the first place because nobody told me before I lost my account.",
        ]
    }
}

fn hue(pal: &Palette, k: u8) -> Color32 {
    match k {
        0 => pal.accent,
        _ => pal.accent_2,
    }
}

fn wrapped(
    p: &egui::Painter,
    anchor: egui::Pos2,
    rtl: bool,
    text: &str,
    size: f32,
    color: Color32,
    width: f32,
) -> f32 {
    let mut job =
        egui::text::LayoutJob::simple(text.to_owned(), FontId::proportional(size), color, width);
    job.halign = if rtl {
        egui::Align::RIGHT
    } else {
        egui::Align::LEFT
    };
    let galley = p.layout_job(job);
    let h = galley.size().y;
    p.galley(anchor, galley, color);
    h
}

fn person_card(
    ui: &mut egui::Ui,
    rect: Rect,
    pal: &Palette,
    lang: Lang,
    who: usize,
    t: f32,
    alpha: f32,
) -> Option<usize> {
    let rtl = lang.rtl();
    let person = &PEOPLE[who];
    let col = hue(pal, person.hue);
    ui::glass_card(ui.painter(), rect, pal, col, alpha);

    let pad = 20.0;
    let x = if rtl {
        rect.right() - pad
    } else {
        rect.left() + pad
    };
    let mono_r = 27.0;
    let mono_c = pos2(
        if rtl {
            rect.right() - pad - mono_r
        } else {
            rect.left() + pad + mono_r
        },
        rect.top() + pad + mono_r,
    );
    let face = portrait(ui, who);
    {
        let p = ui.painter();
        ui::monogram(p, mono_c, mono_r, pal, person.initials, face, col, t, alpha);

        let tx = if rtl {
            mono_c.x - mono_r - 14.0
        } else {
            mono_c.x + mono_r + 14.0
        };
        let anchor = if rtl {
            Align2::RIGHT_CENTER
        } else {
            Align2::LEFT_CENTER
        };
        p.text(
            pos2(tx, mono_c.y - 11.0),
            anchor,
            lang.t(person.name.0, person.name.1),
            FontId::proportional(16.5),
            pal.text.gamma_multiply(alpha),
        );
        p.text(
            pos2(tx, mono_c.y + 12.0),
            anchor,
            lang.t(person.role.0, person.role.1),
            FontId::proportional(11.0),
            col.gamma_multiply(0.95 * alpha),
        );

        let line_y = mono_c.y + mono_r + 16.0;
        p.line_segment(
            [
                pos2(rect.left() + pad, line_y),
                pos2(rect.right() - pad, line_y),
            ],
            egui::Stroke::new(1.0, pal.line.gamma_multiply(alpha)),
        );

        wrapped(
            p,
            pos2(x, line_y + 14.0),
            rtl,
            lang.t(person.bio.0, person.bio.1),
            11.8,
            pal.muted.gamma_multiply(alpha),
            rect.width() - pad * 2.0,
        );
    }

    let tag_y = rect.bottom() - pad - 76.0;
    let p = ui.painter();
    let mut tx = x;
    for tag in person.tags {
        let label = lang.t(tag.0, tag.1);
        let g = p.layout_no_wrap(label.to_owned(), FontId::proportional(9.5), col);
        let w = g.size().x + 20.0;
        let r = Rect::from_min_size(
            pos2(if rtl { tx - w } else { tx }, tag_y),
            vec2(w, 21.0),
        );
        p.rect_filled(r, 999.0, col.gamma_multiply(0.13 * alpha));
        p.text(
            r.center(),
            Align2::CENTER_CENTER,
            label,
            FontId::proportional(9.5),
            col.gamma_multiply(0.95 * alpha),
        );
        tx += if rtl { -(w + 8.0) } else { w + 8.0 };
    }

    let mut hit = None;
    let row = Rect::from_min_size(
        pos2(rect.left() + pad, rect.bottom() - pad - 42.0),
        vec2(rect.width() - pad * 2.0, 40.0),
    );
    ui.scope_builder(egui::UiBuilder::new().max_rect(row), |ui| {
        let l = if rtl {
            egui::Layout::right_to_left(egui::Align::Center)
        } else {
            egui::Layout::left_to_right(egui::Align::Center)
        };
        ui.with_layout(l, |ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            if ui::ghost_button_sized(
                ui,
                pal,
                lang.t("ادعم", "Support"),
                vec2(96.0, 36.0),
            )
            .clicked()
            {
                hit = Some(0);
            }
            if ui::ghost_button_sized(
                ui,
                pal,
                lang.t("تواصل", "Contact"),
                vec2(96.0, 36.0),
            )
            .clicked()
            {
                hit = Some(1);
            }
        });
    });
    hit
}

pub fn page(
    ui: &mut egui::Ui,
    area: Rect,
    pal: &Palette,
    lang: Lang,
    version: &str,
    t: f32,
    entry: f32,
    alpha: f32,
) -> Option<(usize, usize)> {
    let rtl = lang.rtl();
    let mut hit = None;
    let e = ease_out(entry);
    let shifted = Rect::from_min_size(
        pos2(area.left(), area.top() + (1.0 - e) * 20.0),
        area.size(),
    );

    ui.scope_builder(egui::UiBuilder::new().max_rect(shifted), |ui| {
        ui.set_opacity(alpha);
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .id_salt("about")
            .show(ui, |ui| {
                let full = ui.available_width();
                let w = full.min(940.0);
                let pad = ((full - w) * 0.5).max(0.0) + 14.0;
                let w = full - pad * 2.0;
                let band = |ui: &mut egui::Ui, h: f32| {
                    let (r, _) = ui.allocate_exact_size(vec2(full, h), Sense::hover());
                    Rect::from_min_size(pos2(r.left() + pad, r.top()), vec2(w, h))
                };

                let paras = story(rtl);
                let body_w = w - 44.0;
                let two_col = body_w >= 700.0;
                let gap = 30.0;
                let col_w = if two_col { (body_w - gap) * 0.5 } else { body_w };
                let mut heights: Vec<f32> = Vec::with_capacity(paras.len());
                {
                    let p = ui.painter();
                    for para in paras {
                        let job = egui::text::LayoutJob::simple(
                            para.to_owned(),
                            FontId::proportional(12.5),
                            pal.muted,
                            col_w,
                        );
                        heights.push(p.layout_job(job).size().y);
                    }
                }
                let run = |from: usize, to: usize| -> f32 {
                    heights[from..to].iter().map(|h| h + 11.0).sum()
                };
                let split = if two_col {
                    let half = run(0, heights.len()) * 0.5;
                    let mut acc = 0.0;
                    let mut at = heights.len();
                    for (i, h) in heights.iter().enumerate() {
                        acc += h + 11.0;
                        if acc >= half {
                            at = i + 1;
                            break;
                        }
                    }
                    at
                } else {
                    heights.len()
                };
                let story_h = 66.0 + run(0, split).max(run(split, heights.len()));
                let card = band(ui, story_h + 10.0);
                let p = ui.painter();
                ui::glass_card(
                    p,
                    Rect::from_min_size(card.min, vec2(card.width(), story_h)),
                    pal,
                    pal.accent,
                    1.0,
                );
                let hx = if rtl {
                    card.right() - 22.0
                } else {
                    card.left() + 22.0
                };
                let anchor = if rtl {
                    Align2::RIGHT_CENTER
                } else {
                    Align2::LEFT_CENTER
                };
                p.circle_filled(
                    pos2(if rtl { hx - 4.0 } else { hx + 4.0 }, card.top() + 28.0),
                    4.0,
                    pal.accent,
                );
                p.text(
                    pos2(if rtl { hx - 16.0 } else { hx + 16.0 }, card.top() + 28.0),
                    anchor,
                    lang.t("قصة بديل", "The story of badeel"),
                    FontId::proportional(17.0),
                    pal.text,
                );
                let step = if rtl { -(col_w + gap) } else { col_w + gap };
                let top = card.top() + 52.0;
                let mut y = top;
                let mut x = hx;
                for (i, para) in paras.into_iter().enumerate() {
                    if i == split {
                        x = hx + step;
                        y = top;
                    }
                    let h = wrapped(p, pos2(x, y), rtl, para, 12.5, pal.muted, col_w);
                    y += h + 11.0;
                }

                ui.add_space(16.0);
                let lab = band(ui, 24.0);
                ui::band_label(
                    ui.painter(),
                    lab,
                    pal,
                    rtl,
                    lang.t("من بنى بديل", "WHO BUILT BADEEL"),
                );
                ui.add_space(10.0);

                let two = w >= 620.0;
                let cw = if two { (w - 14.0) * 0.5 } else { w };
                let mut bio_h: f32 = 0.0;
                {
                    let p = ui.painter();
                    for person in &PEOPLE {
                        let job = egui::text::LayoutJob::simple(
                            lang.t(person.bio.0, person.bio.1).to_owned(),
                            FontId::proportional(11.8),
                            pal.muted,
                            cw - 40.0,
                        );
                        bio_h = bio_h.max(p.layout_job(job).size().y);
                    }
                }
                let ch = (bio_h + 217.0).max(244.0);
                let rows = if two { 1.0 } else { 2.0 };
                let grid = band(ui, ch * rows + if two { 0.0 } else { 14.0 });
                for i in 0..PEOPLE.len() {
                    let cell = if two {
                        let x = if rtl {
                            grid.right() - cw - i as f32 * (cw + 14.0)
                        } else {
                            grid.left() + i as f32 * (cw + 14.0)
                        };
                        Rect::from_min_size(pos2(x, grid.top()), vec2(cw, ch))
                    } else {
                        Rect::from_min_size(
                            pos2(grid.left(), grid.top() + i as f32 * (ch + 14.0)),
                            vec2(cw, ch),
                        )
                    };
                    if let Some(a) = person_card(ui, cell, pal, lang, i, t, 1.0) {
                        hit = Some((i, a));
                    }
                }

                ui.add_space(16.0);
                let meta = band(ui, 58.0);
                let p = ui.painter();
                ui::glass_card(p, meta, pal, pal.accent_2, 1.0);
                let cells = [
                    (lang.t("الإصدار", "Version"), version.to_string()),
                    (lang.t("الرخصة", "Licence"), "GPL-3.0".to_string()),
                    (
                        lang.t("المصدر", "Source"),
                        "github.com/Ryanathlawi/badeel".to_string(),
                    ),
                ];
                let cellw = meta.width() / cells.len() as f32;
                for (i, (label, value)) in cells.into_iter().enumerate() {
                    let r = Rect::from_min_size(
                        pos2(meta.left() + cellw * i as f32, meta.top()),
                        vec2(cellw, meta.height()),
                    );
                    p.text(
                        pos2(r.center().x, r.top() + 20.0),
                        Align2::CENTER_CENTER,
                        label,
                        FontId::proportional(9.5),
                        pal.faint,
                    );
                    p.text(
                        pos2(r.center().x, r.bottom() - 20.0),
                        Align2::CENTER_CENTER,
                        value,
                        FontId::proportional(11.5),
                        pal.text,
                    );
                    if i > 0 {
                        p.line_segment(
                            [
                                pos2(r.left(), r.top() + 14.0),
                                pos2(r.left(), r.bottom() - 14.0),
                            ],
                            egui::Stroke::new(1.0, pal.line),
                        );
                    }
                }

                ui.add_space(10.0);
                let foot = band(ui, 30.0);
                ui.painter().text(
                    foot.center(),
                    Align2::CENTER_CENTER,
                    lang.t(
                        "شكرًا لكل من جرّب بديل وأرسل ملاحظة — أنتم جزء من هذه القائمة.",
                        "Thank you to everyone who tried badeel and sent a note - you are part of this list too.",
                    ),
                    FontId::proportional(10.5),
                    pal.faint,
                );
                ui.add_space(8.0);
            });
    });

    hit
}
