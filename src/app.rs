use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{self, Read};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use log::{debug, trace, warn};
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink};

use crate::config::Config;
use crate::error::AppError;
use crate::hypr::HyprClient;

pub struct App {
    pub socket_path: PathBuf,
    pub socket2_path: PathBuf,
    pub config: Config,

    pub audio_stream_handle: OutputStream,
    pub audio_sink: Sink,
    pub sound_map: HashMap<PathBuf, Vec<u8>>,
}

impl App {
    pub fn new() -> Result<App, AppError> {
        // {{{ Initialize and Check Paths & Environment Variables
        trace!("Checking environment variables...");
        let xdg_runtime = std::env::var("XDG_RUNTIME_DIR")?;
        let hyprland_instance_signature = std::env::var("HYPRLAND_INSTANCE_SIGNATURE")?;
        trace!(
            "xdg_runtime = {:?}, hyprland_instance_signature = {:?}",
            xdg_runtime, hyprland_instance_signature
        );
        let path = PathBuf::from(xdg_runtime)
            .join("hypr")
            .join(hyprland_instance_signature);

        let socket_path = path.join(".socket.sock");
        let socket2_path = path.join(".socket2.sock");
        for p in [&socket_path, &socket2_path] {
            trace!("checking p = {:?}", p);
            if !p.exists() {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("File not found: {}", p.to_string_lossy()),
                )
                .into());
            }
        }
        // }}}

        // {{{ Load Configuration
        let config_home = if let Ok(config_dir) = std::env::var("XDG_CONFIG_HOME") {
            PathBuf::from(config_dir).join("onionbell")
        } else {
            if let Ok(home) = std::env::var("HOME") {
                PathBuf::from(home).join(".config").join("onionbell")
            } else {
                PathBuf::from("/etc/onionbell")
            }
        };
        debug!("Config Home: {}", config_home.to_string_lossy());

        let config_path = config_home.join("config.toml");
        let config = match OpenOptions::new().read(true).open(&config_path) {
            Ok(mut f) => {
                let mut buf = String::new();
                if let Err(err) = f.read_to_string(&mut buf) {
                    warn!(
                        "Failed to read configuration at {}, using default value as fallback: {}",
                        config_path.to_string_lossy(),
                        err
                    );
                    Config::default()
                } else {
                    match Config::from_source(&buf) {
                        Ok(x) => x,
                        Err(err) => {
                            warn!(
                                "Failed to parse configuration at {}, using default value as fallback: {}",
                                config_path.to_string_lossy(),
                                err
                            );
                            Config::default()
                        }
                    }
                }
            }
            Err(err) => {
                warn!(
                    "Failed to open configuration at {}, using default value as fallback: {}",
                    config_path.to_string_lossy(),
                    err
                );
                Config::default()
            }
        };
        // }}}

        // {{{ Initialize Audio
        let stream_handle = OutputStreamBuilder::open_default_stream()?;
        let sink = Sink::connect_new(&stream_handle.mixer());
        let mut sound_map = HashMap::new();
        for sfx_path in config
            .sound
            .iter()
            .chain(config.rules.iter().filter_map(|x| x.sound.as_ref()))
        {
            if !sound_map.contains_key(sfx_path) {
                debug!("Loading SFX {}", sfx_path.to_string_lossy());
                match OpenOptions::new()
                    .read(true)
                    .open(sfx_path)
                    .map_err(|e| AppError::from(e))
                    .and_then(|mut x| {
                        let mut buf = Vec::new();
                        x.read_to_end(&mut buf).map(|_| buf).map_err(|e| e.into())
                    }) {
                    Ok(x) => {
                        sound_map.insert(sfx_path.clone(), x);
                    }
                    Err(err) => {
                        warn!(
                            "Failed to read or decode source {}: {}",
                            sfx_path.to_string_lossy(),
                            err
                        );
                    }
                }
            }
        }
        // }}}

        Ok(App {
            socket_path,
            socket2_path,
            config,
            sound_map,
            audio_stream_handle: stream_handle,
            audio_sink: sink,
        })
    }

    pub fn get_event(&self, socket: &mut UnixStream) -> Result<String, AppError> {
        trace!("Waiting for an event");
        let mut buffer = Vec::new();
        loop {
            let mut character_buf = [0u8; 4];
            socket.read(&mut character_buf[0..1])?;
            trace!("Read byte {:02X}", character_buf[0]);

            // Check length of current UTF-8 character.
            let len = match character_buf[0] {
                x if x & 0xC0 == 0xC0 => 2, // 110x xxxx => 2 bytes
                x if x & 0xE0 == 0xE0 => 3, // 1110 xxxx => 3 bytes
                x if x & 0xF0 == 0xF0 => 4, // 1111 0xxx => 4 bytes
                _ => 1,
            };
            trace!("character len = {}", len);

            // Although reading to a 0-sized slice is fine, it will somehow block if no more data
            // is present. It's pretty weird because IMHO read() should just return when 0-sized
            // slice is passed, but anyway it is what it is.
            if len > 1 {
                socket.read(&mut character_buf[1..len])?;
            }
            let character = &character_buf[0..len];
            trace!("character: {:02X?}", character);
            if character == &[0xA] {
                break;
            }
            buffer.extend_from_slice(character);
        }
        Ok(String::from_utf8_lossy(&buffer).to_string())
    }

    pub fn run(&mut self) -> Result<(), AppError> {
        let mut socket2 = UnixStream::connect(&self.socket2_path)?;
        loop {
            let event = self.get_event(&mut socket2)?;
            debug!("{}", event);

            // The response is always in format "event_type>>data" according to Hyprland's
            // documentation (https://wiki.hypr.land/IPC/#xdg_runtime_dirhyprhissocket2sock).
            let Some((ev_type, data)) = event.split_once(">>") else {
                warn!("Weird response from socket2: {}", event);
                continue;
            };
            trace!("ev_type = {ev_type}");
            trace!("data = {data}");

            match ev_type {
                "bell" => {
                    let mut sfx_path = None;
                    match HyprClient::get_clients(&self.socket_path) {
                        Ok(clients) => {
                            for rule in &self.config.rules {
                                if HyprClient::match_rule(&clients, data, rule) {
                                    sfx_path = Some(rule.sound.clone());
                                    break;
                                }
                            }
                        }
                        Err(err) => {
                            warn!(
                                "Failed to get clients from Hyprland {}. Rules will not be matched. ",
                                err
                            );
                        }
                    }
                    let sfx_path = sfx_path.unwrap_or(self.config.sound.clone());

                    // Missing sfx_path = no sound
                    if let Some(sfx_path) = sfx_path {
                        self.play_sound(&sfx_path);
                    }
                }
                _ => {
                    debug!("Unhandled event type: {ev_type}");
                }
            }
        }
    }

    fn play_sound(&mut self, sfx_path: &PathBuf) {
        if let Some(data) = self.sound_map.get(sfx_path) {
            match Decoder::try_from(io::Cursor::new(data.clone())) {
                Ok(audio) => {
                    self.audio_stream_handle.mixer().add(audio);
                }
                Err(err) => {
                    warn!(
                        "Failed to play audio {}: {}",
                        sfx_path.to_string_lossy(),
                        err
                    );
                    self.sound_map.remove(sfx_path);
                }
            }
        }
    }
}
