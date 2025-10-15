use anyhow::{Result, anyhow};
use mpris::{PlaybackStatus, Player, PlayerFinder};
use serde::Serialize;
use std::{collections::HashMap, env, fs::File, path::PathBuf, time::Duration};
use symphonia::core::{
    formats::FormatOptions,
    io::MediaSourceStream,
    meta::{MetadataOptions, Value},
    probe::Hint,
};

fn main() -> Result<()> {
    let playername = if let Some(arg) = env::args().nth(1) {
        arg
    } else {
        return Err(anyhow!("you have to pass a player name"));
    };

    let playerfinder = PlayerFinder::new()?;
    let mut track = Track::new();
    track.status = "Quit";

    loop {
        let player = if let Ok(player) = playerfinder.find_by_name(&playername) {
            player
        } else {
            println!("{}", serde_json::to_string(&track)?);
            std::thread::sleep(std::time::Duration::from_millis(200));
            continue;
        };

        while player.is_running() {
            if process_mpris_data(&player, &mut track).is_err() {
                track.status = "Stopped";
            }

            println!("{}", serde_json::to_string(&track)?);
            std::thread::sleep(Duration::from_millis(200));
        }

        track.status = "Quit";
        track.path.clear();
        track.metadata.clear();
        track.duration = 0;
        println!("{}", serde_json::to_string(&track)?);
    }
}

fn process_mpris_data(player: &Player, track: &mut Track) -> Result<()> {
    let metadata = player.get_metadata()?;
    let path = if let Some(path) = metadata.url() {
        PathBuf::from(path)
    } else {
        return Err(anyhow!("failed to get file path"));
    };

    if path != track.path {
        if let Some(duration) = metadata.length_in_microseconds() {
            track.duration = duration;
        } else {
            return Err(anyhow!("failed to get track duration"));
        }

        track.update_track(path)?;
    }
    track.position = player.get_position_in_microseconds()?;

    track.status = match player.get_playback_status()? {
        PlaybackStatus::Playing => "Playing",
        PlaybackStatus::Paused => "Paused",
        PlaybackStatus::Stopped => "Stopped",
    };

    Ok(())
}

#[derive(Debug, Default, Serialize)]
struct Track {
    #[serde(skip_serializing)]
    path: PathBuf,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    metadata: HashMap<String, String>,
    position: u64,
    duration: u64,
    status: &'static str,
}

impl Track {
    fn new() -> Self {
        Self::default()
    }

    fn update_track(&mut self, path: PathBuf) -> Result<()> {
        if !path.exists() {
            return Err(anyhow!("file does not exist"));
        }

        let file = File::open(&path)?;

        let mss = MediaSourceStream::new(Box::new(file), Default::default());
        let mut hint = Hint::new();
        if let Some(ext) = path.extension() {
            hint.with_extension(ext.to_str().unwrap());
        }
        let meta_opts: MetadataOptions = Default::default();
        let fmt_opts: FormatOptions = Default::default();

        let mut probed =
            symphonia::default::get_probe().format(&hint, mss, &fmt_opts, &meta_opts)?;

        let metadataobject = probed.format.metadata();

        let metadata = if let Some(meta) = metadataobject.current() {
            meta
        } else {
            return Err(anyhow!("failed to get metadata"));
        };

        self.path = path;

        self.metadata.clear();

        for tag in metadata.tags() {
            let value = match tag.value.clone() {
                Value::Binary(_) => None,
                Value::Boolean(val) => Some(format!("{val:?}")),
                Value::Flag => Some(String::from("1")),
                Value::Float(val) => Some(format!("{val:?}")),
                Value::SignedInt(val) => Some(format!("{val:?}")),
                Value::String(val) => Some(val),
                Value::UnsignedInt(val) => Some(format!("{val:?}")),
            };

            if let Some(val) = value {
                if let Some(stdtag) = tag.std_key {
                    self.metadata.insert(format!("{stdtag:?}"), val);
                } else {
                    self.metadata.insert(tag.key.clone(), val);
                }
            }
        }

        Ok(())
    }
}
