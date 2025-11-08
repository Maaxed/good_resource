use std::any::TypeId;
use std::cell::Cell;
use std::marker::PhantomData;

use serde::de::{DeserializeSeed, EnumAccess, MapAccess, SeqAccess, Unexpected, VariantAccess, Visitor};
use serde::{Deserialize, Deserializer};

use crate::ResourceKeyMap;
use crate::registry::{Key, UntypedKey};

use super::super::{Registry, RegistryKeyMap, RegistryValueMap};

// Hack to pass special parameters to my deserializer
const PRIVATE_IDENT_HACK: &str = "good_resource::__private::deserialize::Key";

thread_local!
{
	static TYPE_ID_IN: Cell<Option<TypeId>> = const { Cell::new(None) };
	static KEY_OUT: Cell<Option<UntypedKey>> = const { Cell::new(None) };
}

impl<'de, T: 'static> Deserialize<'de> for Key<T>
{
	fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error>
	{
		struct Reset;

		impl Drop for Reset
		{
			fn drop(&mut self)
			{
				TYPE_ID_IN.set(None);
				KEY_OUT.set(None);
			}
		}

		let _ = Reset;
		TYPE_ID_IN.set(Some(TypeId::of::<T>()));

		deserializer.deserialize_newtype_struct(PRIVATE_IDENT_HACK, KeyVisitor(PhantomData))
	}
}

struct KeyVisitor<T>(PhantomData<fn() -> T>);

impl<'de, T: 'static> Visitor<'de> for KeyVisitor<T>
{
	type Value = Key<T>;

	fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result
	{
		write!(formatter, "a registry key of type {}", std::any::type_name::<T>())
	}

	fn visit_str<E>(self, _: &str) -> Result<Self::Value, E>
	where
		E: serde::de::Error,
	{
		let Some(out) = KEY_OUT.take()
		else
		{
			return Err(E::custom("unsupported deserializer, expected a KeyDeserializer"))
		};

		Ok(out.cast().unwrap())
	}
}


pub struct KeyDeserializerWrapper<I, S>
{
	store: S,
	inner: I,
}

pub fn wrap<I, S>(store: S, inner: I) -> KeyDeserializerWrapper<I, S>
{
	KeyDeserializerWrapper
	{
		store,
		inner,
	}
}


macro_rules! delegate_functions
{
	($($fn:ident)*) =>
	{
		$(
			fn $fn<V>(self, visitor: V) -> Result<V::Value, Self::Error>
			where
				V: Visitor<'de>
			{
				self.inner.$fn(wrap(self.store, visitor))
			}
		)*
	};
}

macro_rules! delegate_functions_param
{
	($($fn:ident ( $($param_name:ident : $param_type:ty),* )),* $(,)?) =>
	{
		$(
			fn $fn<V>(self, $($param_name: $param_type ,)* visitor: V) -> Result<V::Value, Self::Error>
			where
				V: Visitor<'de>
			{
				self.inner.$fn($($param_name,)* wrap(self.store, visitor))
			}
		)*
	};
}

impl<'de, D, S> Deserializer<'de> for KeyDeserializerWrapper<D, S>
where
	D: Deserializer<'de>,
	S: ResourceKeyMap,
{
	type Error = D::Error;

	delegate_functions!(
		deserialize_any
		deserialize_bool
		deserialize_i8
		deserialize_i16
		deserialize_i32
		deserialize_i64
		deserialize_i128
		deserialize_u8
		deserialize_u16
		deserialize_u32
		deserialize_u64
		deserialize_u128
		deserialize_f32
		deserialize_f64
		deserialize_char
		deserialize_str
		deserialize_string
		deserialize_bytes
		deserialize_byte_buf
		deserialize_option
		deserialize_unit
		deserialize_seq
		deserialize_map
		deserialize_identifier
		deserialize_ignored_any
	);

	delegate_functions_param!(
		deserialize_unit_struct(name: &'static str),
		//deserialize_newtype_struct(name: &'static str),
		deserialize_tuple(len: usize),
		deserialize_tuple_struct(name: &'static str, len: usize),
		deserialize_struct(name: &'static str, fields: &'static [&'static str]),
		deserialize_enum(name: &'static str, variants: &'static [&'static str]),
	);

	fn deserialize_newtype_struct<V>(
		self,
		name: &'static str,
		visitor: V,
	) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>
	{
		if let Some(type_id) = TYPE_ID_IN.take()
		{
			struct KeyIdVisitor<V, S>(V, S, TypeId);

			impl<'de, V, S> Visitor<'de> for KeyIdVisitor<V, S>
			where
				V: Visitor<'de>,
				S: ResourceKeyMap,
			{
				type Value = V::Value;

				fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result
				{
					self.0.expecting(formatter)
				}

				fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
				where
					E: serde::de::Error,
				{
					let key = self.1.get_key_untyped(v, self.2).ok_or_else(|| E::invalid_value(Unexpected::Str(v), &self))?;
					KEY_OUT.set(Some(key));
					self.0.visit_str(v)
				}
			}

			return self.inner.deserialize_identifier(KeyIdVisitor(visitor, self.store, type_id));
		}

		self.inner.deserialize_newtype_struct(name, wrap(self.store, visitor))
	}

	fn is_human_readable(&self) -> bool
	{
		self.inner.is_human_readable()
	}
}


impl<'de, Seed, S> DeserializeSeed<'de> for KeyDeserializerWrapper<Seed, S>
where
	Seed: DeserializeSeed<'de>,
	S: ResourceKeyMap,
{
	type Value = Seed::Value;

	fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
	where
		D: Deserializer<'de>
	{
		self.inner.deserialize(wrap(self.store, deserializer))
	}
}

macro_rules! delegate_functions_visitor
{
	($($fn:ident : $type:ty),* $(,)?) =>
	{
		$(
			fn $fn<E>(self, v: $type) -> Result<Self::Value, E>
			where
				E: serde::de::Error,
			{
				self.inner.$fn(v)
			}
		)*
	};
}

impl<'de, V, S> Visitor<'de> for KeyDeserializerWrapper<V, S>
where
	V: Visitor<'de>,
	S: ResourceKeyMap,
{
	type Value = V::Value;

	fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result
	{
		self.inner.expecting(formatter)
	}

	delegate_functions_visitor!
	{
		visit_bool: bool,
		visit_i8: i8,
		visit_i16: i16,
		visit_i32: i32,
		visit_i64: i64,
		visit_i128: i128,
		visit_u8: u8,
		visit_u16: u16,
		visit_u32: u32,
		visit_u64: u64,
		visit_u128: u128,
		visit_f32: f32,
		visit_f64: f64,
		visit_char: char,
		visit_str: &str,
		visit_borrowed_str: &'de str,
		visit_string: String,
		visit_bytes: &[u8],
		visit_borrowed_bytes: &'de [u8],
		visit_byte_buf: Vec<u8>,
	}

	fn visit_none<E>(self) -> Result<Self::Value, E>
	where
		E: serde::de::Error,
	{
		self.inner.visit_none()
	}

	fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
	where
		D: Deserializer<'de>,
	{
		self.inner.visit_some(wrap(self.store, deserializer))
	}

	fn visit_unit<E>(self) -> Result<Self::Value, E>
	where
		E: serde::de::Error,
	{
		self.inner.visit_unit()
	}

	fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
	where
		D: Deserializer<'de>,
	{
		self.inner.visit_newtype_struct(wrap(self.store, deserializer))
	}

	fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
	where
		A: serde::de::SeqAccess<'de>,
	{
		self.inner.visit_seq(wrap(self.store, seq))
	}

	fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
	where
		A: serde::de::MapAccess<'de>,
	{
		self.inner.visit_map(wrap(self.store, map))
	}

	fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
	where
		A: serde::de::EnumAccess<'de>,
	{
		self.inner.visit_enum(wrap(self.store, data))
	}
}

impl<'de, A, S> SeqAccess<'de> for KeyDeserializerWrapper<A, S>
where
	A: SeqAccess<'de>,
	S: ResourceKeyMap,
{
	type Error = A::Error;

	fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
	where
		T: DeserializeSeed<'de>
	{
		self.inner.next_element_seed(wrap(self.store, seed))
	}

	fn size_hint(&self) -> Option<usize>
	{
		self.inner.size_hint()
	}
}

impl<'de, A, S> MapAccess<'de> for KeyDeserializerWrapper<A, S>
where
	A: MapAccess<'de>,
	S: ResourceKeyMap,
{
	type Error = A::Error;

	fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
	where
		K: DeserializeSeed<'de>
	{
		self.inner.next_key_seed(wrap(self.store, seed))
	}

	fn next_value_seed<K>(&mut self, seed: K) -> Result<K::Value, Self::Error>
	where
		K: DeserializeSeed<'de>
	{
		self.inner.next_value_seed(wrap(self.store, seed))
	}

	fn next_entry_seed<K, V>(
		&mut self,
		kseed: K,
		vseed: V,
	) -> Result<Option<(K::Value, V::Value)>, Self::Error>
	where
		K: DeserializeSeed<'de>,
		V: DeserializeSeed<'de>,
	{
		self.inner.next_entry_seed(wrap(self.store, kseed), wrap(self.store, vseed))
	}

	fn size_hint(&self) -> Option<usize>
	{
		self.inner.size_hint()
	}
}

impl<'de, A, S> EnumAccess<'de> for KeyDeserializerWrapper<A, S>
where
	A: EnumAccess<'de>,
	S: ResourceKeyMap,
{
	type Error = A::Error;
	type Variant = KeyDeserializerWrapper<A::Variant, S>;

	fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
	where
		V: DeserializeSeed<'de>
	{
		let (r, var) = self.inner.variant_seed(wrap(self.store, seed))?;
		Ok((r, wrap(self.store, var)))
	}
}

impl<'de, A, S> VariantAccess<'de> for KeyDeserializerWrapper<A, S>
where
	A: VariantAccess<'de>,
	S: ResourceKeyMap,
{
	type Error = A::Error;

	fn unit_variant(self) -> Result<(), Self::Error>
	{
		self.inner.unit_variant()
	}

	fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
	where
		T: DeserializeSeed<'de>
	{
		self.inner.newtype_variant_seed(wrap(self.store, seed))
	}

	fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>
	{
		self.inner.tuple_variant(len, wrap(self.store, visitor))
	}

	fn struct_variant<V>(
		self,
		fields: &'static [&'static str],
		visitor: V,
	) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>
	{
		self.inner.struct_variant(fields, wrap(self.store, visitor))
	}
}





pub struct RegistryKeyMapSeed<'l, T>(pub &'l mut Registry<T>);

impl<'de, 'l, T> DeserializeSeed<'de> for RegistryKeyMapSeed<'l, T>
{
	type Value = ();

	fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
	where
		D: Deserializer<'de>
	{
		deserializer.deserialize_map(self)
	}
}

impl<'de, 'l, T> Visitor<'de> for RegistryKeyMapSeed<'l, T>
{
	type Value = ();

	fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result
	{
		write!(formatter, "a map")
	}

	fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
	where
		A: serde::de::MapAccess<'de>,
	{
		struct KeyId(String);

		impl<'de> Deserialize<'de> for KeyId
		{
			fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
			where
				D: Deserializer<'de>
			{
				struct KeyIdVisitor;

				impl<'de> Visitor<'de> for KeyIdVisitor
				{
					type Value = KeyId;
					fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result
					{
						write!(formatter, "an identifier")
					}

					fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
					where
						E: serde::de::Error,
					{
						Ok(KeyId(v.to_owned()))
					}

					fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
					where
						E: serde::de::Error,
					{
						Ok(KeyId(v))
					}
				}

				deserializer.deserialize_identifier(KeyIdVisitor)
			}
		}

		while let Some((k, _)) = map.next_entry::<KeyId, serde::de::IgnoredAny>()?
		{
			self.0.get_or_insert_key(k.0);
		}

		Ok(())
	}
}

pub struct RegistryValueMapSeed<'l, T>(pub &'l RegistryKeyMap<T>, pub &'l mut RegistryValueMap<T>);

impl<'de, 'l, T> DeserializeSeed<'de> for RegistryValueMapSeed<'l, T>
where
	T: Deserialize<'de>,
{
	type Value = ();

	fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
	where
		D: Deserializer<'de>
	{
		struct RegistryValueMapVisitor<'l, T>(&'l RegistryKeyMap<T>, &'l mut RegistryValueMap<T>);

		impl<'de, 'l, T> Visitor<'de> for RegistryValueMapVisitor<'l, T>
		where
			T: Deserialize<'de>,
		{
			type Value = ();

			fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result
			{
				write!(formatter, "a map")
			}

			fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
			where
				A: serde::de::MapAccess<'de>,
			{
				struct KeyIdSeed<'l, T>(&'l RegistryKeyMap<T>);

				impl<'l, 'de, T> DeserializeSeed<'de> for KeyIdSeed<'l, T>
				{
					type Value = Key<T>;

					fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
					where
						D: Deserializer<'de>
					{
						deserializer.deserialize_identifier(self)
					}
				}

				impl<'l, 'de, T> Visitor<'de> for KeyIdSeed<'l, T>
				{
					type Value = Key<T>;

					fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result
					{
						write!(formatter, "a registry id of type {}", std::any::type_name::<T>())
					}

					fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
					where
						E: serde::de::Error,
					{
						self.0.get(v).copied().ok_or_else(|| E::invalid_value(Unexpected::Str(v), &self))
					}
				}

				while let Some((k, v)) = map.next_entry_seed(KeyIdSeed(self.0), PhantomData)?
				{
					let entry = &mut self.1[k.index()];
					entry.replace(v);
				}

				Ok(())
			}
		}

		deserializer.deserialize_map(RegistryValueMapVisitor(self.0, self.1))
	}
}
