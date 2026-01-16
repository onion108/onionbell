use std::collections::HashMap;
use std::env;
use std::fs::OpenOptions;
use std::io::{self, Read};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use log::{debug, trace, warn};
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink, Source};

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
    // {{{ Initialization Stuff

    /// Initialize and check Hyprland sockets' paths.
    /// The first `PathBuf` is the path to the `.socket.sock`, and the second one is `.socket2.sock`.
    /// I really hope if there is a named tuple thing so I can mark them on the type, but
    /// unfortunately there isn't; And it feels really weird to actually have a different type for
    /// such a small thing so I keep it like that.
    fn init_hyprland_socket_path() -> Result<(PathBuf, PathBuf), AppError> {
        trace!("Checking environment variables...");
        let xdg_runtime = env::var("XDG_RUNTIME_DIR")?;
        let hyprland_instance_signature = env::var("HYPRLAND_INSTANCE_SIGNATURE")?;
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
        Ok((socket_path, socket2_path))
    }

    /// Check and load config.
    fn load_config() -> Result<Config, AppError> {
        let config_home = env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|_| {
                env::var("HOME")
                    .map(PathBuf::from)
                    .map(|x| x.join(".config"))
            })
            .map(|x| x.join("onionbell"))
            .unwrap_or("/etc/onionbell".into());
        debug!("Config Home: {}", config_home.to_string_lossy());

        let config_path = config_home.join("config.toml");
        OpenOptions::new()
            .read(true)
            .open(&config_path)
            .map_err(AppError::from)
            .and_then(|mut f| {
                let mut buf = String::new();
                f.read_to_string(&mut buf)?;
                Ok(Config::from_source(&buf)?)
            })
    }

    /// Initialize audio and load all audio data into memory for fast access.
    fn init_audio(
        config: &Config,
    ) -> Result<(OutputStream, Sink, HashMap<PathBuf, Vec<u8>>), AppError> {
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
                    .map_err(AppError::from)
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
        Ok((stream_handle, sink, sound_map))
    }

    // }}}

    pub fn new() -> Result<App, AppError> {
        let (socket_path, socket2_path) = Self::init_hyprland_socket_path()?;

        let config = Self::load_config().unwrap_or_else(|err| {
            warn!("Failed to load configuration: {}", err);
            warn!("Will use default value as fallback. ");
            Config::default()
        });

        let (audio_stream_handle, audio_sink, sound_map) = Self::init_audio(&config)?;

        Ok(App {
            socket_path,
            socket2_path,
            config,
            sound_map,
            audio_stream_handle,
            audio_sink,
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
                    let mut volume = None;
                    match HyprClient::get_clients(&self.socket_path) {
                        Ok(clients) => {
                            for rule in &self.config.rules {
                                if HyprClient::match_rule(&clients, data, rule) {
                                    sfx_path = Some(rule.sound.clone());
                                    volume = Some(rule.volume);
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
                    let volume = volume.unwrap_or(self.config.volume);

                    // Missing sfx_path = no sound
                    if let Some(sfx_path) = sfx_path {
                        self.play_sound(&sfx_path, volume);
                    }
                }
                _ => {
                    debug!("Unhandled event type: {ev_type}");
                }
            }
        }
    }

    fn play_sound(&mut self, sfx_path: &PathBuf, volume: f32) {
        if let Some(data) = self.sound_map.get(sfx_path) {
            match Decoder::try_from(io::Cursor::new(data.clone())) {
                Ok(audio) => {
                    self.audio_stream_handle
                        .mixer()
                        .add(audio.amplify_normalized(volume));
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
