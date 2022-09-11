use std::{collections::BTreeMap, sync::Mutex};

use sdl2::{mixer::{self, Channel, InitFlag, Sdl2MixerContext, Chunk, MAX_VOLUME}};

use crate::game::WorldType;

const MAX_CHANNELS: i32 = 16;
const GROUP_ID: i32 = 1;
const LIGHT_MUSIC_CHANNEL: Channel = Channel(0);
const DARK_MUSIC_CHANNEL: Channel = Channel(1);
const MUSIC_VOLUME: i32 = MAX_VOLUME / 4;

lazy_static::lazy_static! {
    static ref AUDIO_INSTANCE: Mutex<GameAudio> = Mutex::new(GameAudio::new().expect("Failed to init audio"));
}

pub fn set_world(world_type: WorldType) {
    AUDIO_INSTANCE.lock().expect("Poisoned `GameAudio` mutex").set_world(world_type);
}

struct GameAudio {
    mixer_context: Sdl2MixerContext,
    chunks_by_channel: BTreeMap<i32, Chunk>
}

// The `Chunk` type contains a pointer, so we need to manually
// convince the `Mutex` type to allow holding it
unsafe impl Send for GameAudio {}

impl GameAudio {
    fn new() -> Result<GameAudio, String> {
        mixer::open_audio(44100, mixer::DEFAULT_FORMAT, 2, 4096)?;

        let mixer_context = sdl2::mixer::init(InitFlag::OGG)?;

        mixer::allocate_channels(MAX_CHANNELS);

        let group = mixer::Group(GROUP_ID);
        group.add_channels_range(DARK_MUSIC_CHANNEL.0 + 1, MAX_CHANNELS - 1);
        mixer::set_channel_finished(GameAudio::channel_finished);

        let light_chunk = Chunk::from_file("assets/sounds/light.ogg")?;
        LIGHT_MUSIC_CHANNEL.play(&light_chunk, -1)?;

        let dark_chunk = Chunk::from_file("assets/sounds/dark.ogg")?;
        DARK_MUSIC_CHANNEL.play(&dark_chunk, -1)?;

        let mut chunks_by_channel = BTreeMap::new();
        chunks_by_channel.insert(LIGHT_MUSIC_CHANNEL.0, light_chunk);
        chunks_by_channel.insert(DARK_MUSIC_CHANNEL.0, dark_chunk);

        Ok(GameAudio { mixer_context, chunks_by_channel })
    }

    fn channel_finished(channel: Channel) {
        println!("finished: {}", channel.0);
    }

    fn get_music_channel(world_type: WorldType) -> Channel {
        match world_type {
            WorldType::Light => LIGHT_MUSIC_CHANNEL,
            WorldType::Dark => DARK_MUSIC_CHANNEL,
        }
    }

    fn set_world(&self, world_type: WorldType) {
        GameAudio::get_music_channel(world_type).set_volume(MUSIC_VOLUME);
        GameAudio::get_music_channel(world_type.inverse()).set_volume(0);
    }
}
