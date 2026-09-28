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
            "مؤسس بديل والمهندس الذي بناه سطرًا سطرًا، المحرّك الذرّي وطبقة التشفير ونظام الاسترجاع الذي لا يترك ملفًا في غير مكانه والواجهة بكل بكسل فيها، لا يشحن كودًا لا يفهمه ولا يستعير حلًّا لا يقدر أن يدافع عنه، سُرق حسابه مرّة فرفض أن يتكرّر ذلك لأحد غيره، وبنى مع مؤيد البرنامج الذي كان يتمنّى وجوده ذلك اليوم، وقاعدته الوحيدة أن ما لا تستطيع قراءته لا تستطيع ائتمانه",
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

pub fn story(rtl: bool) -> [&'static str; 17] {
    if rtl {
        [
            "لا تبدأ الحكايات الكبيرة في العادة بحدث كبير، بل بلحظة صغيرة يمرّ بها المرء غافلًا ثم يقضي بعدها زمنًا طويلًا يعيد تأمّلها، وحكاية بديل بدأت بلحظة من هذا النوع، في ليلة عادية لا يميّزها شيء، جلست فيها أمام الشاشة كما أفعل كل ليلة، وكتبت كلمة السر كما كتبتها ألف مرة، فجاءني الرد بأنها غير صحيحة",
            "ظننته أول الأمر خطأً في الاتصال، فأعدت المحاولة مرة ومرتين، ثم طلبت استعادة الحساب فإذا البريد المرتبط به لم يعد بريدي، وفي تلك الدقائق القليلة أدركت أن سنوات كاملة من اللعب والمشتريات والصداقات قد انتقلت بصمت إلى يد لا أعرفها، وأن ذلك كله جرى في أقل من عشر دقائق ودون أن أشعر بشيء",
            "وقضيت الأسبوع التالي بين تذاكر الدعم، رسائل تُغلق من تلقاء نفسها، وردود مكتوبة لكل الناس ولا تخاطب أحدًا منهم، وانتظار يطول ثم ينتهي إلى لا شيء، وفي كل مرة أغلقت فيها نافذة الدعم كان سؤال واحد يعود إليّ بإلحاح أشدّ من سابقه، من أين دخلوا؟",
            "ولم يكن الجواب بعيدًا كما ظننت، فقبل ذلك بأسابيع كنت أستعمل أداة صغيرة لتبديل الحسابات، من تلك الأدوات الشائعة التي يفتحها آلاف اللاعبين كل يوم مطمئنّين، أداة مجانية سريعة تعفيك من عناء الخروج والدخول في كل مرة، ولهذا بالذات انتشرت، فالناس قلّما يسألون عمّا يريحهم",
            "وهنا تبيّنت لي حقيقة كنت أعرفها ولا ألتفت إليها، فهذه الأدوات لا تطلب منك كلمة السر ولا رمز التحقق، وإنما تطلب ما هو أثمن منهما جميعًا، أعني جلستك المفتوحة، ذلك الملف الصغير الذي يقول للمنصّة إنك أنت، فمن ملكه لم يعد بحاجة إلى كلمة سرّك أصلًا، لأنه صار في نظر المنصّة أنت",
            "ثم تحفظ هذه الأدوات ذلك الملف في مكان لا تراه، وبصيغة لا تقرؤها، وتفعل به ما لا تعلمه، وما دام كودها مغلقًا فليس في وسع أحد أن يثبت ما يجري لملفاتك بعد أن تسلّمها، لا أنت ولا غيرك، فأنت في الحقيقة لا تبرم عقدًا مع أداة، وإنما تسلّم مفاتيح بيتك لغريب لأنه وعدك بأن يحرسه",
            "وقد علّمتني تلك التجربة أن الثقة التي لا يمكن التحقق منها ليست ثقة، بل أمل نعلّقه على نوايا الآخرين، وأن الأمان الحقيقي لا يُطلب من الناس أن يصدّقوه، وإنما يُعرض عليهم ليروه بأعينهم",
            "فبحثت عن بديل أطمئن إليه، ووجدت أدوات كثيرة، غير أن أكثرها يطلب صلاحيات المدير على الجهاز كله، وبعضها يحفظ الجلسات بلا تشفير يستحق الذكر، وأغلبها لا يعرف العربية أصلًا، ولم أجد بينها أداة واحدة أستطيع أن أفتح كودها وأقرأ بنفسي ما تفعله بملفاتي",
            "فعزمت على أن أكتبه بنفسي، لا لأنني أردت أن أصنع برنامجًا، بل لأنني أردت أن أنام مطمئنًّا وأنا أعلم أين تذهب جلستي بالضبط، وقديمًا قيل إن الحاجة أمّ الاختراع، وأحسب أن الخسارة أمّه الأخرى",
            "وأقمته على قاعدة واحدة لم أتنازل عنها في سطر من سطوره، أنك لا ينبغي أن تثق بنا، فكل شيء في بديل مفتوح مقروء، وكل ملف يحفظه يُشفَّر بمفتاح لا وجود له خارج جهازك، وكل خطوة في التبديل لها طريق رجوع إن تعثّرت، فإن أردت أن تتيقّن فاقرأ الكود، وإن أردت ما هو أبعد من ذلك فابنِ النسخة بنفسك وقارنها بالتي ننشرها",
            "ولم أسلك هذا الطريق وحدي، فمؤيد المطيري لم يأتِ في آخره ليجرّب البرنامج ويبدي رأيه فيه، بل كان معي قبل أن يكون للبرنامج اسم، منذ الليلة التي حكيت له فيها ما جرى وأنا ما زلت أغالب غضبي، فلم يقل لي انسَ الأمر ودعه، وإنما قال جملة بسيطة غيّرت مجرى كل شيء، طيب، وش نسوي؟",
            "ومن تلك الليلة صار الطريق طريقنا معًا، وأمضينا ليالي طويلة نخطّ على الورق كيف ينبغي لهذا الشيء أن يعمل قبل أن يُكتب منه سطر واحد، أمسك أنا بالمحرّك ويمسك هو بالمعمار من جهته، يسأل عن الحالة التي لم تخطر لي، ويستوقفني عند كل قرار متعجّل ليسأل، وماذا لو انقطعت الكهرباء في هذه اللحظة بالذات؟",
            "وفكرة أن تكون لكل خطوة في التبديل طريق رجوع كانت فكرته هو، فقد كنت أبني سبع خطوات تمضي إلى الأمام وحسب، فقال لي إن الخطوة التي لا تعرف كيف تعود لا تستحق أن تُكتب، فأعدنا بناء المحرّك كله على هذا الأصل، وهو اليوم أمتن ما في بديل، وأكثر ما يطمئنني حين أغمض عينيّ",
            "والوجه الذي تراه للبرنامج وجهه هو، ترتيب الشاشات، وموضع كل زر، وما يراه المستخدم في ثانيته الأولى وما لا ينبغي أن يراه أبدًا، كل ذلك خرج من بين يديه، وهو الذي أصرّ على أن تكون الواجهة عربية من اليمين إلى اليسار بحقّ لا بترجمة مقلوبة على عجل، وهو الذي ردّ عليّ عشرات التصاميم التي كنت أراها جميلة، وقال لي إنها جميلة في عينك وحدك لأنك أنت من صنعها، وكان محقًّا",
            "وهو العين التي لا يفوتها شيء، فلا يخرج إصدار من بديل قبل أن يسلكه بيده على أجهزة حقيقية ومنصّات حقيقية وحسابات حقيقية، وقد ردّ عليّ إصدارات كاملة قبل ساعات من نشرها لأن خطوة واحدة فيها كانت غامضة على من يفتح البرنامج أول مرة، وكان على حقّ في كل مرة",
            "وكنّا نختلف كثيرًا، غير أن أجمل ما في بديل وُلد من ذلك الاختلاف، لأن كل ميزة فيه مرّت على عقلين لا على عقل واحد، وهذا عندي هو الفرق بين أداة يكتبها شخص لنفسه وبرنامج يبنيه اثنان لغيرهما",
            "وبديل اليوم مفتوح المصدر كاملًا تحت رخصة GPL-3.0، ولسنا نطلب منك أن تثق بنا، وإنما نطلب منك أن تقرأ، فإن وجدت فيه ما لا يرضيك فأخبرنا، فما كُتبت هذه الأداة في الأصل إلا لأن أحدًا لم يخبرني قبل أن أخسر حسابي.",
        ]
    } else {
        [
            "Great stories seldom begin with great events. They begin with a small moment one passes through without noticing, and then spends a long time turning over afterwards. The story of badeel began with such a moment, on an ordinary night with nothing to set it apart. I sat before the screen as I did every night, typed my password as I had typed it a thousand times, and was told it was wrong.",
            "At first I took it for a connection error and tried again, once, then twice. Then I asked to recover the account, and found that the email tied to it was no longer mine. In those few minutes I understood that whole years of playing, of purchases, of friendships, had passed silently into a hand I did not know, and that all of it had happened in under ten minutes without my feeling a thing.",
            "I spent the following week among support tickets: threads that closed themselves, replies written to everyone and addressed to no one, waits that stretched on and ended in nothing. And each time I closed the support window, a single question returned to me, more insistent than the last: where did they come in?",
            "The answer was not as far away as I had thought. A few weeks earlier I had been using a small account switcher, one of those popular tools that thousands of players open every day without a second thought. Free, fast, and it spares you the trouble of signing out and back in every time, which is precisely why it spread. People seldom question what makes their lives easier.",
            "Here a truth became clear to me that I had known without ever heeding. Such tools do not ask for your password or your verification code. They ask for something worth more than both together: your open session, the small file that tells the platform you are you. Whoever holds it no longer needs your password at all, because in the platform's eyes, they have become you.",
            "These tools then keep that file somewhere you cannot see, in a format you cannot read, and do with it what you cannot know. And so long as their code stays closed, no one can prove what happens to your files once you hand them over, not you and not anyone else. You are not entering an agreement with a tool; you are handing the keys of your house to a stranger because he promised to guard it.",
            "That experience taught me that trust which cannot be verified is not trust but hope, a hope we hang on the intentions of others; and that real security is not something people should be asked to believe, but something laid before them to see with their own eyes.",
            "So I searched for an alternative I could rely on. I found many tools, yet most demanded administrator rights over the entire machine, some stored sessions with no encryption worth mentioning, and nearly all of them knew no Arabic at all. Not one let me open its code and read for myself what it did with my files.",
            "So I resolved to write it myself, not because I wished to make a program, but because I wished to sleep at peace, knowing exactly where my session goes. It has long been said that necessity is the mother of invention; I have come to believe that loss is its other mother.",
            "And I built it on a single principle I have not surrendered in any line of it: you should not have to trust us. Everything in badeel is open and readable; every file it saves is encrypted under a key that exists nowhere outside your machine; and every step of a switch has a way back if it stumbles. If you wish to be certain, read the code. If you wish for more, build it yourself and compare it with what we publish.",
            "I did not walk this road alone. Moayad Almutairi did not arrive at the end to try the program and offer his opinion; he was with me before the program had a name, from the night I told him what had happened while I was still wrestling with my anger. He did not tell me to let it go. He said one simple thing that changed the course of everything: so, what do we do?",
            "From that night the road became ours. We spent long nights sketching on paper how this thing ought to work before a single line of it was written. I held the engine, and he held the architecture from his side, asking about the case that had not occurred to me, stopping me at every hasty decision to ask: and what if the power fails at this very moment?",
            "The idea that every step of a switch must have a way back was his. I had been building seven steps that only moved forward, and he told me that a step which does not know how to return does not deserve to be written. So we rebuilt the whole engine on that foundation, and today it is the strongest part of badeel, and the thing that most lets me rest when I close my eyes.",
            "And the face you see on the program is his. The order of the screens, the place of every button, what a user sees in their first second and what they must never see, all of it came from his hands. He insisted the interface be truly Arabic, right to left by design rather than a translation flipped over in haste, and he turned down dozens of designs I thought beautiful, telling me they were beautiful only in my eyes, because I was the one who made them. He was right.",
            "He is also the eye that nothing slips past. No release leaves badeel until he has walked it through himself on real machines, real platforms and real accounts. He has sent whole builds back hours before they were due, because a single step in them was unclear to someone opening the program for the first time, and he was right every time.",
            "We disagreed often, yet the finest things in badeel were born of that disagreement, because every feature in it passed through two minds rather than one. That, to me, is the difference between a tool one person writes for himself and a program two people build for everyone else.",
            "badeel today is fully open source under GPL-3.0. We do not ask you to trust us; we ask you to read. And if you find in it something that does not please you, tell us, for this tool was written, in the end, only because no one told me before I lost my account.",
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
                        "شكرًا لكل من جرّب بديل وأرسل ملاحظة — أنتم جزء من هذه القائمة",
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
