use std::{
    error::Error,
    io::{Read, Seek},
};

use binrw::{BinRead, BinReaderExt, BinResult, ReadOptions};
use serde::Serialize;

#[derive(Copy, Clone, Debug, BinRead, Serialize)]
pub struct FVec2 {
    x: f32,
    y: f32,
}

#[derive(Debug, BinRead, Serialize)]
pub struct Color {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

#[derive(Debug, Serialize, BinRead)]
#[br(little, repr = i32)]
enum ParticleType {
    Triangle,
    Square,
    Diamond,
}

#[derive(Debug, Serialize)]
enum ParticleEmissionType {
    Center,
    BoxEdge(FVec2),
    Box(FVec2),
    Wind,
    BoxEdgeSpiky(FVec2),
}

impl BinRead for ParticleEmissionType {
    type Args = ();

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        options: &ReadOptions,
        _args: Self::Args,
    ) -> BinResult<Self> {
        let discriminant = reader.read_le::<i32>()?;
        let box_size = reader.read_le::<FVec2>()?;

        // Ignore the box size when it's unused
        match discriminant {
            0 => Ok(Self::Center),
            1 => Ok(Self::BoxEdge(box_size)),
            2 => Ok(Self::Box(box_size)),
            3 => Ok(Self::Wind),
            4 => Ok(Self::BoxEdgeSpiky(box_size)),
            _ => Err(binrw::Error::AssertFail {
                pos: options.offset(),
                message: "Unknown emission type".to_string(),
            }),
        }
    }
}

#[derive(Debug, Serialize, BinRead)]
#[br(little, repr = i32)]
enum ParticleLayer {
    BehindTilemap,
    OverTilemap,
}

#[derive(Debug, Serialize, BinRead)]
#[br(little)]
struct ParticleSystemData {
    duration: i32,
    r#type: ParticleType,
    min_emission_interval: i32,
    max_emission_interval: i32,
    min_emission_rate: i32,
    max_emission_rate: i32,

    min_start_velocity: FVec2,
    max_start_velocity: FVec2,
    gravity: f32,
    max_life_time: i32,
    #[br(parse_with = parse_color_as_float)]
    start_color: Color,
    #[br(parse_with = parse_color_as_float)]
    end_color: Color,
    start_size: f32,
    end_size: f32,
    #[br(parse_with = parse_bool)]
    follow_player: bool,
    #[br(parse_with = parse_bool)]
    play_on_spawn: bool,
    #[br(parse_with = parse_bool)]
    destroy_on_end: bool,
    #[br(parse_with = parse_bool)]
    enable_collision: bool,
    #[br(parse_with = parse_bool)]
    clamp_position_in_bounds: bool,

    #[br(align_before = 4)]
    emission_type: ParticleEmissionType, // Originally "spawnPositionType"
    attract_speed: f32,
    layer: ParticleLayer,
    #[br(parse_with = parse_bool)]
    auto_invert_color: bool,
    #[br(align_before = 4)]
    out_of_box_lifetime_loss: i32,
    clamp_box_size: FVec2,
    #[br(parse_with = parse_bool)]
    symmetrical: bool,
}

#[derive(Debug, Serialize, BinRead)]
#[br(repr = i32)]
pub enum Ability {
    None,
    DoubleJump,
    Glider,
    Dash,
    WallJump,
}

#[derive(Debug, Serialize, BinRead)]
#[br(little)]
struct AbilityBlockData {
    size: FVec2,
    abilities: (Ability, Ability),
}

#[derive(Debug, Serialize, BinRead)]
#[br(little)]
struct DoorData {
    size: FVec2,
    group: i32, // Originally called "type"
}

#[derive(Debug, Serialize, BinRead)]
#[br(little)]
struct KeyObjectData {
    group: i32, // Originally called "type"
}

#[derive(Debug, Serialize, BinRead)]
#[br(little)]
struct WindData {
    size: FVec2,
    force: FVec2,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize)]
pub enum WorldType {
    Light,
    Dark,
}

#[derive(Debug, Serialize, BinRead)]
#[br(little, import(world_switch: bool))]
struct PlatformData {
    size: FVec2,
    goal: FVec2,
    speed: f32,
    #[br(parse_with = parse_bool4)]
    spiky: (bool, bool, bool, bool),
    #[br(parse_with = parse_seen, if(world_switch))]
    world_type: Option<WorldType>, // Originally called "seen"
}

#[derive(Debug, Serialize, BinRead)]
#[br(little)]
struct LevelTagData {
    level_id: i32,
    size: FVec2,
}

#[derive(Debug, Serialize, BinRead)]
#[br(repr = i32)]
enum TutorialType {
    WorldSwitch = 1,
    Jump = 2,
    DashSwitchCombo = 3,
    DoubleJump = 4,
    Glider = 5,
    Dash = 6,
    WallJump = 7,
}

#[derive(Debug, Serialize, BinRead)]
#[br(little)]
struct TutorialData {
    tutorial_type: TutorialType,
    size: FVec2,
    #[br(parse_with = parse_bool)]
    instant: bool,
}

pub type TypedValue = (&'static str, serde_json::Value);

pub fn convert_object_data<T: Read + Seek>(
    prototype_id: i32,
    additional_data: &mut T,
) -> Result<TypedValue, Box<dyn Error>> {
    macro_rules! convert {
        ($name:expr, $ty:ty, $args:expr) => {
            (
                $name,
                serde_json::to_value(<$ty>::read_args(additional_data, $args)?)?,
            )
        };
    }

    let value = match prototype_id {
        0 | 1 => convert!("AbilityBlock", AbilityBlockData, ()), // Originally ColorObject
        2 => convert!("Wind", WindData, ()),
        3 => convert!("Platform", PlatformData, (false,)), // Originally MovingObject
        4 => convert!("ParticleSystem", ParticleSystemData, ()),
        5 | 6 => convert!("Platform", PlatformData, (true,)), // Originally MovingSwitchObject
        7 | 8 | 9 => convert!("Key", KeyObjectData, ()),
        10 | 11 | 12 => convert!("Door", DoorData, ()),
        13 => convert!("LevelTag", LevelTagData, ()),
        14 => convert!("Door", DoorData, ()),
        15 => convert!("Tutorial", TutorialData, ()),
        _ => panic!("Unknown prototype ID {}", prototype_id),
    };
    Ok(value)
}

/// Custom parse function to convert a four-byte color to four floats
fn parse_color_as_float<R: Read + Seek>(
    reader: &mut R,
    _ro: &ReadOptions,
    _: (),
) -> BinResult<Color> {
    let (r, g, b, a) = reader.read_le::<(u8, u8, u8, u8)>()?;
    Ok(Color {
        r: r as f32,
        g: g as f32,
        b: b as f32,
        a: a as f32,
    })
}

fn parse_bool<R: Read + Seek>(reader: &mut R, ro: &ReadOptions, _: ()) -> BinResult<bool> {
    let val = reader.read_le::<u8>()?;
    if val <= 1 {
        Ok(val != 0)
    } else {
        Err(binrw::Error::AssertFail {
            pos: ro.offset(),
            message: "Invalid bool value".to_string(),
        })
    }
}

fn parse_bool4<R: Read + Seek>(
    reader: &mut R,
    ro: &ReadOptions,
    _: (),
) -> BinResult<(bool, bool, bool, bool)> {
    let b0 = parse_bool(reader, ro, ())?;
    let b1 = parse_bool(reader, ro, ())?;
    let b2 = parse_bool(reader, ro, ())?;
    let b3 = parse_bool(reader, ro, ())?;
    Ok((b0, b1, b2, b3))
}

fn parse_seen<R: Read + Seek>(
    reader: &mut R,
    options: &ReadOptions,
    _: (),
) -> BinResult<Option<WorldType>> {
    // This is originally encoded as a boolean "seen", which is true if the platform
    // is visible in the Dark world
    let val = reader.read_le::<u8>()?;
    if val == 0 {
        Ok(Some(WorldType::Light))
    } else if val == 1 {
        Ok(Some(WorldType::Dark))
    } else {
        Err(binrw::Error::AssertFail {
            pos: options.offset(),
            message: "Invalid \"seen\" value".to_string(),
        })
    }
}
