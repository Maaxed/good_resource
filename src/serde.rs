use std::fmt::Debug;
use std::fs::File;

use serde::de::DeserializeSeed;


pub trait DeserializerFactory
{
	type Error: Debug;

	const FILE_EXTENSION: &str;
	
	fn deserialize_from_file<T, S: for<'l> DeserializeSeed<'l, Value = T>>(&mut self, file: &mut File, seed: S) -> Result<T, Self::Error>;
}


#[cfg(feature = "ron")]
pub mod ron
{
	use super::DeserializerFactory;
	use ron::{ Options, de::SpannedError };

	impl DeserializerFactory for Options
	{
		type Error = SpannedError;

		const FILE_EXTENSION: &str = "ron";

		fn deserialize_from_file<T, S: for<'l> serde::de::DeserializeSeed<'l, Value = T>>(&mut self, file: &mut std::fs::File, seed: S) -> Result<T, Self::Error>
		{
			self.from_reader_seed(file, seed)
		}
	}
}

#[cfg(feature = "json")]
pub mod json
{
	use std::io::BufReader;

	use super::DeserializerFactory;
	use serde_json::{Deserializer, Error};

	pub struct Json;

	impl DeserializerFactory for Json
	{
		type Error = Error;

		const FILE_EXTENSION: &str = "ron";

		fn deserialize_from_file<T, S: for<'l> serde::de::DeserializeSeed<'l, Value = T>>(&mut self, file: &mut std::fs::File, seed: S) -> Result<T, Self::Error>
		{
			let buffer = BufReader::new(file);
			let mut de = Deserializer::from_reader(buffer);

			let value = seed.deserialize(&mut de)?;

			de.end()?;
			Ok(value)
		}
	}
}
