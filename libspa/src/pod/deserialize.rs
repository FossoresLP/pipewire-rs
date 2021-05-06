//! This module deals with deserializing raw SPA pods into rust types.
//!
//! A raw pod can be deserialized into any implementor of the [`PodDeserialize`] trait
//! by using [`PodDeserializer::deserialize_from`].
//!
//! The crate provides a number of implementors of this trait either directly,
//! or through [`FixedSizedPod`](`super::FixedSizedPod`).
//!
//! You can also implement the [`PodDeserialize`] trait on another type yourself. See the traits documentation for more
//! information on how to do that.

use std::marker::PhantomData;

use nom::{
    bytes::complete::{tag, take},
    combinator::{map, map_res, verify},
    number::{complete::u32, Endianness},
    sequence::{delimited, terminated},
    IResult,
};

use super::{CanonicalFixedSizedPod, FixedSizedPod};

/// Implementors of this trait can be deserialized from the raw SPA Pod format using a [`PodDeserializer`]-
///
/// Their [`deserialize`](`PodDeserialize::deserialize`) method should invoke exactly one of the `deserialize_*()` methods
/// of the provided [`PodDeserializer`] that fits the type that should be deserialized.
///
/// If you want to deserialize from a pod that always has the same size, implement [`super::FixedSizedPod`] instead
/// and this trait will be implemented for you automatically.
///
/// # Examples
/// Deserialize a `String` pod without copying:
/// ```rust
/// use std::io;
/// use libspa::pod::deserialize::{PodDeserialize, PodDeserializer, DeserializeSuccess};
///
/// struct ContainsStr<'s>(&'s str);
///
/// impl<'de> PodDeserialize<'de> for ContainsStr<'de> {
///     fn deserialize(
///         deserializer: PodDeserializer<'de>,
///     ) -> Result<(Self, DeserializeSuccess<'de>), nom::Err<nom::error::Error<&'de [u8]>>>
///     where
///         Self: Sized,
///     {
///         deserializer.deserialize_str().map(|(s, success)| (ContainsStr(s), success))
///     }
/// }
/// ```
/// `Bytes` pods are created in the same way, but with the `serialize_bytes` method.
///
/// Deserialize an `Array` pod with `Int` elements:
/// ```rust
/// use std::io;
/// use libspa::pod::deserialize::{PodDeserialize, PodDeserializer, DeserializeSuccess};
///
/// struct Numbers(Vec<i32>);
///
/// impl<'de> PodDeserialize<'de> for Numbers {
///     fn deserialize(
///         deserializer: PodDeserializer<'de>,
///     ) -> Result<(Self, DeserializeSuccess<'de>), nom::Err<nom::error::Error<&'de [u8]>>>
///     where
///         Self: Sized,
///     {
///         let (mut array_deserializer, num_elems) = deserializer.deserialize_array()?;
///         let mut vec = Vec::with_capacity(num_elems as usize);
///         for _ in 0..num_elems {
///             vec.push(array_deserializer.deserialize_element()?);
///         }
///         array_deserializer
///             .end()
///             .map(|success| (Numbers(vec), success))
///     }
/// }
/// ```
///
/// Make a struct deserialize from a `Struct` pod:
/// ```rust
/// use std::{convert::TryInto, io};
/// use libspa::pod::deserialize::{PodDeserialize, PodDeserializer, DeserializeSuccess};
///
/// struct Animal {
///     name: String,
///     feet: u8,
///     can_fly: bool,
/// }
///
/// impl<'de> PodDeserialize<'de> for Animal {
///     fn deserialize(
///         deserializer: PodDeserializer<'de>,
///     ) -> Result<(Self, DeserializeSuccess<'de>), nom::Err<nom::error::Error<&'de [u8]>>>
///     where
///         Self: Sized,
///     {
///         let mut struct_deserializer = deserializer.deserialize_struct()?;
///
///         let result = Animal {
///             name: struct_deserializer
///                 .deserialize_field()?
///                 .expect("Input has too few fields"),
///             feet: struct_deserializer
///                 .deserialize_field::<i32>()?
///                 .expect("Input has too few fields")
///                 .try_into()
///                 .expect("Animal is a millipede, has too many feet for a u8."),
///             can_fly: struct_deserializer
///                 .deserialize_field()?
///                 .expect("Input has too few fields"),
///         };
///
///         struct_deserializer
///             .end()
///             .map(|success| (result, success))
///     }
/// }
/// ```
pub trait PodDeserialize<'de> {
    /// Deserialize the type by using the provided [`PodDeserializer`]
    fn deserialize(
        deserializer: PodDeserializer<'de>,
    ) -> Result<(Self, DeserializeSuccess<'de>), nom::Err<nom::error::Error<&'de [u8]>>>
    where
        Self: Sized;
}

// Deserialize a `String` pod. Returned `&str` is zero-copy (is a slice of the input).
impl<'de> PodDeserialize<'de> for &'de str {
    fn deserialize(
        deserializer: PodDeserializer<'de>,
    ) -> Result<(Self, DeserializeSuccess<'de>), nom::Err<nom::error::Error<&'de [u8]>>>
    where
        Self: Sized,
    {
        deserializer.deserialize_str()
    }
}

// Deserialize a `String` pod. The returned string is an owned copy.
impl<'de> PodDeserialize<'de> for String {
    fn deserialize(
        deserializer: PodDeserializer<'de>,
    ) -> Result<(Self, DeserializeSuccess<'de>), nom::Err<nom::error::Error<&'de [u8]>>>
    where
        Self: Sized,
    {
        deserializer
            .deserialize_str()
            .map(|(s, success)| (s.to_owned(), success))
    }
}

// Deserialize a `Bytes` pod. Returned `&[u8]` is zero-copy (is a slice of the input).
impl<'de> PodDeserialize<'de> for &'de [u8] {
    fn deserialize(
        deserializer: PodDeserializer<'de>,
    ) -> Result<(Self, DeserializeSuccess<'de>), nom::Err<nom::error::Error<&'de [u8]>>>
    where
        Self: Sized,
    {
        deserializer.deserialize_bytes()
    }
}

// Deserialize a `Bytes` pod. The returned bytes array is an owned copy.
impl<'de> PodDeserialize<'de> for Vec<u8> {
    fn deserialize(
        deserializer: PodDeserializer<'de>,
    ) -> Result<(Self, DeserializeSuccess<'de>), nom::Err<nom::error::Error<&'de [u8]>>>
    where
        Self: Sized,
    {
        deserializer
            .deserialize_bytes()
            .map(|(b, success)| (b.to_owned(), success))
    }
}

// Deserialize an `Array` type pod.
impl<'de, P: FixedSizedPod> PodDeserialize<'de> for Vec<P> {
    fn deserialize(
        deserializer: PodDeserializer<'de>,
    ) -> Result<(Self, DeserializeSuccess<'de>), nom::Err<nom::error::Error<&'de [u8]>>>
    where
        Self: Sized,
    {
        let (mut arr_deserializer, num_elems) = deserializer.deserialize_array::<P>()?;

        let mut result = Vec::with_capacity(num_elems as usize);
        for _ in 0..num_elems {
            result.push(arr_deserializer.deserialize_element()?);
        }

        let success = arr_deserializer.end()?;

        Ok((result, success))
    }
}

/// This struct is returned by [`PodDeserialize`] implementors on deserialization sucess.
///
/// Because this can only be constructed by the [`PodDeserializer`], [`PodDeserialize`] implementors are forced
/// to finish deserialization of their pod instead of stopping after deserializing only part of a pod.
pub struct DeserializeSuccess<'de>(PodDeserializer<'de>);

/// This struct is responsible for deserializing a raw pod into a [`PodDeserialize`] implementor.
pub struct PodDeserializer<'de> {
    input: &'de [u8],
}

impl<'de, 'a> PodDeserializer<'de> {
    /// Deserialize a [`PodDeserialize`] implementor from a raw pod.
    ///
    /// Deserialization will only succeed if the raw pod matches the kind of pod expected by the [`PodDeserialize`]
    /// implementor.
    ///
    /// # Returns
    ///
    /// The remaining input and the type on success,
    /// or an error that specifies where parsing failed.
    #[allow(clippy::clippy::type_complexity)]
    pub fn deserialize_from<P: PodDeserialize<'de>>(
        input: &'de [u8],
    ) -> Result<(&'de [u8], P), nom::Err<nom::error::Error<&'de [u8]>>> {
        let deserializer = Self { input };
        P::deserialize(deserializer).map(|(res, success)| (success.0.input, res))
    }

    /// Execute the provide parse function, returning the parsed value or an error.
    fn parse<T, F>(&mut self, mut f: F) -> Result<T, nom::Err<nom::error::Error<&'de [u8]>>>
    where
        F: FnMut(&'de [u8]) -> IResult<&'de [u8], T>,
    {
        f(self.input).map(|(input, result)| {
            self.input = input;
            result
        })
    }

    /// Parse the size from the header and ensure it has the correct type.
    pub(super) fn header<'b>(type_: u32) -> impl FnMut(&'b [u8]) -> IResult<&'b [u8], u32> {
        terminated(u32(Endianness::Native), tag(type_.to_ne_bytes()))
    }

    /// Deserialize any fixed size pod.
    ///
    /// Deserialization will only succeed if the [`FixedSizedPod::CanonicalType`] of the requested type matches the type
    /// of the pod.
    pub fn deserialize_fixed_sized_pod<P: FixedSizedPod>(
        mut self,
    ) -> Result<(P, DeserializeSuccess<'de>), nom::Err<nom::error::Error<&'de [u8]>>> {
        let padding = if 8 - (P::CanonicalType::SIZE % 8) == 8 {
            0
        } else {
            8 - (P::CanonicalType::SIZE % 8)
        };

        self.parse(delimited(
            Self::header(P::CanonicalType::TYPE),
            map(P::CanonicalType::deserialize_body, |res| {
                P::from_canonical_type(&res)
            }),
            take(padding),
        ))
        .map(|res| (res, DeserializeSuccess(self)))
    }

    /// Deserialize a `String` pod.
    pub fn deserialize_str(
        mut self,
    ) -> Result<(&'de str, DeserializeSuccess<'de>), nom::Err<nom::error::Error<&'de [u8]>>> {
        let len = self.parse(Self::header(spa_sys::SPA_TYPE_String))?;
        let padding = (8 - len) % 8;
        self.parse(terminated(
            map_res(terminated(take(len - 1), tag([b'\0'])), std::str::from_utf8),
            take(padding),
        ))
        .map(|res| (res, DeserializeSuccess(self)))
    }

    /// Deserialize a `Bytes` pod.
    #[allow(clippy::clippy::type_complexity)]
    pub fn deserialize_bytes(
        mut self,
    ) -> Result<(&'de [u8], DeserializeSuccess<'de>), nom::Err<nom::error::Error<&'de [u8]>>> {
        let len = self.parse(Self::header(spa_sys::SPA_TYPE_Bytes))?;
        let padding = (8 - len) % 8;
        self.parse(terminated(take(len), take(padding)))
            .map(|res| (res, DeserializeSuccess(self)))
    }

    /// Start parsing an array pod containing elements of type `E`.
    ///
    /// # Returns
    /// - The array deserializer and the number of elements in the array on success
    /// - An error if the header could not be parsed
    #[allow(clippy::type_complexity)] // Return type cannot be reasonably made smaller.
    pub fn deserialize_array<E>(
        mut self,
    ) -> Result<(ArrayPodDeserializer<'de, E>, u32), nom::Err<nom::error::Error<&'de [u8]>>>
    where
        E: FixedSizedPod,
    {
        let len = self.parse(Self::header(spa_sys::SPA_TYPE_Array))?;
        self.parse(verify(Self::header(E::CanonicalType::TYPE), |len| {
            *len == E::CanonicalType::SIZE
        }))?;

        let num_elems = if E::CanonicalType::SIZE != 0 {
            (len - 8) / E::CanonicalType::SIZE
        } else {
            0
        };

        Ok((
            ArrayPodDeserializer {
                deserializer: self,
                length: num_elems,
                deserialized: 0,
                _phantom: PhantomData,
            },
            num_elems,
        ))
    }

    /// Start parsing a struct pod.
    ///
    /// # Errors
    /// Returns a parsing error if input does not start with a struct pod.
    pub fn deserialize_struct(
        mut self,
    ) -> Result<StructPodDeserializer<'de>, nom::Err<nom::error::Error<&'de [u8]>>> {
        let len = self.parse(Self::header(spa_sys::SPA_TYPE_Struct))?;

        Ok(StructPodDeserializer {
            deserializer: Some(self),
            remaining: len,
        })
    }
}

/// This struct handles deserializing arrays.
///
/// It can be obtained by calling [`PodDeserializer::deserialize_array`].
///
/// The exact number of elements that was returned from that call must be deserialized
/// using the [`deserialize_element`](`Self::deserialize_element`) function,
/// followed by calling its [`end`](`Self::end`) function to finish deserialization of the array.
pub struct ArrayPodDeserializer<'de, E: FixedSizedPod> {
    deserializer: PodDeserializer<'de>,
    // The total number of elements that must be deserialized from this array.
    length: u32,
    // The number of elements that have been deserialized so far.
    deserialized: u32,
    /// The struct has the type parameter E to ensure all deserialized elements are the same type,
    /// but doesn't actually own any E, so we need the `PhantomData<E>` instead.
    _phantom: PhantomData<E>,
}

impl<'de, E: FixedSizedPod> ArrayPodDeserializer<'de, E> {
    /// Deserialize a single element.
    ///
    /// # Panics
    /// Panics if there are no elements left to deserialize.
    pub fn deserialize_element(&mut self) -> Result<E, nom::Err<nom::error::Error<&'de [u8]>>> {
        if !self.deserialized < self.length {
            panic!("No elements left in the pod to deserialize");
        }

        let result = self
            .deserializer
            .parse(|input| E::CanonicalType::deserialize_body(input))
            .map(|res| E::from_canonical_type(&res));
        self.deserialized += 1;
        result
    }

    /// Finish deserializing the array.
    ///
    /// # Panics
    /// Panics if not all elements of the array were deserialized.
    pub fn end(
        mut self,
    ) -> Result<DeserializeSuccess<'de>, nom::Err<nom::error::Error<&'de [u8]>>> {
        assert!(
            self.length == self.deserialized,
            "Not all fields were deserialized from the struct pod"
        );

        // Deserialize remaining padding bytes.
        let bytes_read = self.deserialized * E::CanonicalType::SIZE;
        let padding = if bytes_read % 8 == 0 {
            0
        } else {
            8 - (bytes_read as usize % 8)
        };
        self.deserializer.parse(take(padding))?;

        Ok(DeserializeSuccess(self.deserializer))
    }
}

/// This struct handles deserializing structs.
///
/// It can be obtained by calling [`PodDeserializer::deserialize_struct`].
///
/// Fields of the struct must be deserialized using its [`deserialize_field`](`Self::deserialize_field`)
/// until it returns `None`.
/// followed by calling its [`end`](`Self::end`) function to finish deserialization of the struct.
pub struct StructPodDeserializer<'de> {
    /// The deserializer is saved in an option, but can be expected to always be a `Some`
    /// when `deserialize_field()` or `end()` is called.
    ///
    /// `deserialize_field()` `take()`s the deserializer, uses it to deserialize the field,
    /// and then puts the deserializer back inside.
    deserializer: Option<PodDeserializer<'de>>,
    /// Remaining struct pod body length in bytes
    remaining: u32,
}

impl<'de> StructPodDeserializer<'de> {
    /// Deserialize a single field of the struct
    ///
    /// Returns `Some` when a field was successfully deserialized and `None` when all fields have been read.
    pub fn deserialize_field<P: PodDeserialize<'de>>(
        &mut self,
    ) -> Result<Option<P>, nom::Err<nom::error::Error<&'de [u8]>>> {
        if self.remaining == 0 {
            Ok(None)
        } else {
            let deserializer = self
                .deserializer
                .take()
                .expect("StructPodDeserializer does not contain a deserializer");

            // The amount of input bytes remaining before deserializing the element.
            let remaining_input_len = deserializer.input.len();

            let (res, success) = P::deserialize(deserializer)?;

            // The amount of bytes deserialized is the length of the remaining input
            // minus the length of the remaining input now.
            self.remaining -= remaining_input_len as u32 - success.0.input.len() as u32;

            self.deserializer = Some(success.0);

            Ok(Some(res))
        }
    }

    /// Finish deserialization of the pod.
    ///
    /// # Panics
    /// Panics if not all fields of the pod have been deserialized.
    pub fn end(self) -> Result<DeserializeSuccess<'de>, nom::Err<nom::error::Error<&'de [u8]>>> {
        assert!(
            self.remaining == 0,
            "Not all fields have been deserialized from the struct"
        );

        // No padding parsing needed: Last field will already end aligned.

        Ok(DeserializeSuccess(self.deserializer.expect(
            "StructPodDeserializer does not contain a deserializer",
        )))
    }
}
