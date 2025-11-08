use std::path::Path;

use good_ressource::ResourceDef;
use good_ressource::registry::{Key, Registry};
use ron::Options;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Mob
{
	health: u32,
	damage: u32,
}

#[derive(Debug, Deserialize)]
pub struct World
{
	mobs: Vec<Key<Mob>>,
	neighbor_worlds: Vec<Key<World>>,
}

#[derive(Debug, Default)]
struct Resources
{
	mobs: Registry<Mob>,
	worlds: Registry<World>,
}



impl Resources
{
	fn resource_def(&mut self) -> impl ResourceDef
	{
		(
			self.mobs.loading_from_folder(Path::new("examples/test/resources/mob"), Options::default()),
			self.worlds.loading_from_file(Path::new("examples/test/resources/world.ron"), Options::default()),
		)
	}
}

fn main()
{
	let mut resources = Resources::default();
	resources.resource_def().load().unwrap();

	dbg!(resources);
}
