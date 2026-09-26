#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Lang {
    Ar,
    En,
}

impl Lang {
    pub fn rtl(self) -> bool {
        self == Lang::Ar
    }

    pub fn toggled(self) -> Self {
        match self {
            Lang::Ar => Lang::En,
            Lang::En => Lang::Ar,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Lang::Ar => "EN",
            Lang::En => "عربي",
        }
    }

    pub fn t(self, ar: &'static str, en: &'static str) -> &'static str {
        match self {
            Lang::Ar => ar,
            Lang::En => en,
        }
    }
}

#[macro_export]
macro_rules! tr {
    ($lang:expr, $ar:expr, $en:expr) => {
        $lang.t($ar, $en)
    };
}
