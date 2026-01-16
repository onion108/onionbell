use std::io::Read;
use std::io::Write;
use std::{os::unix::net::UnixStream, path::Path};

use log::trace;
use serde::Deserialize;

use crate::config::Rule;
use crate::config::WorkspaceRule;
use crate::error::AppError;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HyprClient {
    pub address: String,
    pub mapped: bool,
    pub hidden: bool,
    pub at: [i32; 2],
    pub size: [i32; 2],
    pub workspace: HyprWorkspace,
    pub floating: bool,
    pub pseudo: bool,
    pub monitor: i32,
    pub class: String,
    pub title: String,
    pub initial_class: String,
    pub initial_title: String,
    pub pid: i32,
    pub xwayland: bool,
    pub pinned: bool,
    pub fullscreen: i32,
    pub fullscreen_client: i32,
    pub grouped: Vec<String>,
    // tags: [],
    pub swallowing: String,
    #[serde(rename = "focusHistoryID")]
    pub focus_history_id: i32,
    pub inhibiting_idle: bool,
    pub xdg_tag: String,
    pub xdg_description: String,
    pub content_type: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct HyprWorkspace {
    pub id: i32,
    pub name: String,
}

impl HyprClient {
    pub fn get_clients<P: AsRef<Path>>(socket: P) -> Result<Vec<HyprClient>, AppError> {
        let response;
        {
            let mut socket = UnixStream::connect(socket)?;
            write!(socket, "-j/clients")?;

            let mut buf = String::new();
            socket.read_to_string(&mut buf)?;
            response = buf;
        }
        Ok(serde_json::from_str(&response)?)
    }

    pub fn match_rule(clients: &[HyprClient], data: &str, rule: &Rule) -> bool {
        let mut client = None;
        for c in clients {
            if !c.address.starts_with("0x") {
                trace!("Invalid address: {}", c.address);
                continue;
            }
            if &c.address[2..] == data {
                client = Some(c);
                break;
            }
        }
        let Some(client) = client else {
            trace!("client not found");
            return false;
        };

        let mut accumulator = true;
        if let Some(ref workspace) = rule.workspace {
            accumulator = accumulator
                && match workspace {
                    WorkspaceRule::Id(id) => &client.workspace.id == id,
                    WorkspaceRule::Name(name) => &client.workspace.name == name,
                };
        }
        trace!("workspace: accumulator = {accumulator}");

        if let Some(ref floating) = rule.floating {
            accumulator = accumulator && (&client.floating == floating)
        }
        trace!("floating: accumulator = {accumulator}");

        if let Some(ref xwayland) = rule.xwayland {
            accumulator = accumulator && (&client.xwayland == xwayland)
        }
        trace!("xwayland: accumulator = {accumulator}");

        if let Some(ref class_regex) = rule.class_regex {
            accumulator = accumulator && class_regex.is_match(&client.class)
        }
        trace!("class_regex: accumulator = {accumulator}");

        if let Some(ref title_regex) = rule.title_regex {
            accumulator = accumulator && title_regex.is_match(&client.title)
        }
        trace!("title_regex: accumulator = {accumulator}");
        accumulator
    }
}

#[allow(unused)]
mod test {
    use std::env;

    use regex::Regex;

    use super::*;
    #[test]
    fn test_clients_parse() {
        let client_source = r#"
        {
            "address": "0x558e928c04d0",
            "mapped": true,
            "hidden": false,
            "at": [9, 80],
            "size": [1582, 911],
            "workspace": {
                "id": 3,
                "name": "3"
            },
            "floating": false,
            "pseudo": false,
            "monitor": 0,
            "class": "QQ",
            "title": "QQ",
            "initialClass": "QQ",
            "initialTitle": "QQ",
            "pid": 296480,
            "xwayland": false,
            "pinned": false,
            "fullscreen": 0,
            "fullscreenClient": 0,
            "grouped": ["0x558e928c04d0"],
            "tags": [],
            "swallowing": "0x0",
            "focusHistoryID": 1,
            "inhibitingIdle": false,
            "xdgTag": "",
            "xdgDescription": "",
            "contentType": "none"
        }
            "#;
        let client = serde_json::from_str::<HyprClient>(client_source).unwrap();
        assert_eq!(client.address, "0x558e928c04d0");
        assert_eq!(client.mapped, true);
        assert_eq!(client.hidden, false);
        assert_eq!(client.at, [9, 80]);
        assert_eq!(client.size, [1582, 911]);
        assert_eq!(client.workspace.id, 3);
        assert_eq!(client.workspace.name, "3");
        assert_eq!(client.floating, false);
        assert_eq!(client.pseudo, false);
        assert_eq!(client.monitor, 0);
        assert_eq!(client.class, "QQ");
        assert_eq!(client.title, "QQ");
        assert_eq!(client.initial_class, "QQ");
        assert_eq!(client.initial_title, "QQ");
        assert_eq!(client.pid, 296480);
        assert_eq!(client.xwayland, false);
        assert_eq!(client.pinned, false);
        assert_eq!(client.fullscreen, 0);
        assert_eq!(client.fullscreen_client, 0);
        assert_eq!(client.grouped, vec!["0x558e928c04d0"]);
        // tags
        assert_eq!(client.swallowing, "0x0");
        assert_eq!(client.focus_history_id, 1);
        assert_eq!(client.inhibiting_idle, false);
        assert_eq!(client.xdg_tag, "");
        assert_eq!(client.xdg_description, "");
        assert_eq!(client.content_type, "none");
    }

    #[test]
    fn match_rule() {
        // {{{ Huge Data
        let data = r##"
[{
    "address": "0x558e92a1b830",
    "mapped": true,
    "hidden": false,
    "at": [9, 49],
    "size": [1582, 942],
    "workspace": {
        "id": 4,
        "name": "4"
    },
    "floating": false,
    "pseudo": false,
    "monitor": 0,
    "class": "discord",
    "title": "#annoyncements | Tsoding - Discord",
    "initialClass": "discord",
    "initialTitle": "Discord",
    "pid": 505038,
    "xwayland": false,
    "pinned": false,
    "fullscreen": 0,
    "fullscreenClient": 0,
    "grouped": [],
    "tags": [],
    "swallowing": "0x0",
    "focusHistoryID": 3,
    "inhibitingIdle": false,
    "xdgTag": "",
    "xdgDescription": "",
    "contentType": "none"
},{
    "address": "0x558e91924520",
    "mapped": true,
    "hidden": false,
    "at": [9, 49],
    "size": [1582, 942],
    "workspace": {
        "id": 1,
        "name": "1"
    },
    "floating": false,
    "pseudo": false,
    "monitor": 0,
    "class": "kitty",
    "title": "tmux a",
    "initialClass": "kitty",
    "initialTitle": "kitty",
    "pid": 1379,
    "xwayland": false,
    "pinned": false,
    "fullscreen": 0,
    "fullscreenClient": 0,
    "grouped": [],
    "tags": [],
    "swallowing": "0x0",
    "focusHistoryID": 0,
    "inhibitingIdle": false,
    "xdgTag": "",
    "xdgDescription": "",
    "contentType": "none"
},{
    "address": "0x558e928c04d0",
    "mapped": true,
    "hidden": false,
    "at": [9, 49],
    "size": [1582, 942],
    "workspace": {
        "id": 3,
        "name": "3"
    },
    "floating": false,
    "pseudo": false,
    "monitor": 0,
    "class": "QQ",
    "title": "QQ",
    "initialClass": "QQ",
    "initialTitle": "QQ",
    "pid": 296480,
    "xwayland": false,
    "pinned": false,
    "fullscreen": 0,
    "fullscreenClient": 0,
    "grouped": [],
    "tags": [],
    "swallowing": "0x0",
    "focusHistoryID": 2,
    "inhibitingIdle": false,
    "xdgTag": "",
    "xdgDescription": "",
    "contentType": "none"
},{
    "address": "0x558e9243ab50",
    "mapped": true,
    "hidden": false,
    "at": [9, 49],
    "size": [1582, 942],
    "workspace": {
        "id": 2,
        "name": "2"
    },
    "floating": false,
    "pseudo": false,
    "monitor": 0,
    "class": "firefox",
    "title": "rust test assert panic - Google 検索 — Mozilla Firefox",
    "initialClass": "firefox",
    "initialTitle": "Mozilla Firefox",
    "pid": 1386,
    "xwayland": false,
    "pinned": false,
    "fullscreen": 0,
    "fullscreenClient": 0,
    "grouped": [],
    "tags": [],
    "swallowing": "0x0",
    "focusHistoryID": 1,
    "inhibitingIdle": false,
    "xdgTag": "",
    "xdgDescription": "",
    "contentType": "none"
}]
            "##;
        // }}}

        let clients: Vec<HyprClient> = serde_json::from_str(data).unwrap();

        assert!(HyprClient::match_rule(
            &clients,
            "558e9243ab50",
            &Rule {
                workspace: Some(WorkspaceRule::Id(2)),
                class_regex: Some(Regex::new("^firefox$").unwrap()),
                title_regex: Some(Regex::new("^rust.*").unwrap()),
                ..Default::default()
            }
        ));

        assert!(!HyprClient::match_rule(
            &clients,
            "558e928c04d0",
            &Rule {
                workspace: Some(WorkspaceRule::Name("3".into())),
                class_regex: Some(Regex::new("^QQ$").unwrap()),
                title_regex: Some(Regex::new("^rust.*").unwrap()),
                ..Default::default()
            }
        ));

        assert!(!HyprClient::match_rule(
            &clients,
            "lksjhaldskjfhkasljhfklajsh",
            &Rule {
                workspace: Some(WorkspaceRule::Name("3".into())),
                class_regex: Some(Regex::new("^QQ$").unwrap()),
                title_regex: Some(Regex::new("^rust.*").unwrap()),
                ..Default::default()
            }
        ));

        assert!(HyprClient::match_rule(
            &clients,
            "558e91924520",
            &Rule {
                workspace: Some(WorkspaceRule::Name("1".into())),
                class_regex: Some(Regex::new("^kit..$").unwrap()),
                title_regex: Some(Regex::new("^t[a-z].x.*").unwrap()),
                floating: Some(false),
                xwayland: Some(false),
                ..Default::default()
            }
        ));
    }
}
