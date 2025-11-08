mod serde;


use std::any::TypeId;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::File;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};
use std::path::Path;
use derive_where::derive_where;
use serde::de::{RegistryKeyMapSeed, RegistryValueMapSeed};
use ::serde::de::DeserializeOwned;

use crate::serde::DeserializerFactory;
use crate::{ResourceDef, ResourceKeyMap, ResourcePathKind};

#[derive_where(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Key<T>
{
	index: u32,
	pd: PhantomData<fn() -> T>,
}

impl<T> Key<T>
{
	pub(crate) fn new(index: usize) -> Self
	{
		Self
		{
			index: index as u32,
			pd: PhantomData,
		}
	}

	fn index(&self) -> usize
	{
		self.index as usize
	}

	pub fn as_untyped(&self) -> UntypedKey
	where
		T: 'static
	{
		UntypedKey
		{
			type_id: TypeId::of::<T>(),
			index: self.index,
		}
	}
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct UntypedKey
{
	type_id: TypeId,
	index: u32,
}

impl UntypedKey
{
	pub fn cast<T: 'static>(&self) -> Option<Key<T>>
	{
		if self.type_id == TypeId::of::<T>()
		{
			Some(Key
			{
				index: self.index,
				pd: PhantomData,
			})
		}
		else
		{
			None
		}
	}
}


#[derive(Debug, Clone)]
pub struct Entry<T>
{
	id: String,
	value: Option<T>,
	yeeted: bool,
}

impl<T> Entry<T>
{
	fn new(id: String) -> Self
	{
		Self
		{
			id,
			value: None,
			yeeted: true,
		}
	}

	fn from_id_value(id: String, value: T) -> Self
	{
		Self
		{
			id,
			value: Some(value),
			yeeted: false,
		}
	}

	fn as_ref<'l>(&'l self, key: Key<T>) -> EntryRef<'l, T>
	{
		EntryRef
		{
			key,
			id: &self.id,
			value: self.value.as_ref(),
			yeeted: self.yeeted,
		}
	}

	fn as_mut<'l>(&'l mut self, key: Key<T>) -> EntryMut<'l, T>
	{
		EntryMut
		{
			key,
			id: &self.id,
			value: self.value.as_mut(),
			yeeted: self.yeeted,
		}
	}

	pub fn replace(&mut self, value: T) -> Option<T>
	{
		let res = self.value.replace(value);
		self.yeeted = false;
		res
	}
}

#[derive(Debug, Copy, Clone)]
pub struct EntryRef<'l, T>
{
	pub key: Key<T>,
	pub id: &'l str,
	pub value: Option<&'l T>,
	pub yeeted: bool,
}

#[derive(Debug)]
pub struct EntryMut<'l, T>
{
	pub key: Key<T>,
	pub id: &'l str,
	pub value: Option<&'l mut T>,
	pub yeeted: bool,
}


pub type RegistryKeyMap<T> = HashMap<String, Key<T>>;
pub type RegistryValueMap<T> = Vec<Entry<T>>;


// Type invariants:
// - id_to_key.len() == key_to_entry.len()
// - for (id, key) in id_to_key { key_to_entry[key.index].id == id }
#[derive(Debug, Clone)]
#[derive_where(Default)]
pub struct Registry<T>
{
	id_to_key: RegistryKeyMap<T>,
	key_to_entry: RegistryValueMap<T>,
}

impl<T> Registry<T>
{
	pub fn new() -> Self
	{
		Self::default()
	}

	pub fn with_capacity(capacity: usize) -> Self
	{
		Self
		{
			id_to_key: HashMap::with_capacity(capacity),
			key_to_entry: Vec::with_capacity(capacity),
		}
	}

	pub fn get_or_insert_key(&mut self, id: String) -> Key<T>
	{
		*self.id_to_key.entry(id).or_insert_with_key(|id|
		{
			let key = self.key_to_entry.len();
			self.key_to_entry.push(Entry::new(id.clone()));
			Key::new(key)
		})
	}

	fn get_or_insert_key_value(&mut self, id: String, value: T) -> Key<T>
	{
		use std::collections::hash_map::Entry as HashEntry;
		match self.id_to_key.entry(id)
		{
			HashEntry::Occupied(o) =>
			{
				self.key_to_entry[o.get().index()].replace(value);
				*o.get()
			},
			HashEntry::Vacant(v) =>
			{
				let key = self.key_to_entry.len();
				self.key_to_entry.push(Entry::from_id_value(v.key().clone(), value));
				Key::new(key)
			},
		}
	}

	pub fn insert_value(&mut self, key: Key<T>, value: T) -> Option<T>
	{
		self.key_to_entry[key.index()].replace(value)
	}

	pub fn entry<'l>(&'l self, key: Key<T>) -> EntryRef<'l, T>
	{
		self.key_to_entry[key.index()].as_ref(key)
	}

	pub fn entry_mut<'l>(&'l mut self, key: Key<T>) -> EntryMut<'l, T>
	{
		self.key_to_entry[key.index()].as_mut(key)
	}

	pub fn get_key(&self, id: &str) -> Option<Key<T>>
	{
		self.id_to_key.get(id).copied()
	}

	pub fn yeet_all(&mut self)
	{
		for entry in self.key_to_entry.iter_mut()
		{
			entry.yeeted = true;
		}
	}

	pub fn iter<'l>(&'l self) -> impl DoubleEndedIterator<Item = EntryRef<'l, T>>
	{
		self.key_to_entry.iter().enumerate().map(|(i, entry)| entry.as_ref(Key::new(i)))
	}

	pub fn iter_mut<'l>(&'l mut self) -> impl DoubleEndedIterator<Item = EntryMut<'l, T>>
	{
		self.key_to_entry.iter_mut().enumerate().map(|(i, entry)| entry.as_mut(Key::new(i)))
	}

	pub fn file_def<'l, P, F>(&'l mut self, file_path: &'l P, factory: F) -> RegistryFileDef<'l, T, F>
	where
		P: AsRef<Path> + ?Sized,
		F: DeserializerFactory,
	{
		RegistryFileDef
		{
			file_path: file_path.as_ref(),
			registry: self,
			factory,
		}
	}

	pub fn dir_def<'l, P, F>(&'l mut self, dir_path: &'l P, factory: F) -> RegistryDirectoryDef<'l, T, F>
	where
		P: AsRef<Path> + ?Sized,
		F: DeserializerFactory,
	{
		RegistryDirectoryDef
		{
			dir_path: dir_path.as_ref(),
			registry: self,
			factory,
		}
	}
}

impl<T> Index<Key<T>> for Registry<T>
{
	type Output = T;

	fn index(&self, index: Key<T>) -> &Self::Output
	{
		self.key_to_entry[index.index()].value.as_ref().expect("the value mut be inserted")
	}
}

impl<T> IndexMut<Key<T>> for Registry<T>
{
	fn index_mut(&mut self, index: Key<T>) -> &mut Self::Output
	{
		self.key_to_entry[index.index()].value.as_mut().expect("the value mut be inserted")
	}
}


impl<T> FromIterator<(String, T)> for Registry<T>
{
	fn from_iter<I: IntoIterator<Item = (String, T)>>(iter: I) -> Self
	{
		let iter = iter.into_iter();

		let mut this = Self::with_capacity(iter.size_hint().0);

		for (id, value) in iter
		{
			this.get_or_insert_key_value(id, value);
		}

		this
	}
}


impl<T> Extend<(String, T)> for Registry<T>
{
	fn extend<I: IntoIterator<Item = (String, T)>>(&mut self, iter: I)
	{
		let iter = iter.into_iter();

		let min_size = iter.size_hint().0;
		self.id_to_key.reserve(min_size);
		self.key_to_entry.reserve(min_size);

		for (id, value) in iter
		{
			self.get_or_insert_key_value(id, value);
		}
	}
}

impl<T: 'static> ResourceKeyMap for &RegistryKeyMap<T>
{
	fn get_key_untyped(self, id: &str, type_id: TypeId) -> Option<UntypedKey>
	{
		if type_id == TypeId::of::<T>()
		{
			self.get(id).map(|k| k.as_untyped())
		}
		else
		{
			None
		}
	}
}


#[derive(Debug)]
pub enum RegistryDeserializeError<D>
{
	Io(std::io::Error),
	Deserialize(D),
}

impl<D> From<std::io::Error> for RegistryDeserializeError<D>
{
	fn from(value: std::io::Error) -> Self
	{
		Self::Io(value)
	}
}


#[derive(Debug)]
pub struct RegistryFileDef<'l, T, F>
{
	file_path: &'l Path,
	registry: &'l mut Registry<T>,
	factory: F,
}

impl<'l, T, F> ResourceDef for RegistryFileDef<'l, T, F>
where
	T: DeserializeOwned + 'static,
	F: DeserializerFactory,
{
	type Error = RegistryDeserializeError<F::Error>;
	type KeyMap = &'l RegistryKeyMap<T>;
	type ValueMap = (&'l Path, &'l RegistryKeyMap<T>, &'l mut RegistryValueMap<T>, F);

	fn visit_resource_paths<E>(&self, mut visitor: impl FnMut(&std::path::Path, ResourcePathKind) -> Result<(), E>) -> Result<(), E>
	{
		visitor(self.file_path, ResourcePathKind::File)
	}

	fn load_keys(mut self) -> Result<(Self::KeyMap, Self::ValueMap), Self::Error>
	{
		let mut file = File::open(self.file_path)?;

		self.registry.yeet_all();
		
		self.factory.deserialize_from_file(&mut file, RegistryKeyMapSeed(self.registry)).map_err(RegistryDeserializeError::Deserialize)?;

		Ok((
			&self.registry.id_to_key,
			(self.file_path, &self.registry.id_to_key, &mut self.registry.key_to_entry, self.factory),
		))
	}

	fn load_values(keys: impl ResourceKeyMap, mut values: Self::ValueMap) ->  Result<(), Self::Error>
	{
		let mut file = File::open(values.0)?;
		let seed = serde::de::wrap(keys, RegistryValueMapSeed(values.1, values.2));
		values.3.deserialize_from_file(&mut file, seed).map_err(RegistryDeserializeError::Deserialize)?;

		Ok(())
	}
}






/// Read each file in a directory as a registry entry.
/// The name of the file is the id of the entry.
#[derive(Debug)]
pub struct RegistryDirectoryDef<'l, T, F>
{
	dir_path: &'l Path,
	registry: &'l mut Registry<T>,
	factory: F,
}

impl<'l, T, F> RegistryDirectoryDef<'l, T, F>
where
	F: DeserializerFactory,
{
	fn file_path_to_id(path: &Path) -> Option<&str>
	{
		if !path.is_file()
		{
			return None;
		}

		if path.extension().is_some_and(|ext| !ext.eq_ignore_ascii_case(F::FILE_EXTENSION))
		{
			return None;
		}

		let file_name = path.file_stem().and_then(OsStr::to_str)?;
		if file_name.starts_with('.')
		{
			return None;
		}

		Some(file_name)
	}
}

impl<'l, T, F> ResourceDef for RegistryDirectoryDef<'l, T, F>
where
	T: DeserializeOwned + 'static,
	F: DeserializerFactory,
{
	type Error = RegistryDeserializeError<F::Error>;
	type KeyMap = &'l RegistryKeyMap<T>;
	type ValueMap = (&'l Path, &'l RegistryKeyMap<T>, &'l mut RegistryValueMap<T>, F);

	fn visit_resource_paths<E>(&self, mut visitor: impl FnMut(&std::path::Path, ResourcePathKind) -> Result<(), E>) -> Result<(), E>
	{
		visitor(self.dir_path, ResourcePathKind::Directory)
	}

	fn load_keys(self) -> Result<(Self::KeyMap, Self::ValueMap), Self::Error>
	{
		let entries = std::fs::read_dir(self.dir_path)?;

		self.registry.yeet_all();

		for entry in entries
		{
			let path = entry?.path();

			let Some(file_name) = Self::file_path_to_id(&path)
			else
			{
				continue;
			};

			self.registry.get_or_insert_key(file_name.to_owned());
		}

		Ok((
			&self.registry.id_to_key,
			(self.dir_path, &self.registry.id_to_key, &mut self.registry.key_to_entry, self.factory),
		))
	}

	fn load_values(keys: impl ResourceKeyMap, mut values: Self::ValueMap) ->  Result<(), Self::Error>
	{
		for entry in std::fs::read_dir(values.0)?
		{
			let path = entry?.path();

			let Some(file_name) = Self::file_path_to_id(&path)
			else
			{
				continue;
			};

			let Some(key) = values.1.get(file_name).copied()
			else
			{
				continue;
			};

			let mut file = File::open(path)?;
			let seed = serde::de::wrap(keys, PhantomData::<T>);
			let value = values.3.deserialize_from_file(&mut file, seed).map_err(RegistryDeserializeError::Deserialize)?;

			let entry = &mut values.2[key.index()];
			entry.replace(value);
		}

		Ok(())
	}
}
