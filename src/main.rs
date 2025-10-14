use anyhow::{Result, anyhow};
use mpris::PlayerFinder;
use std::{collections::HashMap, env, path::PathBuf};

fn main() -> Result<()> {
    let playername = if let Some(arg) = env::args().next() {
        arg
    } else {
        return Err(anyhow!("you have to pass a player name"));
    };

    let playerfinder = PlayerFinder::new()?;

    loop {
        let player = if let Ok(player) = playerfinder.find_by_name(&playername) {
            player
        } else {
            std::thread::sleep(std::time::Duration::from_millis(200));
            continue;
        };

        let mut track = Track::new();

        while player.is_running() {
            let metadata = player.get_metadata()?;
            let path = if let Some(path) = metadata.url() {
                PathBuf::from(path)
            } else {
                continue;
            };

            if path != track.path {
                track.update_track(path);
            }
        }
    }
}

#[derive(Debug, Default)]
struct Track {
    path: PathBuf,
    metadata: HashMap<String, String>,
    position: u64,
}

impl Track {
    fn new() -> Self {
        Self::default()
    }

    fn update_track(&mut self, path: PathBuf) {}
}
