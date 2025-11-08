use std::time::Duration;

use good_resource::ResourceDef;
use good_resource::notify::NotifyReloader;
use good_resource::registry::{Key, Registry};
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
			self.mobs.dir_def("examples/test/resources/mob", Options::default()),
			self.worlds.file_def("examples/test/resources/world.ron", Options::default()),
		)
	}
}

fn main()
{
	let mut resources = Resources::default();
	resources.resource_def().load().unwrap();

	dbg!(&resources);

	let mut reloaded = NotifyReloader::new(&resources.resource_def()).unwrap();

	loop
	{
		std::thread::sleep(Duration::from_secs_f32(0.5));

		if reloaded.check_for_reload(resources.resource_def()).unwrap()
		{
			dbg!(&resources);
		}
	}
}
