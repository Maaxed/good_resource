use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Result, Watcher};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::{ResourceDef, ResourcePathKind};

pub struct NotifyReloader
{
	_watcher: RecommendedWatcher,
	need_reload: Arc<AtomicBool>,
}

impl NotifyReloader
{
	pub fn new(def: &impl ResourceDef) -> Result<Self>
	{
		let need_reload = Arc::new(AtomicBool::new(false));
		let need_reload2 = need_reload.clone();

		let mut watcher = notify::recommended_watcher(move |event: Result<Event>|
		{
			let Ok(event) = event
			else
			{
				return;
			};

			use EventKind as EK;
			if !matches!(event.kind, EK::Create(_) | EK::Modify(_) | EK::Remove(_))
			{
				return;
			}

			need_reload2.store(true, Ordering::Relaxed);
		})?;

		def.visit_resource_paths(|path, kind|
		{
			use RecursiveMode as RM;
			let recursive = if kind == ResourcePathKind::RecursiveDirectory { RM::Recursive } else { RM::NonRecursive };
			watcher.watch(path, recursive)
		})?;

		Ok(Self
		{
			_watcher: watcher,
			need_reload,
		})
	}

	pub fn check_for_reload<D: ResourceDef>(&mut self, def: D) -> std::result::Result<bool, D::Error>
	{
		if self.need_reload.swap(false, Ordering::Relaxed)
		{
			def.load()?;
			Ok(true)
		}
		else
		{
			Ok(false)
		}
	}
}
