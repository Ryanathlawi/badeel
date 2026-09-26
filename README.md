# بديل · badeel

مبدّل حسابات الألعاب لويندوز، يبدّل بين حساباتك في سبع منصّات بضغطة واحدة وبدون كتابة كلمة سر، وكل جلسة محفوظة مشفّرة ومربوطة بجهازك وحده.

A Windows game account switcher: move between your accounts on seven platforms in one click, with no password typing, every session stored encrypted and bound to your machine alone.

**الموقع · Website** — https://ryanathlawi.github.io/badeel-site/

---

## المنصّات · Platforms

ستيم، باتل نت، رايوت، إيبك، يوبيسوفت، روكستار، جوج جالاكسي

Steam · Battle.net · Riot Games · Epic Games · Ubisoft Connect · Rockstar · GOG Galaxy

## الأمان · Security

- تشفير `AES-256-GCM` لكل ملف جلسة
- اشتقاق كلمة السر الاختيارية بـ `Argon2id` بأربعة وستين ميجابايت من الذاكرة وثلاث جولات
- المفتاح مربوط بحساب ويندوز وبالجهاز عبر `DPAPI`، فالنسخة المنقولة إلى جهاز آخر لا تُفتح
- كل خطوة في التبديل لها طريق رجوع، فإن تعثّرت خطوة رجع كل ملف إلى مكانه
- بلا خوادم وبلا حسابات وبلا تتبّع، والاتصال الوحيد هو فحص التحديثات على GitHub ويمكن إيقافه

بديل لا يطلب كلمة سر المنصّة ولا يقرأها ولا يخزّنها، وإنما ينقل ملفات الجلسة نفسها التي أنشأتها المنصّة.

## البناء · Build

```bash
cargo build --release
```

الناتج ملف واحد في `target/release/badeel.exe`، بلا مثبِّت وبلا صلاحيات مدير.

## الفريق · Team

- **ريان الأثلاوي** — المؤسس، هندسة النظام والتصميم
- **مؤيد المطيري** — المؤسس المشارك، المعمار والتجربة والجودة

## الرخصة · Licence

GPL-3.0 — انظر [LICENSE](LICENSE)
