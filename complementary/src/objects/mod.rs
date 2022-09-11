pub mod ability_block;
pub mod door;
pub mod key;
pub mod level_tag;
pub mod particle_system;
pub mod platform;
pub mod tutorial;
pub mod wind;

use std::{
	fs::File,
	io::{self, BufReader},
	path::Path,
};

use serde::Deserialize;

use crate::{
	game::{ObjectTickState, WorldType},
	math::{FVec2, Bounds, Direction},
	rendering::DrawState,
	window::DrawContext, player::{Player, CollisionType}, level::LevelState,
};

use self::{
	ability_block::{AbilityBlockData, AbilityBlockRenderer},
	door::{DoorData, DoorRenderer, DoorState},
	key::{KeyData, KeyRenderer, KeyState},
	level_tag::{LevelTagData, LevelTagRenderer},
	particle_system::{ParticleSystemData, ParticleSystemRenderer, ParticleSystemObject, ParticleSystemState},
	platform::{PlatformData, PlatformRenderer, PlatformState},
	tutorial::{TutorialData, TutorialRenderer},
	wind::{WindData, WindRenderer},
};

// Used during deserialization
#[derive(Debug, Deserialize)]
struct SerializedObject {
	position: FVec2,
	#[serde(flatten)]
	data: ObjectData,
}

macro_rules! object_multi_list {
	($(($vec_name:ident, $name:ident, $data:ty, $state:ty)),*) => {
		// Used during deserialization
		#[derive(Debug, Deserialize)]
		#[serde(tag = "type", content = "data")]
		enum ObjectData {
			$(
				$name($data),
			)*
		}

        $(
            impl TryFrom<SerializedObject> for Object<$data, $state> {
                type Error = ObjectSetLoadError;

                fn try_from(obj: SerializedObject) -> Result<Self, Self::Error> {
                    match obj.data {
                        ObjectData::$name(inner) => Ok(Self::new(obj.position, inner)),
                        _ => Err(ObjectSetLoadError::InvalidSourceType)
                    }
                }
            }
        )*

		// The paste! macro is used to create an identifier in the form "renderer_[name]"
		paste::paste! {
			pub struct ObjectMultiList {
				$(
					pub $vec_name: Vec<Object::<$data, $state>>,
					[<renderer_ $vec_name>]: [<$name Renderer>],
				)*
			}

			impl ObjectMultiList {
				fn new(serialized_objects: Vec<SerializedObject>, device: &wgpu::Device) -> Self {
					$(
						let mut $vec_name = Vec::new();
						let [<renderer_ $vec_name>] = [<$name Renderer>]::new(&device);
					)*

					for obj in serialized_objects {
						match obj.data {
							$(
								ObjectData::$name(inner) => $vec_name.push(Object::<$data, $state>::new(obj.position, inner)),
							)*
						};
					}

					Self {
						$(
							$vec_name,
							[<renderer_ $vec_name>],
						)*
					}
				}

				fn draw(&mut self, context: &mut DrawContext, state: &DrawState, world_type: WorldType) {
					$(
						self.[<renderer_ $vec_name>].draw(&self.$vec_name, context, state, world_type);
					)*
				}
			}

			impl Tickable for ObjectMultiList {
				fn tick(&mut self, state: &mut ObjectTickState) {
					$(
						for obj in &mut self.$vec_name {
							obj.tick(state);
						}
					)*
				}
			}
		}
	};
}

macro_rules! object_multi_list_collision {
	($($vec_name:ident),*) => {
		impl ObjectMultiList {
			pub fn check_collision(&self, bounds: &Bounds, world_type: WorldType) -> Option<CollisionType> {
				$(
					if let Some(ty) = self.$vec_name.iter().find_map(|obj| obj.collides_with(bounds, world_type)) {
						return Some(ty);
					}
				)*
				None
			}

			pub fn handle_directional_collision(&mut self, bounds: &Bounds, player: &mut Player, level_state: &mut LevelState, world_type: WorldType, direction: Direction) -> Option<CollisionType> {
				$(
					for obj in &mut self.$vec_name {
						if let Some(ty) = obj.collides_with(&bounds, world_type) {
							obj.on_directional_collision(player, level_state, direction);
							return Some(ty);
						}
					}
				)*
				None
			}
		}
	};
}

object_multi_list! {
	(ability_blocks, AbilityBlock, AbilityBlockData, ()),
	(winds, Wind, WindData, ()),
	(platforms, Platform, PlatformData, PlatformState),
	(particle_systems, ParticleSystem, ParticleSystemData, ParticleSystemState),
	(keys, Key, KeyData, KeyState),
	(doors, Door, DoorData, DoorState),
	(level_tags, LevelTag, LevelTagData, ()),
	(tutorials, Tutorial, TutorialData, ())
}

object_multi_list_collision!(ability_blocks, platforms, keys, doors);

// Used at run-time
#[derive(Debug)]
pub struct Object<TData, TState> {
	pub position: FVec2,
	data: TData,
	state: TState,
}

pub trait Tickable {
	fn tick(&mut self, state: &mut ObjectTickState);
}

pub trait Positional {
	fn position(&self) -> FVec2;
}

impl<TData, TState> Positional for Object<TData, TState> {
	fn position(&self) -> FVec2 {
		self.position
	}
}

pub trait PositionalWithSize : Positional {
	fn size(&self) -> FVec2;

	fn bounds(&self) -> Bounds {
		let pos = self.position();
		Bounds::new(pos, pos + self.size())
	}
}

pub trait Collidable : PositionalWithSize {
	fn collides_with(&self, other: &Bounds, world_type: WorldType) -> Option<CollisionType> {
		self.bounds().overlaps(other).then_some(CollisionType::Solid)
	}

	fn on_directional_collision(&mut self, _player: &mut Player, _level_state: &mut LevelState, _direction: Direction) {
		// Do nothing by default
	}
}

fn load_prefab_data<P: AsRef<Path>>(path: &P) -> Result<SerializedObject, ObjectSetLoadError> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    Ok(serde_json::from_reader(reader)?)
}

pub fn load_particle_system<P: AsRef<Path>>(path: &P) -> Result<ParticleSystemObject, ObjectSetLoadError> {
    let prefab = load_prefab_data(path)?;
    prefab.try_into()
}

pub struct ObjectSet {
	pub objects: ObjectMultiList,
}

impl ObjectSet {
	pub fn load_from_file<T: AsRef<Path>>(
		path: T,
		device: &wgpu::Device,
	) -> Result<ObjectSet, ObjectSetLoadError> {
		let file = File::open(path)?;
		let reader = BufReader::new(file);

		let object_data: Vec<SerializedObject> = serde_json::from_reader(reader)?;

		let objects = ObjectMultiList::new(object_data, &device);

		Ok(ObjectSet { objects })
	}

	pub fn draw(&mut self, context: &mut DrawContext, state: &DrawState, world_type: WorldType) {
		self.objects.draw(context, state, world_type);
	}

	pub fn check_collision(&self, bounds: &Bounds, world_type: WorldType) -> Option<CollisionType> {
		self.objects.check_collision(bounds, world_type)
	}

	pub fn handle_directional_collision(&mut self, bounds: &Bounds, player: &mut Player, level_state: &mut LevelState, world_type: WorldType, direction: Direction) -> Option<CollisionType> {
		self.objects.handle_directional_collision(bounds, player, level_state, world_type, direction)
	}
}

impl Tickable for ObjectSet {
	fn tick(&mut self, state: &mut ObjectTickState) {
		self.objects.tick(state);
	}
}

#[derive(thiserror::Error, Debug)]
pub enum ObjectSetLoadError {
	#[error("IO error: {0}")]
	Io(#[from] io::Error),
	#[error("invalid data: {0}")]
	InvalidData(#[from] serde_json::Error),
    #[error("invalid source type")]
	InvalidSourceType,
}
