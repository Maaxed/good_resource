pub mod registry;
pub mod serde;


use std::any::TypeId;
use std::fmt::Debug;

use registry::{Key, UntypedKey};
use variadics_please::all_tuples_enumerated;


pub trait ResourceKeyMap: Copy
{
	fn get_key<T: 'static>(self, id: &str) -> Option<Key<T>>
	{
		self.get_key_untyped(id, TypeId::of::<T>()).and_then(|k| k.cast())
	}

	fn get_key_untyped(self, id: &str, type_id: TypeId) -> Option<UntypedKey>;
}


pub trait ResourceDef: Sized
{
	type Error: Debug;
	type KeyMap: ResourceKeyMap;
	type ValueMap;

	fn load_keys(self) -> Result<(Self::KeyMap, Self::ValueMap), Self::Error>;

	fn load_values(keys: impl ResourceKeyMap, values: Self::ValueMap) -> Result<(), Self::Error>;

	fn load(self) -> Result<(), Self::Error>
	{
		let (keys, values) = self.load_keys()?;
		Self::load_values(keys, values)
	}
}


macro_rules! impl_resource_key_map_tuple
{
	($(($i:tt, $T:ident)),*) =>
	{
		impl<$($T,)*> ResourceKeyMap for ($($T,)*)
		where
			$($T: ResourceKeyMap,)*
		{
			fn get_key_untyped(self, id: &str, type_id: TypeId) -> Option<UntypedKey>
			{
				$(
					if let Some(key) = self.$i.get_key_untyped(id, type_id)
					{
						return Some(key);
					}
				)*

				None
			}
		}
	};
}

all_tuples_enumerated!(impl_resource_key_map_tuple, 1, 10, T);


macro_rules! impl_resource_def_tuple
{
	($(($i:tt, $T:ident, $k:ident, $v:ident)),*) =>
	{
		impl<E, $($T,)*> ResourceDef for ($($T,)*)
		where
			E: Debug,
			$($T: ResourceDef<Error = E>,)*
		{
			type Error = E;
			type KeyMap = ($($T::KeyMap,)*);
			type ValueMap = ($($T::ValueMap,)*);

			fn load_keys(self) -> Result<(Self::KeyMap, Self::ValueMap), Self::Error>
			{
				$(
					let ($k, $v) = self.$i.load_keys()?;
				)*
				Ok((
					($($k,)*),
					($($v,)*),
				))
			}

			fn load_values(keys: impl ResourceKeyMap, values: Self::ValueMap) -> Result<(), Self::Error>
			{
				$(
					$T::load_values(keys, values.$i)?;
				)*
				Ok(())
			}
		}
	};
}

all_tuples_enumerated!(impl_resource_def_tuple, 1, 10, T, k, v);
