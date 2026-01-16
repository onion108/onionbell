mod serde_helpers;

use self::serde_helpers::{default_volume, validate_volume};
use regex::Regex;
use serde::Deserialize;
use std::path::PathBuf;

/// The config of onionbell contains a `sound` key and several rules.
/// Read each field's documentation for more information.
#[derive(Debug, Deserialize, Default)]
pub struct Config {
    /// `sound` is an optional key, represents path to an audio file that will be played when
    /// the `bell` event is triggered. When this key is not present, no sound will play at all.
    pub sound: Option<PathBuf>,

    /// The volume of the sound, ranges from 0.0 to 1.0.
    /// The default value is 1.0.
    #[serde(default = "default_volume", deserialize_with = "validate_volume")]
    pub volume: f32,

    /// Rules to match before using the global `sound` key as the audio file to play.
    ///
    /// Rules are checked in order, and the first match will be used.
    #[serde(default, alias = "rule")]
    pub rules: Vec<Rule>,
}

/// A rule that matches against properties of the window who sends the `bell` event (we'll call it
/// the *source window* afterwards).
#[derive(Debug, Deserialize, Default)]
pub struct Rule {
    /// `sound` is an optional key, represents path to an audio file that will be played when the
    /// `bell` event is triggered and the current rule matches. When this key is not present, no
    /// sound will play at all when the rule matches the source window, even if the global `sound`
    /// key is present.
    pub sound: Option<PathBuf>,

    /// The volume of the sound, ranges from 0.0 to 1.0.
    /// The default value is 1.0.
    #[serde(default = "default_volume", deserialize_with = "validate_volume")]
    pub volume: f32,

    /// The workspace that the source window lives in.
    pub workspace: Option<WorkspaceRule>,

    /// Whether the source window is floating.
    pub floating: Option<bool>,

    /// A regular expression to match with the `class` property of the source window.
    #[serde(with = "serde_regex")]
    #[serde(default)]
    pub class_regex: Option<Regex>,

    /// A regular expression to match with the `title` property of the source window.
    #[serde(with = "serde_regex")]
    #[serde(default)]
    pub title_regex: Option<Regex>,

    /// Whether the source window is an XWayland window.
    pub xwayland: Option<bool>,
}

/// The type of `workspace` key in the rule.
/// This key is an untagged enum. When `workspace` is a number, it will be matched against the
/// `workspace.id` property of the source window. When it is a string, `workspace.name` will be
/// checked instead.
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum WorkspaceRule {
    /// `id` of the workspace.
    Id(i32),

    /// `name` of the workspace.
    Name(String),
}

impl Config {
    pub fn from_source(source: &str) -> Result<Config, toml::de::Error> {
        toml::from_str(source)
    }
}

#[allow(unused)]
mod test {
    use super::*;

    #[test]
    fn invalid_volume() {
        let error = Config::from_source("volume = 100.0");
        assert!(error.is_err());

        let error = error.unwrap_err();
        assert_eq!(
            error.message(),
            "invalid value: floating point `100.0`, expected volume must be between 0.0 and 1.0"
        );
        assert_eq!(error.span(), Some(9..14));

        let error = Config::from_source(
            r#"
            [[rule]]
            volume = 100.0
            "#,
        );
        assert!(error.is_err());

        let error = error.unwrap_err();
        assert_eq!(
            error.message(),
            "invalid value: floating point `100.0`, expected volume must be between 0.0 and 1.0"
        );
        assert_eq!(error.span(), Some(43..48));
    }

    #[test]
    fn test_empty_source() {
        let config = Config::from_source("").unwrap();
        assert!(config.sound.is_none());
        assert!(config.rules.is_empty());
    }
    #[test]
    fn test_toplevel_fields() {
        let config = Config::from_source(
            "sound = \"/home/onion27/Music/2-14. 渦状銀河のシンフォニエッタ.mp3\"",
        )
        .unwrap();
        assert_eq!(
            config.sound,
            Some(PathBuf::from(
                "/home/onion27/Music/2-14. 渦状銀河のシンフォニエッタ.mp3"
            ))
        );
        assert_eq!(config.volume, 1.0);
        assert!(config.rules.is_empty());
    }
    #[test]
    fn test_rules() {
        let config = Config::from_source(
            r#"
            sound = "/home/onion27/Music/2-14. 渦状銀河のシンフォニエッタ.mp3"
            volume = 0.95

            [[rule]]
            sound = "/home/onion27/Music/Apollo TJ.hangneil.mp3"
            workspace = 3
            floating = false

            [[rule]]
            sound = "/home/onion27/Music/maimai でらっくす躯樹の墓守 隣の庭は青い(庭師Aoi)210(木)登場.mp3"
            volume = 0.8
            workspace = "foo"
            class_regex = "^QQ.*$"
            title_regex = "^abc\\..*$"
            xwayland = false
            "#,
        )
        .unwrap();
        assert_eq!(
            config.sound,
            Some(PathBuf::from(
                "/home/onion27/Music/2-14. 渦状銀河のシンフォニエッタ.mp3"
            ))
        );
        assert_eq!(config.volume, 0.95);
        assert_eq!(config.rules.len(), 2);

        assert_eq!(
            config.rules[0].sound,
            Some(PathBuf::from("/home/onion27/Music/Apollo TJ.hangneil.mp3"))
        );
        assert_eq!(config.rules[0].volume, 1.0);
        assert_eq!(config.rules[0].workspace, Some(WorkspaceRule::Id(3)));
        assert_eq!(config.rules[0].floating, Some(false));
        assert!(config.rules[0].class_regex.is_none());
        assert!(config.rules[0].title_regex.is_none());
        assert!(config.rules[0].xwayland.is_none());

        assert_eq!(
            config.rules[1].sound,
            Some(PathBuf::from(
                "/home/onion27/Music/maimai でらっくす躯樹の墓守 隣の庭は青い(庭師Aoi)210(木)登場.mp3"
            ))
        );
        assert_eq!(config.rules[1].volume, 0.8);
        assert_eq!(
            config.rules[1].workspace,
            Some(WorkspaceRule::Name("foo".into()))
        );
        assert_eq!(config.rules[1].floating, None);
        assert!(
            config.rules[1]
                .class_regex
                .as_ref()
                .unwrap()
                .is_match("QQalskjhslk")
        );
        assert!(config.rules[1].class_regex.as_ref().unwrap().is_match("QQ"));
        assert!(config.rules[1].class_regex.as_ref().unwrap().is_match("QQ"));
        assert!(
            !config.rules[1]
                .class_regex
                .as_ref()
                .unwrap()
                .is_match("aQQ")
        );
        assert!(
            !config.rules[1]
                .class_regex
                .as_ref()
                .unwrap()
                .is_match("completely irrelevent thing")
        );
        assert!(
            !config.rules[1]
                .class_regex
                .as_ref()
                .unwrap()
                .is_match("SDEZ 1.60")
        );
        assert!(
            config.rules[1]
                .title_regex
                .as_ref()
                .unwrap()
                .is_match("abc.")
        );
        assert!(
            config.rules[1]
                .title_regex
                .as_ref()
                .unwrap()
                .is_match("abc.asl")
        );
        assert!(
            !config.rules[1]
                .title_regex
                .as_ref()
                .unwrap()
                .is_match("abc")
        );
        assert!(
            !config.rules[1]
                .title_regex
                .as_ref()
                .unwrap()
                .is_match("aaabc.aslaa")
        );
        assert_eq!(config.rules[1].xwayland, Some(false));
    }
}
