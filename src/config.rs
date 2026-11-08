//! OpenStream TV configuration. `OPENSTREAM_*` is the primary environment
//! prefix; historical `JIOTV_*` names remain accepted for compatibility.

use serde::Deserialize;
use std::env;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub epg: bool,
    pub debug: bool,
    pub disable_ts_handler: bool,
    pub disable_logout: bool,
    pub drm: bool,
    /// `tv_plus` is the documented config key. `extras` (the previous name)
    /// and `tvplus` are still accepted so existing config files keep working.
    #[serde(rename = "tv_plus", alias = "extras", alias = "tvplus")]
    pub extras: bool,
    /// Enables the optional Fastway Live TV provider. It still needs
    /// `ost fastway login` (or the Settings card) before channels appear.
    pub fastway: bool,
    /// Optional PlayboxTV account, backed by the reverse-engineered
    /// `playboxtv.tv.android.in` client (see `docs/ott-providers/playbox.md`).
    /// Off by default.
    #[serde(default)]
    pub playbox: bool,
    /// Experimental Tata Play Smart diagnostic provider. Off by default.
    #[serde(default)]
    pub tataplay: bool,
    /// VZY (Dish/Watcho white-label) free tier. Off by default.
    #[serde(default)]
    pub vzy: bool,
    /// Airtel Xstream Play live TV, driven by the `tv.accedo.airtel.wynk`
    /// client (see `docs/ott-provider-airtel/index.md`). It needs an Airtel
    /// OTP login before channels appear. Off by default.
    #[serde(default)]
    pub airtel: bool,
    pub disable_auth: bool,
    pub title: String,
    pub disable_url_encryption: bool,
    pub proxy: String,
    /// HTTPS origin published by a known reverse proxy or named tunnel.
    pub public_origin: String,
    pub path_prefix: String,
    pub log_path: String,
    #[serde(default = "default_true")]
    pub log_to_stdout: bool,
    pub custom_channels_file: String,
    pub default_categories: Vec<i64>,
    pub default_languages: Vec<i64>,
}

fn default_true() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Config {
            epg: false,
            debug: false,
            disable_ts_handler: false,
            disable_logout: false,
            drm: true,
            extras: false,
            fastway: false,
            playbox: false,
            tataplay: false,
            vzy: false,
            airtel: false,
            disable_auth: false,
            title: "OpenStream TV".to_string(),
            disable_url_encryption: false,
            proxy: String::new(),
            public_origin: String::new(),
            path_prefix: String::new(),
            log_path: String::new(),
            log_to_stdout: true,
            custom_channels_file: String::new(),
            default_categories: Vec::new(),
            default_languages: Vec::new(),
        }
    }
}

impl Config {
    /// Loads config from `filename` if given, else the first common config
    /// file in the current directory, else environment variables only.
    pub fn load(filename: Option<&str>) -> anyhow::Result<Config> {
        let mut cfg = Config::default();
        let path = filename
            .map(str::to_string)
            .filter(|s| !s.is_empty())
            .or_else(common_file);

        if let Some(path) = path {
            let text = std::fs::read_to_string(&path)
                .map_err(|e| anyhow::anyhow!("reading config file {path}: {e}"))?;
            cfg = toml::from_str(&text)
                .map_err(|e| anyhow::anyhow!("parsing config file {path}: {e}"))?;
        }
        cfg.apply_env();
        if !cfg.public_origin.is_empty() {
            cfg.public_origin = crate::public_origin::validate(&cfg.public_origin)?;
        }
        Ok(cfg)
    }

    fn apply_env(&mut self) {
        macro_rules! env_bool {
            ($field:ident, $legacy:literal, $name:literal) => {
                if let Ok(v) = env::var($legacy) {
                    if let Ok(b) = v.parse::<bool>() {
                        self.$field = b;
                    }
                }
                if let Ok(v) = env::var($name) {
                    if let Ok(b) = v.parse::<bool>() {
                        self.$field = b;
                    }
                }
            };
        }
        macro_rules! env_str {
            ($field:ident, $legacy:literal, $name:literal) => {
                if let Ok(v) = env::var($legacy) {
                    self.$field = v;
                }
                if let Ok(v) = env::var($name) {
                    self.$field = v;
                }
            };
        }
        env_bool!(epg, "JIOTV_EPG", "OPENSTREAM_EPG");
        env_bool!(debug, "JIOTV_DEBUG", "OPENSTREAM_DEBUG");
        env_bool!(
            disable_ts_handler,
            "JIOTV_DISABLE_TS_HANDLER",
            "OPENSTREAM_DISABLE_TS_HANDLER"
        );
        env_bool!(
            disable_logout,
            "JIOTV_DISABLE_LOGOUT",
            "OPENSTREAM_DISABLE_LOGOUT"
        );
        env_bool!(drm, "JIOTV_DRM", "OPENSTREAM_DRM");
        if let Ok(v) = env::var("JIOTV_EXTRAS") {
            if let Ok(b) = v.parse::<bool>() {
                self.extras = b;
            }
        }
        env_bool!(extras, "JIOTV_TVPLUS", "OPENSTREAM_TVPLUS");
        env_bool!(fastway, "JIOTV_FASTWAY", "OPENSTREAM_FASTWAY");
        env_bool!(playbox, "JIOTV_PLAYBOX", "OPENSTREAM_PLAYBOX");
        env_bool!(tataplay, "JIOTV_TATAPLAY", "OPENSTREAM_TATAPLAY");
        env_bool!(vzy, "JIOTV_VZY", "OPENSTREAM_VZY");
        env_bool!(airtel, "JIOTV_AIRTEL", "OPENSTREAM_AIRTEL");
        env_bool!(
            disable_auth,
            "JIOTV_DISABLE_AUTH",
            "OPENSTREAM_DISABLE_AUTH"
        );
        env_str!(title, "JIOTV_TITLE", "OPENSTREAM_TITLE");
        env_bool!(
            disable_url_encryption,
            "JIOTV_DISABLE_URL_ENCRYPTION",
            "OPENSTREAM_DISABLE_URL_ENCRYPTION"
        );
        env_str!(proxy, "JIOTV_PROXY", "OPENSTREAM_PROXY");
        env_str!(
            public_origin,
            "JIOTV_PUBLIC_ORIGIN",
            "OPENSTREAM_PUBLIC_ORIGIN"
        );
        env_str!(path_prefix, "JIOTV_PATH_PREFIX", "OPENSTREAM_PATH_PREFIX");
        env_str!(log_path, "JIOTV_LOG_PATH", "OPENSTREAM_LOG_PATH");
        env_bool!(
            log_to_stdout,
            "JIOTV_LOG_TO_STDOUT",
            "OPENSTREAM_LOG_TO_STDOUT"
        );
        env_str!(
            custom_channels_file,
            "JIOTV_CUSTOM_CHANNELS_FILE",
            "OPENSTREAM_CUSTOM_CHANNELS_FILE"
        );
        if let Ok(v) = env::var("JIOTV_DEFAULT_CATEGORIES") {
            self.default_categories = parse_int_list(&v);
        }
        if let Ok(v) = env::var("OPENSTREAM_DEFAULT_CATEGORIES") {
            self.default_categories = parse_int_list(&v);
        }
        if let Ok(v) = env::var("JIOTV_DEFAULT_LANGUAGES") {
            self.default_languages = parse_int_list(&v);
        }
        if let Ok(v) = env::var("OPENSTREAM_DEFAULT_LANGUAGES") {
            self.default_languages = parse_int_list(&v);
        }
    }
}

fn parse_int_list(v: &str) -> Vec<i64> {
    v.split(',')
        .filter_map(|s| s.trim().parse::<i64>().ok())
        .collect()
}

fn common_file() -> Option<String> {
    const NAMES: &[&str] = &[
        "openstream.toml",
        "openstream.yml",
        "openstream.yaml",
        "openstream.json",
        "jiotv_go.yml",
        "jiotv_go.yaml",
        "jiotv_go.toml",
        "jiotv_go.json",
        "config.json",
        "config.yml",
        "config.toml",
        "config.yaml",
    ];
    NAMES
        .iter()
        .find(|n| Path::new(n).exists())
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // env::set_var is process-global; serialize tests that touch it.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn default_drm_is_true() {
        assert!(Config::default().drm);
    }

    #[test]
    fn disable_logout_defaults_false_and_respects_explicit_toml() {
        assert!(!toml::from_str::<Config>("").unwrap().disable_logout);
        assert!(
            !toml::from_str::<Config>("disable_logout = false\n")
                .unwrap()
                .disable_logout
        );
        assert!(
            toml::from_str::<Config>("disable_logout = true\n")
                .unwrap()
                .disable_logout
        );
    }

    #[test]
    fn env_overrides_defaults() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("OPENSTREAM_TITLE", "Test Title");
        std::env::set_var("OPENSTREAM_DRM", "false");
        let cfg = Config::load(None).unwrap();
        assert_eq!(cfg.title, "Test Title");
        assert!(!cfg.drm);
        std::env::remove_var("OPENSTREAM_TITLE");
        std::env::remove_var("OPENSTREAM_DRM");
    }

    #[test]
    fn documented_env_var_turns_tv_plus_on() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("OPENSTREAM_TVPLUS", "true");
        let cfg = Config::load(None).unwrap();
        assert!(cfg.extras);
        std::env::remove_var("OPENSTREAM_TVPLUS");
    }

    #[test]
    fn deprecated_env_var_name_still_works() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("JIOTV_EXTRAS", "true");
        let cfg = Config::load(None).unwrap();
        assert!(cfg.extras);
        std::env::remove_var("JIOTV_EXTRAS");
    }

    #[test]
    fn openstream_env_var_wins_over_legacy() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("OPENSTREAM_TVPLUS", "true");
        std::env::set_var("JIOTV_TVPLUS", "false");
        let cfg = Config::load(None).unwrap();
        assert!(cfg.extras);
        std::env::remove_var("OPENSTREAM_TVPLUS");
        std::env::remove_var("JIOTV_TVPLUS");
    }

    #[test]
    fn playbox_is_off_by_default_and_can_be_enabled() {
        let _guard = ENV_LOCK.lock().unwrap();
        assert!(!Config::default().playbox);
        std::env::set_var("JIOTV_PLAYBOX", "true");
        let cfg = Config::load(None).unwrap();
        assert!(cfg.playbox);
        std::env::remove_var("JIOTV_PLAYBOX");
    }

    #[test]
    fn vzy_is_off_by_default_and_can_be_enabled() {
        let _guard = ENV_LOCK.lock().unwrap();
        assert!(!Config::default().vzy);
        std::env::set_var("JIOTV_VZY", "true");
        let cfg = Config::load(None).unwrap();
        assert!(cfg.vzy);
        std::env::remove_var("JIOTV_VZY");
    }

    #[test]
    fn toml_vzy_key_is_read() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("jiotv_go.toml");
        std::fs::write(&path, "vzy = true\n").unwrap();
        assert!(Config::load(Some(path.to_str().unwrap())).unwrap().vzy);
    }

    #[test]
    fn airtel_is_off_by_default_and_can_be_enabled() {
        let _guard = ENV_LOCK.lock().unwrap();
        assert!(!Config::default().airtel);
        std::env::set_var("JIOTV_AIRTEL", "true");
        let cfg = Config::load(None).unwrap();
        assert!(cfg.airtel);
        std::env::remove_var("JIOTV_AIRTEL");
    }

    #[test]
    fn toml_airtel_key_is_read() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("jiotv_go.toml");
        std::fs::write(&path, "airtel = true\n").unwrap();
        assert!(Config::load(Some(path.to_str().unwrap())).unwrap().airtel);
    }

    #[test]
    fn toml_playbox_key_is_read() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("jiotv_go.toml");
        std::fs::write(&path, "playbox = true\n").unwrap();
        assert!(Config::load(Some(path.to_str().unwrap())).unwrap().playbox);
    }

    #[test]
    fn tataplay_is_off_by_default_and_can_be_enabled() {
        let _guard = ENV_LOCK.lock().unwrap();
        assert!(!Config::default().tataplay);
        std::env::set_var("JIOTV_TATAPLAY", "true");
        let cfg = Config::load(None).unwrap();
        assert!(cfg.tataplay);
        std::env::remove_var("JIOTV_TATAPLAY");
    }

    #[test]
    fn toml_tataplay_key_is_read() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("jiotv_go.toml");
        std::fs::write(&path, "tataplay = true\n").unwrap();
        assert!(Config::load(Some(path.to_str().unwrap())).unwrap().tataplay);
    }

    #[test]
    fn removed_vzyone_switch_is_not_configuration() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("JIOTV_VZYONE", "true");
        let cfg = Config::load(None).unwrap();
        assert!(!cfg.playbox);
        assert!(!cfg.tataplay);
        std::env::remove_var("JIOTV_VZYONE");
    }

    #[test]
    fn public_origin_is_validated_from_env_and_toml() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("jiotv_go.toml");
        std::fs::write(&path, "public_origin = \"https://tv.example.com:443/\"\n").unwrap();
        assert_eq!(
            Config::load(Some(path.to_str().unwrap()))
                .unwrap()
                .public_origin,
            "https://tv.example.com"
        );
        std::env::set_var("JIOTV_PUBLIC_ORIGIN", "https://legacy.example:8443");
        std::env::set_var("OPENSTREAM_PUBLIC_ORIGIN", "https://other.example:8443");
        assert_eq!(
            Config::load(Some(path.to_str().unwrap()))
                .unwrap()
                .public_origin,
            "https://other.example:8443"
        );
        std::env::set_var("OPENSTREAM_PUBLIC_ORIGIN", "http://unsafe.example");
        assert!(Config::load(Some(path.to_str().unwrap())).is_err());
        std::env::remove_var("OPENSTREAM_PUBLIC_ORIGIN");
        std::env::remove_var("JIOTV_PUBLIC_ORIGIN");
    }

    #[test]
    fn documented_toml_key_is_parsed() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("jiotv_go.toml");
        std::fs::write(&path, "tv_plus = true\n").unwrap();
        let cfg = Config::load(Some(path.to_str().unwrap())).unwrap();
        assert!(cfg.extras);
    }

    #[test]
    fn deprecated_toml_key_names_still_work() {
        let _guard = ENV_LOCK.lock().unwrap();
        for key in ["extras", "tvplus"] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("jiotv_go.toml");
            std::fs::write(&path, format!("{key} = true\n")).unwrap();
            let cfg = Config::load(Some(path.to_str().unwrap())).unwrap();
            assert!(cfg.extras, "{key} should still enable the TV+ source");
        }
    }

    #[test]
    fn toml_file_is_parsed() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("jiotv_go.toml");
        std::fs::write(&path, "title = \"From File\"\nepg = true\n").unwrap();
        let cfg = Config::load(Some(path.to_str().unwrap())).unwrap();
        assert_eq!(cfg.title, "From File");
        assert!(cfg.epg);
    }
}
