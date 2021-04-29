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

use std::{convert::Infallible, marker::PhantomData};

use nom::{
    bytes::complete::{tag, take},
    combinator::{map, map_res, verify},
    number::{complete::u32, Endianness},
    sequence::{delimited, preceded, terminated},
    IResult,
};

use super::{CanonicalFixedSizedPod, FixedSizedPod, Value, ValueArray};
use crate::utils::{Fd, Fraction, Id, Rectangle};

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
/// use libspa::pod::deserialize::{PodDeserialize, PodDeserializer, DeserializeError, DeserializeSuccess, StringVisitor};
///
/// struct ContainsStr<'s>(&'s str);
///
/// impl<'de> PodDeserialize<'de> for ContainsStr<'de> {
///     fn deserialize(
///         deserializer: PodDeserializer<'de>,
///     ) -> Result<(Self, DeserializeSuccess<'de>), DeserializeError<&'de [u8]>>
///     where
///         Self: Sized,
///     {
///         deserializer.deserialize_str(StringVisitor).map(|(s, success)| (ContainsStr(s), success))
///     }
/// }
/// ```
/// `Bytes` pods are created in the same way, but with the `serialize_bytes` method.
///
/// Deserialize an `Array` pod with `Int` elements:
/// ```rust
/// use std::io;
/// use std::io::Cursor;
/// use libspa::pod::deserialize::{PodDeserialize, PodDeserializer, DeserializeError, DeserializeSuccess, Visitor};
/// use libspa::pod::serialize::PodSerializer;
///
/// struct Numbers(Vec<i32>);
///
/// impl<'de> PodDeserialize<'de> for Numbers {
///     fn deserialize(
///         deserializer: PodDeserializer<'de>,
///     ) -> Result<(Self, DeserializeSuccess<'de>), DeserializeError<&'de [u8]>>
///     where
///         Self: Sized,
///     {
///         struct NumbersVisitor;
///
///         impl<'de> Visitor<'de> for NumbersVisitor {
///             type Value = Numbers;
///             type ArrayElem = i32;
///
///             fn visit_array(
///                 &self,
///                 elements: Vec<Self::ArrayElem>,
///             ) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
///                 Ok(Numbers(elements))
///             }
///         }
///
///         deserializer.deserialize_array(NumbersVisitor)
///     }
/// }
/// ```
///
/// Make a struct deserialize from a `Struct` pod:
/// ```rust
/// use std::{convert::TryInto, io};
/// use libspa::pod::deserialize::{PodDeserialize, PodDeserializer, DeserializeError, DeserializeSuccess, Visitor, StructPodDeserializer};
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
///     ) -> Result<(Self, DeserializeSuccess<'de>), DeserializeError<&'de [u8]>>
///     where
///         Self: Sized,
///     {
///     struct AnimalVisitor;
///
///     impl<'de> Visitor<'de> for AnimalVisitor {
///         type Value = Animal;
///         type ArrayElem = std::convert::Infallible;
///
///         fn visit_struct(
///             &self,
///             struct_deserializer: &mut StructPodDeserializer<'de>,
///         ) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
///             Ok(Animal {
///                 name: struct_deserializer
///                     .deserialize_field()?
///                     .expect("Input has too few fields"),
///                 feet: struct_deserializer
///                     .deserialize_field::<i32>()?
///                     .expect("Input has too few fields")
///                     .try_into()
///                     .expect("Animal is a millipede, has too many feet for a u8."),
///                 can_fly: struct_deserializer
///                     .deserialize_field()?
///                     .expect("Input has too few fields"),
///             })
///         }
///     }
///
///     deserializer.deserialize_struct(AnimalVisitor)
///    }
/// }
/// ```
pub trait PodDeserialize<'de> {
    /// Deserialize the type by using the provided [`PodDeserializer`]
    fn deserialize(
        deserializer: PodDeserializer<'de>,
    ) -> Result<(Self, DeserializeSuccess<'de>), DeserializeError<&'de [u8]>>
    where
        Self: Sized;
}

// Deserialize a `String` pod. Returned `&str` is zero-copy (is a slice of the input).
impl<'de> PodDeserialize<'de> for &'de str {
    fn deserialize(
        deserializer: PodDeserializer<'de>,
    ) -> Result<(Self, DeserializeSuccess<'de>), DeserializeError<&'de [u8]>>
    where
        Self: Sized,
    {
        deserializer.deserialize_str(StringVisitor)
    }
}

// Deserialize a `String` pod. The returned string is an owned copy.
impl<'de> PodDeserialize<'de> for String {
    fn deserialize(
        deserializer: PodDeserializer<'de>,
    ) -> Result<(Self, DeserializeSuccess<'de>), DeserializeError<&'de [u8]>>
    where
        Self: Sized,
    {
        deserializer
            .deserialize_str(StringVisitor)
            .map(|(s, success)| (s.to_owned(), success))
    }
}

// Deserialize a `Bytes` pod. Returned `&[u8]` is zero-copy (is a slice of the input).
impl<'de> PodDeserialize<'de> for &'de [u8] {
    fn deserialize(
        deserializer: PodDeserializer<'de>,
    ) -> Result<(Self, DeserializeSuccess<'de>), DeserializeError<&'de [u8]>>
    where
        Self: Sized,
    {
        deserializer.deserialize_bytes(BytesVisitor)
    }
}

// Deserialize a `Bytes` pod. The returned bytes array is an owned copy.
impl<'de> PodDeserialize<'de> for Vec<u8> {
    fn deserialize(
        deserializer: PodDeserializer<'de>,
    ) -> Result<(Self, DeserializeSuccess<'de>), DeserializeError<&'de [u8]>>
    where
        Self: Sized,
    {
        deserializer
            .deserialize_bytes(BytesVisitor)
            .map(|(b, success)| (b.to_owned(), success))
    }
}

// Deserialize an `Array` type pod.
impl<'de, P: FixedSizedPod + CanonicalFixedSizedPod + std::marker::Copy> PodDeserialize<'de>
    for Vec<P>
{
    fn deserialize(
        deserializer: PodDeserializer<'de>,
    ) -> Result<(Self, DeserializeSuccess<'de>), DeserializeError<&'de [u8]>>
    where
        Self: Sized,
    {
        deserializer.deserialize_array::<_, P>(VecVisitor::<P>::default())
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
    pub fn deserialize_from<P: PodDeserialize<'de>>(
        input: &'de [u8],
    ) -> Result<(&'de [u8], P), DeserializeError<&'de [u8]>> {
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

    /// Variant of [`Self::parse`] not consuming the parsed data
    fn peek<T, F>(&self, mut f: F) -> Result<T, nom::Err<nom::error::Error<&'de [u8]>>>
    where
        F: FnMut(&'de [u8]) -> IResult<&'de [u8], T>,
    {
        f(self.input).map(|(_input, result)| result)
    }

    /// Parse the size from the header and ensure it has the correct type.
    pub(super) fn header<'b>(type_: u32) -> impl FnMut(&'b [u8]) -> IResult<&'b [u8], u32> {
        terminated(u32(Endianness::Native), tag(type_.to_ne_bytes()))
    }

    /// Parse and return the type from the header
    pub(super) fn type_<'b>() -> impl FnMut(&'b [u8]) -> IResult<&'b [u8], u32> {
        preceded(u32(Endianness::Native), u32(Endianness::Native))
    }

    /// Deserialize any fixed size pod.
    ///
    /// Deserialization will only succeed if the [`FixedSizedPod::CanonicalType`] of the requested type matches the type
    /// of the pod.
    fn deserialize_fixed_sized_pod<P: FixedSizedPod>(
        mut self,
    ) -> Result<(P, DeserializeSuccess<'de>), DeserializeError<&'de [u8]>> {
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
        .map_err(|err| err.into())
    }

    /// Deserialize a `none` pod.
    pub fn deserialize_none<V>(
        self,
        visitor: V,
    ) -> Result<(V::Value, DeserializeSuccess<'de>), DeserializeError<&'de [u8]>>
    where
        V: Visitor<'de>,
    {
        let res = self.deserialize_fixed_sized_pod::<()>().unwrap();
        Ok((visitor.visit_none()?, res.1))
    }

    /// Deserialize a `boolean` pod.
    pub fn deserialize_bool<V>(
        self,
        visitor: V,
    ) -> Result<(V::Value, DeserializeSuccess<'de>), DeserializeError<&'de [u8]>>
    where
        V: Visitor<'de>,
    {
        let res = self.deserialize_fixed_sized_pod()?;
        Ok((visitor.visit_bool(res.0)?, res.1))
    }

    /// Deserialize an `int` pod.
    pub fn deserialize_int<V>(
        self,
        visitor: V,
    ) -> Result<(V::Value, DeserializeSuccess<'de>), DeserializeError<&'de [u8]>>
    where
        V: Visitor<'de>,
    {
        let res = self.deserialize_fixed_sized_pod()?;
        Ok((visitor.visit_int(res.0)?, res.1))
    }

    /// Deserialize a `long` pod.
    pub fn deserialize_long<V>(
        self,
        visitor: V,
    ) -> Result<(V::Value, DeserializeSuccess<'de>), DeserializeError<&'de [u8]>>
    where
        V: Visitor<'de>,
    {
        let res = self.deserialize_fixed_sized_pod()?;
        Ok((visitor.visit_long(res.0)?, res.1))
    }

    /// Deserialize a `float` pod.
    pub fn deserialize_float<V>(
        self,
        visitor: V,
    ) -> Result<(V::Value, DeserializeSuccess<'de>), DeserializeError<&'de [u8]>>
    where
        V: Visitor<'de>,
    {
        let res = self.deserialize_fixed_sized_pod()?;
        Ok((visitor.visit_float(res.0)?, res.1))
    }

    /// Deserialize a `double` pod.
    pub fn deserialize_double<V>(
        self,
        visitor: V,
    ) -> Result<(V::Value, DeserializeSuccess<'de>), DeserializeError<&'de [u8]>>
    where
        V: Visitor<'de>,
    {
        let res = self.deserialize_fixed_sized_pod()?;
        Ok((visitor.visit_double(res.0)?, res.1))
    }

    /// Deserialize a `String` pod.
    pub fn deserialize_str<V>(
        mut self,
        visitor: V,
    ) -> Result<(V::Value, DeserializeSuccess<'de>), DeserializeError<&'de [u8]>>
    where
        V: Visitor<'de>,
    {
        let len = self.parse(Self::header(spa_sys::SPA_TYPE_String))?;
        let padding = (8 - len) % 8;
        let res = self.parse(terminated(
            map_res(terminated(take(len - 1), tag([b'\0'])), std::str::from_utf8),
            take(padding),
        ))?;
        Ok((visitor.visit_string(res)?, DeserializeSuccess(self)))
    }

    /// Deserialize a `Bytes` pod.
    pub fn deserialize_bytes<V>(
        mut self,
        visitor: V,
    ) -> Result<(V::Value, DeserializeSuccess<'de>), DeserializeError<&'de [u8]>>
    where
        V: Visitor<'de>,
    {
        let len = self.parse(Self::header(spa_sys::SPA_TYPE_Bytes))?;
        let padding = (8 - len) % 8;
        let res = self.parse(terminated(take(len), take(padding)))?;
        Ok((visitor.visit_bytes(res)?, DeserializeSuccess(self)))
    }

    /// Start parsing an array pod containing elements of type `E`.
    ///
    /// # Returns
    /// - The array deserializer and the number of elements in the array on success
    /// - An error if the header could not be parsed
    pub fn new_array_deserializer<E>(
        mut self,
    ) -> Result<(ArrayPodDeserializer<'de, E>, u32), DeserializeError<&'de [u8]>>
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
    fn new_struct_deserializer(
        mut self,
    ) -> Result<StructPodDeserializer<'de>, DeserializeError<&'de [u8]>> {
        let len = self.parse(Self::header(spa_sys::SPA_TYPE_Struct))?;

        Ok(StructPodDeserializer {
            deserializer: Some(self),
            remaining: len,
        })
    }

    /// Deserialize a `Rectangle` pod.
    pub fn deserialize_rectangle<V>(
        self,
        visitor: V,
    ) -> Result<(V::Value, DeserializeSuccess<'de>), DeserializeError<&'de [u8]>>
    where
        V: Visitor<'de>,
    {
        let res = self.deserialize_fixed_sized_pod()?;
        Ok((visitor.visit_rectangle(res.0)?, res.1))
    }

    /// Deserialize a `Fraction` pod.
    pub fn deserialize_fraction<V>(
        self,
        visitor: V,
    ) -> Result<(V::Value, DeserializeSuccess<'de>), DeserializeError<&'de [u8]>>
    where
        V: Visitor<'de>,
    {
        let res = self.deserialize_fixed_sized_pod()?;
        Ok((visitor.visit_fraction(res.0)?, res.1))
    }

    /// Deserialize an `Id` pod.
    pub fn deserialize_id<V>(
        self,
        visitor: V,
    ) -> Result<(V::Value, DeserializeSuccess<'de>), DeserializeError<&'de [u8]>>
    where
        V: Visitor<'de>,
    {
        let res = self.deserialize_fixed_sized_pod()?;
        Ok((visitor.visit_id(res.0)?, res.1))
    }

    /// Deserialize a `Fd` pod.
    pub fn deserialize_fd<V>(
        self,
        visitor: V,
    ) -> Result<(V::Value, DeserializeSuccess<'de>), DeserializeError<&'de [u8]>>
    where
        V: Visitor<'de>,
    {
        let res = self.deserialize_fixed_sized_pod()?;
        Ok((visitor.visit_fd(res.0)?, res.1))
    }

    /// Deserialize a `Struct` pod.
    pub fn deserialize_struct<V>(
        self,
        visitor: V,
    ) -> Result<(V::Value, DeserializeSuccess<'de>), DeserializeError<&'de [u8]>>
    where
        V: Visitor<'de>,
    {
        let mut struct_deserializer = self.new_struct_deserializer()?;
        let res = visitor.visit_struct(&mut struct_deserializer)?;
        let success = struct_deserializer.end()?;
        Ok((res, success))
    }

    fn deserialize_array_vec<T>(
        self,
    ) -> Result<(Vec<T>, DeserializeSuccess<'de>), DeserializeError<&'de [u8]>>
    where
        T: CanonicalFixedSizedPod + FixedSizedPod + std::marker::Copy,
    {
        let mut array_deserializer: ArrayPodDeserializer<'de, T> = self.new_array_deserializer()?.0;
        let mut elements = Vec::with_capacity(array_deserializer.length as usize);
        for _ in 0..array_deserializer.length {
            elements.push(array_deserializer.deserialize_element()?);
        }
        let success = array_deserializer.end()?;

        Ok((elements, success))
    }

    /// Deserialize an `array` pod containing elments of type `T`.
    pub fn deserialize_array<V, T>(
        self,
        visitor: V,
    ) -> Result<(V::Value, DeserializeSuccess<'de>), DeserializeError<&'de [u8]>>
    where
        V: Visitor<'de, ArrayElem = T>,
        T: CanonicalFixedSizedPod + FixedSizedPod + std::marker::Copy,
    {
        let (elements, success) = self.deserialize_array_vec::<T>()?;
        let res = visitor.visit_array(elements)?;
        Ok((res, success))
    }

    /// Deserialize any kind of pod using a visitor producing [`Value`].
    pub fn deserialize_any(
        self,
    ) -> Result<(Value, DeserializeSuccess<'de>), DeserializeError<&'de [u8]>> {
        let type_ = self.peek(Self::type_())?;

        match type_ {
            spa_sys::SPA_TYPE_None => self.deserialize_none(ValueVisitor),
            spa_sys::SPA_TYPE_Bool => self.deserialize_bool(ValueVisitor),
            spa_sys::SPA_TYPE_Id => self.deserialize_id(ValueVisitor),
            spa_sys::SPA_TYPE_Int => self.deserialize_int(ValueVisitor),
            spa_sys::SPA_TYPE_Long => self.deserialize_long(ValueVisitor),
            spa_sys::SPA_TYPE_Float => self.deserialize_float(ValueVisitor),
            spa_sys::SPA_TYPE_Double => self.deserialize_double(ValueVisitor),
            spa_sys::SPA_TYPE_String => self.deserialize_str(ValueVisitor),
            spa_sys::SPA_TYPE_Bytes => self.deserialize_bytes(ValueVisitor),
            spa_sys::SPA_TYPE_Rectangle => self.deserialize_rectangle(ValueVisitor),
            spa_sys::SPA_TYPE_Fraction => self.deserialize_fraction(ValueVisitor),
            spa_sys::SPA_TYPE_Fd => self.deserialize_fd(ValueVisitor),
            spa_sys::SPA_TYPE_Struct => self.deserialize_struct(ValueVisitor),
            spa_sys::SPA_TYPE_Array => self.deserialize_array_any(),
            _ => Err(DeserializeError::InvalidType),
        }
    }

    fn deserialize_array_any(
        self,
    ) -> Result<(Value, DeserializeSuccess<'de>), DeserializeError<&'de [u8]>> {
        let child_type = self.peek(preceded(Self::type_(), Self::type_()))?;

        let (array, success) = match child_type {
            spa_sys::SPA_TYPE_None => {
                let (elements, success) = self.deserialize_array_vec::<()>()?;
                let array = ValueArrayNoneVisitor.visit_array(elements)?;
                (array, success)
            }
            spa_sys::SPA_TYPE_Bool => {
                let (elements, success) = self.deserialize_array_vec::<bool>()?;
                let array = ValueArrayBoolVisitor.visit_array(elements)?;
                (array, success)
            }
            spa_sys::SPA_TYPE_Id => {
                let (elements, success) = self.deserialize_array_vec::<Id>()?;
                let array = ValueArrayIdVisitor.visit_array(elements)?;
                (array, success)
            }
            spa_sys::SPA_TYPE_Int => {
                let (elements, success) = self.deserialize_array_vec::<i32>()?;
                let array = ValueArrayIntVisitor.visit_array(elements)?;
                (array, success)
            }
            spa_sys::SPA_TYPE_Long => {
                let (elements, success) = self.deserialize_array_vec::<i64>()?;
                let array = ValueArrayLongVisitor.visit_array(elements)?;
                (array, success)
            }
            spa_sys::SPA_TYPE_Float => {
                let (elements, success) = self.deserialize_array_vec::<f32>()?;
                let array = ValueArrayFloatVisitor.visit_array(elements)?;
                (array, success)
            }
            spa_sys::SPA_TYPE_Double => {
                let (elements, success) = self.deserialize_array_vec::<f64>()?;
                let array = ValueArrayDoubleVisitor.visit_array(elements)?;
                (array, success)
            }
            spa_sys::SPA_TYPE_Rectangle => {
                let (elements, success) = self.deserialize_array_vec::<Rectangle>()?;
                let array = ValueArrayRectangleVisitor.visit_array(elements)?;
                (array, success)
            }
            spa_sys::SPA_TYPE_Fraction => {
                let (elements, success) = self.deserialize_array_vec::<Fraction>()?;
                let array = ValueArrayFractionVisitor.visit_array(elements)?;
                (array, success)
            }
            spa_sys::SPA_TYPE_Fd => {
                let (elements, success) = self.deserialize_array_vec::<Fd>()?;
                let array = ValueArrayFdVisitor.visit_array(elements)?;
                (array, success)
            }
            _ => return Err(DeserializeError::InvalidType),
        };

        Ok((Value::ValueArray(array), success))
    }

    /// Variant of [`Self::deserialize_from`] returning the parsed value as a [`Value`].
    pub fn deserialize_any_from(
        input: &'de [u8],
    ) -> Result<(&'de [u8], Value), DeserializeError<&'de [u8]>> {
        Self::deserialize_from(input)
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
    pub fn deserialize_element(&mut self) -> Result<E, DeserializeError<&'de [u8]>> {
        if !self.deserialized < self.length {
            panic!("No elements left in the pod to deserialize");
        }

        let result = self
            .deserializer
            .parse(|input| E::CanonicalType::deserialize_body(input))
            .map(|res| E::from_canonical_type(&res))
            .map_err(|err| err.into());

        self.deserialized += 1;
        result
    }

    /// Finish deserializing the array.
    ///
    /// # Panics
    /// Panics if not all elements of the array were deserialized.
    pub fn end(mut self) -> Result<DeserializeSuccess<'de>, DeserializeError<&'de [u8]>> {
        assert!(
            self.length == self.deserialized,
            "Not all fields were deserialized from the array pod"
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
    ) -> Result<Option<P>, DeserializeError<&'de [u8]>> {
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
    pub fn end(self) -> Result<DeserializeSuccess<'de>, DeserializeError<&'de [u8]>> {
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

#[derive(Debug, PartialEq)]
/// Represent an error raised when deserializing a pod
pub enum DeserializeError<I> {
    /// Parsing error
    Nom(nom::Err<nom::error::Error<I>>),
    /// The visitor does not support the type
    UnsupportedType,
    /// The type is either invalid or not yet supported
    InvalidType,
}

impl<I> From<nom::Err<nom::error::Error<I>>> for DeserializeError<I> {
    fn from(err: nom::Err<nom::error::Error<I>>) -> Self {
        DeserializeError::Nom(err)
    }
}

/// This trait represents a visitor is "driven" by the deserializer to construct an instance of your type.
pub trait Visitor<'de>: Sized {
    /// The value produced by this visitor
    type Value;
    /// The element type [`Visitor::visit_array`] is expecting as input.
    /// Only used for visitors implementing this method,
    /// [`std::convert::Infallible`] can be used as a default.
    type ArrayElem;

    /// The input contains a `none`.
    fn visit_none(&self) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Err(DeserializeError::UnsupportedType)
    }

    /// The input contains a `bool`.
    fn visit_bool(&self, _v: bool) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Err(DeserializeError::UnsupportedType)
    }

    /// The input contains an `i32`.
    fn visit_int(&self, _v: i32) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Err(DeserializeError::UnsupportedType)
    }

    /// The input contains an `i64`.
    fn visit_long(&self, _v: i64) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Err(DeserializeError::UnsupportedType)
    }

    /// The input contains an `f32`.
    fn visit_float(&self, _v: f32) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Err(DeserializeError::UnsupportedType)
    }

    /// The input contains an `f64`.
    fn visit_double(&self, _v: f64) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Err(DeserializeError::UnsupportedType)
    }

    /// The input contains a string.
    fn visit_string(&self, _v: &'de str) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Err(DeserializeError::UnsupportedType)
    }

    /// The input contains a bytes array.
    fn visit_bytes(&self, _v: &'de [u8]) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Err(DeserializeError::UnsupportedType)
    }

    /// The input contains a [`Rectangle`].
    fn visit_rectangle(&self, _v: Rectangle) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Err(DeserializeError::UnsupportedType)
    }

    /// The input contains a [`Fraction`].
    fn visit_fraction(&self, _v: Fraction) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Err(DeserializeError::UnsupportedType)
    }

    /// The input contains an [`Id`].
    fn visit_id(&self, _v: Id) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Err(DeserializeError::UnsupportedType)
    }

    /// The input contains an [`Fd`].
    fn visit_fd(&self, _v: Fd) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Err(DeserializeError::UnsupportedType)
    }

    /// The input contains a structure.
    fn visit_struct(
        &self,
        _struct_deserializer: &mut StructPodDeserializer<'de>,
    ) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Err(DeserializeError::UnsupportedType)
    }

    /// The input contains an array.
    fn visit_array(
        &self,
        _elements: Vec<Self::ArrayElem>,
    ) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Err(DeserializeError::UnsupportedType)
    }
}

/// A visitor producing `()` for none values.
pub struct NoneVisitor;

impl<'de> Visitor<'de> for NoneVisitor {
    type Value = ();
    type ArrayElem = Infallible;

    fn visit_none(&self) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Ok(())
    }
}

/// A visitor producing [`bool`] for boolean values.
pub struct BoolVisitor;

impl<'de> Visitor<'de> for BoolVisitor {
    type Value = bool;
    type ArrayElem = Infallible;

    fn visit_bool(&self, v: bool) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Ok(v)
    }
}

/// A visitor producing [`i32`] for integer values.
pub struct IntVisitor;

impl<'de> Visitor<'de> for IntVisitor {
    type Value = i32;
    type ArrayElem = Infallible;

    fn visit_int(&self, v: i32) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Ok(v)
    }
}

/// A visitor producing [`i64`] for long values.
pub struct LongVisitor;

impl<'de> Visitor<'de> for LongVisitor {
    type Value = i64;
    type ArrayElem = Infallible;

    fn visit_long(&self, v: i64) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Ok(v)
    }
}

/// A visitor producing [`f32`] for float values.
pub struct FloatVisitor;

impl<'de> Visitor<'de> for FloatVisitor {
    type Value = f32;
    type ArrayElem = Infallible;

    fn visit_float(&self, v: f32) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Ok(v)
    }
}

/// A visitor producing [`f64`] for double values.
pub struct DoubleVisitor;

impl<'de> Visitor<'de> for DoubleVisitor {
    type Value = f64;
    type ArrayElem = Infallible;

    fn visit_double(&self, v: f64) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Ok(v)
    }
}

/// A visitor producing [`&str`] for string values.
pub struct StringVisitor;

impl<'de> Visitor<'de> for StringVisitor {
    type Value = &'de str;
    type ArrayElem = Infallible;

    fn visit_string(&self, v: &'de str) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Ok(v)
    }
}

/// A visitor producing [`&[u8]`] for bytes values.
pub struct BytesVisitor;

impl<'de> Visitor<'de> for BytesVisitor {
    type Value = &'de [u8];
    type ArrayElem = Infallible;

    fn visit_bytes(&self, v: &'de [u8]) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Ok(v)
    }
}

/// A visitor producing [`Rectangle`] for rectangle values.
pub struct RectangleVisitor;

impl<'de> Visitor<'de> for RectangleVisitor {
    type Value = Rectangle;
    type ArrayElem = Infallible;

    fn visit_rectangle(&self, v: Rectangle) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Ok(v)
    }
}

/// A visitor producing [`Fraction`] for fraction values.
pub struct FractionVisitor;

impl<'de> Visitor<'de> for FractionVisitor {
    type Value = Fraction;
    type ArrayElem = Infallible;

    fn visit_fraction(&self, v: Fraction) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Ok(v)
    }
}

/// A visitor producing [`Id`] for ID values.
pub struct IdVisitor;

impl<'de> Visitor<'de> for IdVisitor {
    type Value = Id;
    type ArrayElem = Infallible;

    fn visit_id(&self, v: Id) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Ok(v)
    }
}

/// A visitor producing [`Fd`] for file descriptor values.
pub struct FdVisitor;

impl<'de> Visitor<'de> for FdVisitor {
    type Value = Fd;
    type ArrayElem = Infallible;

    fn visit_fd(&self, v: Fd) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Ok(v)
    }
}
/// A visitor producing [`Vec`] for array values.
pub struct VecVisitor<E: FixedSizedPod> {
    _phantom: PhantomData<E>,
}

impl<E: FixedSizedPod> Default for VecVisitor<E> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<'de, E: CanonicalFixedSizedPod + std::marker::Copy> Visitor<'de> for VecVisitor<E> {
    type Value = Vec<E>;
    type ArrayElem = E;

    fn visit_array(&self, elements: Vec<E>) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Ok(elements)
    }
}

/// A visitor producing [`Value`] for all type of values.
pub struct ValueVisitor;

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;
    type ArrayElem = std::convert::Infallible;

    fn visit_none(&self) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Ok(Value::None)
    }

    fn visit_bool(&self, v: bool) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Ok(Value::Bool(v))
    }

    fn visit_int(&self, v: i32) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Ok(Value::Int(v))
    }

    fn visit_long(&self, v: i64) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Ok(Value::Long(v))
    }

    fn visit_float(&self, v: f32) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Ok(Value::Float(v))
    }

    fn visit_double(&self, v: f64) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Ok(Value::Double(v))
    }

    fn visit_string(&self, v: &'de str) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Ok(Value::String(v.to_string()))
    }

    fn visit_bytes(&self, v: &'de [u8]) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Ok(Value::Bytes(v.to_vec()))
    }

    fn visit_rectangle(&self, v: Rectangle) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Ok(Value::Rectangle(v))
    }

    fn visit_fraction(&self, v: Fraction) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Ok(Value::Fraction(v))
    }

    fn visit_id(&self, v: Id) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Ok(Value::Id(v))
    }

    fn visit_fd(&self, v: Fd) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Ok(Value::Fd(v))
    }

    fn visit_struct(
        &self,
        struct_deserializer: &mut StructPodDeserializer<'de>,
    ) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        let mut res = Vec::new();

        while let Some(value) = struct_deserializer.deserialize_field()? {
            res.push(value);
        }

        Ok(Value::Struct(res))
    }
}

struct ValueArrayNoneVisitor;

impl<'de> Visitor<'de> for ValueArrayNoneVisitor {
    type Value = ValueArray;
    type ArrayElem = ();

    fn visit_array(
        &self,
        elements: Vec<Self::ArrayElem>,
    ) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Ok(ValueArray::None(elements))
    }
}

struct ValueArrayBoolVisitor;

impl<'de> Visitor<'de> for ValueArrayBoolVisitor {
    type Value = ValueArray;
    type ArrayElem = bool;

    fn visit_array(
        &self,
        elements: Vec<Self::ArrayElem>,
    ) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Ok(ValueArray::Bool(elements))
    }
}

struct ValueArrayIdVisitor;

impl<'de> Visitor<'de> for ValueArrayIdVisitor {
    type Value = ValueArray;
    type ArrayElem = Id;

    fn visit_array(
        &self,
        elements: Vec<Self::ArrayElem>,
    ) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Ok(ValueArray::Id(elements))
    }
}

struct ValueArrayIntVisitor;

impl<'de> Visitor<'de> for ValueArrayIntVisitor {
    type Value = ValueArray;
    type ArrayElem = i32;

    fn visit_array(
        &self,
        elements: Vec<Self::ArrayElem>,
    ) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Ok(ValueArray::Int(elements))
    }
}

struct ValueArrayLongVisitor;

impl<'de> Visitor<'de> for ValueArrayLongVisitor {
    type Value = ValueArray;
    type ArrayElem = i64;

    fn visit_array(
        &self,
        elements: Vec<Self::ArrayElem>,
    ) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Ok(ValueArray::Long(elements))
    }
}

struct ValueArrayFloatVisitor;

impl<'de> Visitor<'de> for ValueArrayFloatVisitor {
    type Value = ValueArray;
    type ArrayElem = f32;

    fn visit_array(
        &self,
        elements: Vec<Self::ArrayElem>,
    ) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Ok(ValueArray::Float(elements))
    }
}

struct ValueArrayDoubleVisitor;

impl<'de> Visitor<'de> for ValueArrayDoubleVisitor {
    type Value = ValueArray;
    type ArrayElem = f64;

    fn visit_array(
        &self,
        elements: Vec<Self::ArrayElem>,
    ) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Ok(ValueArray::Double(elements))
    }
}

struct ValueArrayRectangleVisitor;

impl<'de> Visitor<'de> for ValueArrayRectangleVisitor {
    type Value = ValueArray;
    type ArrayElem = Rectangle;

    fn visit_array(
        &self,
        elements: Vec<Self::ArrayElem>,
    ) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Ok(ValueArray::Rectangle(elements))
    }
}

struct ValueArrayFractionVisitor;

impl<'de> Visitor<'de> for ValueArrayFractionVisitor {
    type Value = ValueArray;
    type ArrayElem = Fraction;

    fn visit_array(
        &self,
        elements: Vec<Self::ArrayElem>,
    ) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Ok(ValueArray::Fraction(elements))
    }
}

struct ValueArrayFdVisitor;

impl<'de> Visitor<'de> for ValueArrayFdVisitor {
    type Value = ValueArray;
    type ArrayElem = Fd;

    fn visit_array(
        &self,
        elements: Vec<Self::ArrayElem>,
    ) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
        Ok(ValueArray::Fd(elements))
    }
}
