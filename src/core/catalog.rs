#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Close {
    Graceful,
    Force,
}

#[derive(Clone, Copy, Debug)]
pub enum Item {
    File(&'static str),
    Dir(&'static str),
    Reg(&'static str, &'static str),
}

#[derive(Clone, Copy, Debug)]
pub enum Identity {
    Marker(&'static str),
    Steam,
    Bnet,
}

impl Identity {
    /// المنصّة نفسها تحفظ قائمة حساباتها، فيقرأها بديل ولا ينسخ لها ملفات
    pub fn own_list(&self) -> bool {
        matches!(self, Identity::Steam | Identity::Bnet)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Locator {
    Path(&'static str),
    RegExe {
        key: &'static str,
        value: &'static str,
    },
    RegDir {
        key: &'static str,
        value: &'static str,
        exe: &'static str,
    },
    JsonExe {
        file: &'static str,
        key: &'static str,
    },
}

/// لعبة تُعرف بملفّها، ليسمّيها بديل للمستخدم بدل أن يقول اسم العملية
pub struct Game {
    pub exe: &'static str,
    pub ar: &'static str,
    pub en: &'static str,
}

impl Game {
    pub fn name(&self, arabic: bool) -> &'static str {
        if arabic { self.ar } else { self.en }
    }
}

pub struct Platform {
    pub id: &'static str,
    pub name_ar: &'static str,
    pub name_en: &'static str,
    pub exes: &'static [&'static str],
    pub close: Close,
    pub locate: &'static [Locator],
    pub launch_args: &'static [&'static str],
    pub items: &'static [Item],
    pub identity: Identity,
    /// ألعاب هذه المنصّة، يفحصها بديل قبل التبديل فلا يقطع مباراة جارية
    pub games: &'static [Game],
}

impl Platform {
    pub fn name(&self, arabic: bool) -> &'static str {
        if arabic { self.name_ar } else { self.name_en }
    }
}

pub const PLATFORMS: &[Platform] = &[
    Platform {
        id: "steam",
        name_ar: "ستيم",
        name_en: "Steam",
        exes: &["steam.exe", "steamwebhelper.exe"],
        close: Close::Graceful,
        locate: &[
            Locator::RegDir {
                key: r"HKCU\Software\Valve\Steam",
                value: "SteamPath",
                exe: "steam.exe",
            },
            Locator::Path(r"%ProgramFiles(x86)%\Steam\steam.exe"),
            Locator::Path(r"%ProgramFiles%\Steam\steam.exe"),
        ],
        launch_args: &[],
        items: &[],
        identity: Identity::Steam,
        games: &[
            Game { exe: "cs2.exe", ar: "كاونتر سترايك 2", en: "Counter-Strike 2" },
            Game { exe: "dota2.exe", ar: "دوتا 2", en: "Dota 2" },
            Game { exe: "r5apex.exe", ar: "أيبكس ليجندز", en: "Apex Legends" },
            Game { exe: "r5apex_dx12.exe", ar: "أيبكس ليجندز", en: "Apex Legends" },
            Game { exe: "RustClient.exe", ar: "رست", en: "Rust" },
            Game { exe: "TslGame.exe", ar: "ببجي", en: "PUBG" },
        ],
    },
    Platform {
        id: "battlenet",
        name_ar: "باتل نت",
        name_en: "Battle.net",
        exes: &["Battle.net.exe", "Battle.net Helper.exe"],
        close: Close::Force,
        locate: &[
            Locator::RegExe {
                key: r"HKLM\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\Battle.net",
                value: "DisplayIcon",
            },
            Locator::RegDir {
                key: r"HKLM\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\Battle.net",
                value: "InstallLocation",
                exe: "Battle.net.exe",
            },
            Locator::Path(r"%ProgramFiles(x86)%\Battle.net\Battle.net.exe"),
            Locator::Path(r"%ProgramFiles%\Battle.net\Battle.net.exe"),
        ],
        launch_args: &[],
        items: &[],
        identity: Identity::Bnet,
        games: &[
            Game { exe: "Overwatch.exe", ar: "أوفرواتش", en: "Overwatch" },
            Game { exe: "Diablo IV.exe", ar: "ديابلو 4", en: "Diablo IV" },
            Game { exe: "Wow.exe", ar: "وورلد أوف ووركرافت", en: "World of Warcraft" },
            Game { exe: "SC2_x64.exe", ar: "ستاركرافت 2", en: "StarCraft II" },
            Game { exe: "Hearthstone.exe", ar: "هيرثستون", en: "Hearthstone" },
        ],
    },
    Platform {
        id: "riot",
        name_ar: "رايوت",
        name_en: "Riot Games",
        exes: &[
            "RiotClientServices.exe",
            "RiotClientUx.exe",
            "RiotClientUxRender.exe",
            "LeagueClient.exe",
            "VALORANT.exe",
        ],
        close: Close::Force,
        locate: &[
            Locator::JsonExe {
                file: r"%ProgramData%\Riot Games\RiotClientInstalls.json",
                key: "rc_default",
            },
            Locator::JsonExe {
                file: r"%ProgramData%\Riot Games\RiotClientInstalls.json",
                key: "rc_live",
            },
            Locator::Path(r"%ProgramFiles%\Riot Games\Riot Client\RiotClientServices.exe"),
            Locator::Path(r"C:\Riot Games\Riot Client\RiotClientServices.exe"),
        ],
        launch_args: &[],
        items: &[
            Item::File(r"%LOCALAPPDATA%\Riot Games\Riot Client\Data\RiotGamesPrivateSettings.yaml"),
            Item::File(r"%LOCALAPPDATA%\Riot Games\Riot Client\Data\RiotClientPrivateSettings.yaml"),
            Item::Dir(r"%LOCALAPPDATA%\Riot Games\Riot Client\Data\Sessions"),
            Item::Dir(r"%LOCALAPPDATA%\Riot Games\Riot Client\Data\Cookies"),
        ],
        identity: Identity::Marker(r"%LOCALAPPDATA%\Riot Games\Riot Client\Data\.badeel-id"),
        games: &[
            Game { exe: "VALORANT.exe", ar: "فالورانت", en: "VALORANT" },
            Game { exe: "VALORANT-Win64-Shipping.exe", ar: "فالورانت", en: "VALORANT" },
            Game { exe: "League of Legends.exe", ar: "ليق أوف ليجندز", en: "League of Legends" },
            Game { exe: "LoR.exe", ar: "ليجندز أوف رونتيرا", en: "Legends of Runeterra" },
        ],
    },
    Platform {
        id: "epic",
        name_ar: "إيبك",
        name_en: "Epic Games",
        exes: &["EpicGamesLauncher.exe", "EpicWebHelper.exe"],
        close: Close::Graceful,
        locate: &[
            Locator::Path(
                r"%ProgramFiles(x86)%\Epic Games\Launcher\Portal\Binaries\Win64\EpicGamesLauncher.exe",
            ),
            Locator::Path(
                r"%ProgramFiles%\Epic Games\Launcher\Portal\Binaries\Win64\EpicGamesLauncher.exe",
            ),
        ],
        launch_args: &[],
        items: &[Item::File(
            r"%LOCALAPPDATA%\EpicGamesLauncher\Saved\Config\Windows\GameUserSettings.ini",
        )],
        identity: Identity::Marker(
            r"%LOCALAPPDATA%\EpicGamesLauncher\Saved\Config\Windows\.badeel-id",
        ),
        games: &[
            Game { exe: "FortniteClient-Win64-Shipping.exe", ar: "فورتنايت", en: "Fortnite" },
            Game { exe: "RocketLeague.exe", ar: "روكيت ليق", en: "Rocket League" },
        ],
    },
    Platform {
        id: "ubisoft",
        name_ar: "يوبيسوفت",
        name_en: "Ubisoft Connect",
        exes: &[
            "UbisoftConnect.exe",
            "UbisoftGameLauncher.exe",
            "UbisoftGameLauncher64.exe",
            "UbisoftExtension.exe",
            "upc.exe",
            "UplayService.exe",
            "UplayWebCore.exe",
            "UplayCrashReporter.exe",
        ],
        close: Close::Force,
        locate: &[
            Locator::RegDir {
                key: r"HKCU\Software\Ubisoft\Launcher",
                value: "InstallDir",
                exe: "upc.exe",
            },
            Locator::RegExe {
                key: r"HKLM\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\Uplay",
                value: "DisplayIcon",
            },
            Locator::Path(r"%ProgramFiles(x86)%\Ubisoft\Ubisoft Game Launcher\upc.exe"),
            Locator::Path(r"%ProgramFiles%\Ubisoft\Ubisoft Game Launcher\upc.exe"),
        ],
        launch_args: &[],
        items: &[
            Item::File(r"%LOCALAPPDATA%\Ubisoft Game Launcher\user.dat"),
            Item::File(r"%LOCALAPPDATA%\Ubisoft Game Launcher\ConnectSecureStorage.dat"),
            Item::File(r"%LOCALAPPDATA%\Ubisoft Game Launcher\settings.yaml"),
        ],
        identity: Identity::Marker(r"%LOCALAPPDATA%\Ubisoft Game Launcher\.badeel-id"),
        games: &[
            Game { exe: "RainbowSix.exe", ar: "رينبو سكس سيج", en: "Rainbow Six Siege" },
            Game { exe: "ACValhalla.exe", ar: "أساسنز كريد فالهالا", en: "Assassin's Creed Valhalla" },
            Game { exe: "FarCry6.exe", ar: "فار كراي 6", en: "Far Cry 6" },
        ],
    },
    Platform {
        id: "rockstar",
        name_ar: "روكستار",
        name_en: "Rockstar",
        exes: &[
            "Launcher.exe",
            "LauncherPatcher.exe",
            "RockstarService.exe",
            "SocialClubHelper.exe",
            "RockstarErrorHandler.exe",
        ],
        close: Close::Force,
        locate: &[
            Locator::RegDir {
                key: r"HKLM\SOFTWARE\WOW6432Node\Rockstar Games\Launcher",
                value: "InstallFolder",
                exe: "Launcher.exe",
            },
            Locator::Path(r"%ProgramFiles%\Rockstar Games\Launcher\Launcher.exe"),
            Locator::Path(r"%ProgramFiles(x86)%\Rockstar Games\Launcher\Launcher.exe"),
        ],
        launch_args: &[],
        items: &[
            Item::File(r"%LOCALAPPDATA%\Rockstar Games\Launcher\settings_user.dat"),
            Item::File(r"%ProgramData%\Rockstar Games\Launcher\settings_machine.dat"),
            Item::File(r"%LOCALAPPDATA%\Rockstar Games\Launcher\CrashLogs\settings.dat"),
        ],
        identity: Identity::Marker(r"%LOCALAPPDATA%\Rockstar Games\Launcher\.badeel-id"),
        games: &[
            Game { exe: "GTA5.exe", ar: "جي تي ايه 5", en: "GTA V" },
            Game { exe: "GTA5_Enhanced.exe", ar: "جي تي ايه 5", en: "GTA V" },
            Game { exe: "RDR2.exe", ar: "ريد ديد ريدمبشن 2", en: "Red Dead Redemption 2" },
        ],
    },
    Platform {
        id: "gog",
        name_ar: "جوج جالاكسي",
        name_en: "GOG Galaxy",
        exes: &[
            "GalaxyClient.exe",
            "GalaxyClientService.exe",
            "GalaxyCommunication.exe",
            "GOG Galaxy Notifications Renderer.exe",
        ],
        close: Close::Force,
        locate: &[
            Locator::RegDir {
                key: r"HKLM\SOFTWARE\WOW6432Node\GOG.com\GalaxyClient\paths",
                value: "client",
                exe: "GalaxyClient.exe",
            },
            Locator::Path(r"%ProgramFiles(x86)%\GOG Galaxy\GalaxyClient.exe"),
            Locator::Path(r"%ProgramFiles%\GOG Galaxy\GalaxyClient.exe"),
        ],
        launch_args: &[],
        items: &[
            Item::File(r"%LOCALAPPDATA%\GOG.com\Galaxy\Configuration\config.json"),
            Item::Reg(r"HKCU\Software\GOG.com\Galaxy", "refreshToken"),
            Item::Reg(r"HKCU\Software\GOG.com\Galaxy\settings", "userId"),
            Item::Reg(r"HKCU\Software\GOG.com\Galaxy\settings", "username"),
        ],
        identity: Identity::Marker(r"%LOCALAPPDATA%\GOG.com\Galaxy\Configuration\.badeel-id"),
        games: &[
            Game { exe: "Cyberpunk2077.exe", ar: "سايبربانك 2077", en: "Cyberpunk 2077" },
            Game { exe: "witcher3.exe", ar: "ذا ويتشر 3", en: "The Witcher 3" },
        ],
    },
];

pub fn index_of(id: &str) -> usize {
    PLATFORMS.iter().position(|p| p.id == id).unwrap_or(0)
}

impl Item {
    pub fn raw_path(&self) -> Option<&'static str> {
        match self {
            Item::File(p) | Item::Dir(p) => Some(p),
            Item::Reg(..) => None,
        }
    }

    pub fn slot(&self, index: usize) -> String {
        match self {
            Item::File(p) | Item::Dir(p) => {
                let leaf = p.rsplit(['\\', '/']).next().unwrap_or("item");
                format!("{index:02}-{}", super::paths::sanitize(leaf))
            }
            Item::Reg(key, value) => format!(
                "{index:02}-reg-{}",
                super::paths::sanitize(&format!("{key}-{value}"))
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_platform_has_a_way_to_launch_and_identify() {
        for p in PLATFORMS {
            assert!(!p.locate.is_empty(), "{}", p.id);
            assert!(!p.exes.is_empty(), "{}", p.id);
            if !p.identity.own_list() {
                assert!(!p.items.is_empty(), "{}", p.id);
            }
        }
    }

    #[test]
    fn slots_are_unique_within_a_platform() {
        for p in PLATFORMS {
            let slots: Vec<String> = p.items.iter().enumerate().map(|(i, it)| it.slot(i)).collect();
            let mut sorted = slots.clone();
            sorted.sort();
            sorted.dedup();
            assert_eq!(sorted.len(), slots.len(), "{}", p.id);
        }
    }
}
