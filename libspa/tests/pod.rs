use libspa::{
    pod::deserialize::PodDeserializer,
    pod::{
        deserialize::{
            DeserializeError, DeserializeSuccess, PodDeserialize, StructPodDeserializer, Visitor,
        },
        serialize::{PodSerialize, PodSerializer, SerializeSuccess},
        CanonicalFixedSizedPod,
    },
    utils::{Fd, Fraction, Id, Rectangle},
};
use std::{ffi::CString, io::Cursor};

pub mod c {
    #[allow(non_camel_case_types)]
    type spa_pod = u8;

    #[link(name = "pod")]
    extern "C" {
        pub fn build_none(buffer: *mut u8, len: usize) -> i32;
        pub fn build_bool(buffer: *mut u8, len: usize, boolean: i32) -> i32;
        pub fn build_id(buffer: *mut u8, len: usize, id: u32) -> i32;
        pub fn build_int(buffer: *mut u8, len: usize, int: i32) -> i32;
        pub fn build_long(buffer: *mut u8, len: usize, long: i64) -> i32;
        pub fn build_float(buffer: *mut u8, len: usize, float: f32) -> i32;
        pub fn build_double(buffer: *mut u8, len: usize, float: f64) -> i32;
        pub fn build_string(buffer: *mut u8, len: usize, string: *const u8) -> i32;
        pub fn build_bytes(buffer: *mut u8, len: usize, bytes: *const u8, len: usize) -> i32;
        pub fn build_rectangle(buffer: *mut u8, len: usize, width: u32, height: u32) -> i32;
        pub fn build_fraction(buffer: *mut u8, len: usize, num: u32, denom: u32) -> i32;
        pub fn build_array(
            buffer: *mut u8,
            len: usize,
            child_size: u32,
            child_type: u32,
            n_elems: u32,
            elems: *const u8,
        ) -> i32;
        pub fn build_test_struct(
            buffer: *mut u8,
            len: usize,
            int: i32,
            string: *const u8,
            rect_width: u32,
            rect_height: u32,
        ) -> *const spa_pod;
        pub fn build_fd(buffer: *mut u8, len: usize, fd: i64) -> i32;
        pub fn print_pod(pod: *const spa_pod);
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn none() {
    let vec_rs: Vec<u8> = PodSerializer::serialize(Cursor::new(Vec::new()), &())
        .unwrap()
        .0
        .into_inner();
    let mut vec_c: Vec<u8> = vec![0; 8];
    assert_eq!(unsafe { c::build_none(vec_c.as_mut_ptr(), vec_c.len()) }, 0);

    assert_eq!(vec_rs, vec_c);

    assert_eq!(
        PodDeserializer::deserialize_from(&vec_rs),
        Ok((&[] as &[u8], ()))
    );
}

#[test]
#[cfg_attr(miri, ignore)]
fn bool_true() {
    let vec_rs: Vec<u8> = PodSerializer::serialize(Cursor::new(Vec::new()), &true)
        .unwrap()
        .0
        .into_inner();
    let mut vec_c: Vec<u8> = vec![0; 16];
    assert_eq!(
        unsafe { c::build_bool(vec_c.as_mut_ptr(), vec_c.len(), 1) },
        0
    );
    assert_eq!(vec_rs, vec_c);

    assert_eq!(
        PodDeserializer::deserialize_from(&vec_rs),
        Ok((&[] as &[u8], true))
    );
}

#[test]
#[cfg_attr(miri, ignore)]
fn bool_false() {
    let vec_rs: Vec<u8> = PodSerializer::serialize(Cursor::new(Vec::new()), &false)
        .unwrap()
        .0
        .into_inner();
    let mut vec_c: Vec<u8> = vec![0; 16];
    assert_eq!(
        unsafe { c::build_bool(vec_c.as_mut_ptr(), vec_c.len(), 0) },
        0
    );

    assert_eq!(vec_rs, vec_c);

    assert_eq!(
        PodDeserializer::deserialize_from(&vec_rs),
        Ok((&[] as &[u8], false))
    );
}

#[test]
#[cfg_attr(miri, ignore)]
fn int() {
    let int: i32 = 765;

    let vec_rs: Vec<u8> = PodSerializer::serialize(Cursor::new(Vec::new()), &int)
        .unwrap()
        .0
        .into_inner();
    let mut vec_c: Vec<u8> = vec![0; 16];
    assert_eq!(
        unsafe { c::build_int(vec_c.as_mut_ptr(), vec_c.len(), int) },
        0
    );

    assert_eq!(vec_rs, vec_c);

    assert_eq!(
        PodDeserializer::deserialize_from(&vec_rs),
        Ok((&[] as &[u8], int))
    );
}

#[test]
#[cfg_attr(miri, ignore)]
fn long() {
    let long: i64 = 0x1234_5678_9876_5432;

    let vec_rs: Vec<u8> = PodSerializer::serialize(Cursor::new(Vec::new()), &long)
        .unwrap()
        .0
        .into_inner();
    let mut vec_c: Vec<u8> = vec![0; 16];
    assert_eq!(
        unsafe { c::build_long(vec_c.as_mut_ptr(), vec_c.len(), long) },
        0
    );

    assert_eq!(vec_rs, vec_c);

    assert_eq!(
        PodDeserializer::deserialize_from(&vec_rs),
        Ok((&[] as &[u8], long))
    );
}

#[test]
#[cfg_attr(miri, ignore)]
fn float() {
    let float: f32 = std::f32::consts::PI;

    let vec_rs: Vec<u8> = PodSerializer::serialize(Cursor::new(Vec::new()), &float)
        .unwrap()
        .0
        .into_inner();
    let mut vec_c: Vec<u8> = vec![0; 16];
    assert_eq!(
        unsafe { c::build_float(vec_c.as_mut_ptr(), vec_c.len(), float) },
        0
    );

    assert_eq!(vec_rs, vec_c);

    assert_eq!(
        PodDeserializer::deserialize_from(&vec_rs),
        Ok((&[] as &[u8], float))
    );
}

#[test]
#[cfg_attr(miri, ignore)]
fn double() {
    let double: f64 = std::f64::consts::PI;

    let vec_rs: Vec<u8> = PodSerializer::serialize(Cursor::new(Vec::new()), &double)
        .unwrap()
        .0
        .into_inner();
    let mut vec_c: Vec<u8> = vec![0; 16];
    assert_eq!(
        unsafe { c::build_double(vec_c.as_mut_ptr(), vec_c.len(), double) },
        0
    );

    assert_eq!(vec_rs, vec_c);

    assert_eq!(
        PodDeserializer::deserialize_from(&vec_rs),
        Ok((&[] as &[u8], double))
    );
}

#[test]
#[cfg_attr(miri, ignore)]
fn string() {
    let string = "foo";

    let vec_rs: Vec<u8> = PodSerializer::serialize(Cursor::new(Vec::new()), string)
        .unwrap()
        .0
        .into_inner();
    let mut vec_c: Vec<u8> = vec![0; 16];
    let c_string = CString::new(string).unwrap();
    assert_eq!(
        unsafe {
            c::build_string(
                vec_c.as_mut_ptr(),
                vec_c.len(),
                c_string.as_bytes_with_nul().as_ptr(),
            )
        },
        0
    );
    assert_eq!(vec_rs, vec_c);

    // Zero-copy deserializing.
    assert_eq!(
        PodDeserializer::deserialize_from(&vec_rs),
        Ok((&[] as &[u8], string))
    );

    // Deserializing by copying.
    assert_eq!(
        PodDeserializer::deserialize_from(&vec_rs),
        Ok((&[] as &[u8], String::from(string)))
    );
}

#[test]
#[cfg_attr(miri, ignore)]
fn string_no_padding() {
    // The pod resulting from this string should have no padding bytes.
    let string = "1234567";

    let vec_rs: Vec<u8> = PodSerializer::serialize(Cursor::new(Vec::new()), string)
        .unwrap()
        .0
        .into_inner();
    let mut vec_c: Vec<u8> = vec![0; 16];
    let c_string = CString::new(string).unwrap();
    assert_eq!(
        unsafe {
            c::build_string(
                vec_c.as_mut_ptr(),
                vec_c.len(),
                c_string.as_bytes_with_nul().as_ptr(),
            )
        },
        0
    );
    assert_eq!(vec_rs, vec_c);

    // Zero-copy deserializing.
    assert_eq!(
        PodDeserializer::deserialize_from(&vec_rs),
        Ok((&[] as &[u8], string))
    );

    // Deserializing by copying.
    assert_eq!(
        PodDeserializer::deserialize_from(&vec_rs),
        Ok((&[] as &[u8], String::from(string)))
    );
}

#[test]
#[cfg_attr(miri, ignore)]
fn string_empty() {
    let string = "";

    let mut vec_c: Vec<u8> = vec![0; 16];
    let vec_rs: Vec<u8> = PodSerializer::serialize(Cursor::new(Vec::new()), string)
        .unwrap()
        .0
        .into_inner();
    let c_string = CString::new(string).unwrap();
    assert_eq!(
        unsafe {
            c::build_string(
                vec_c.as_mut_ptr(),
                vec_c.len(),
                c_string.as_bytes_with_nul().as_ptr(),
            )
        },
        0
    );
    assert_eq!(vec_rs, vec_c);

    // Zero-copy deserializing.
    assert_eq!(
        PodDeserializer::deserialize_from(&vec_rs),
        Ok((&[] as &[u8], string))
    );

    // Deserializing by copying.
    assert_eq!(
        PodDeserializer::deserialize_from(&vec_rs),
        Ok((&[] as &[u8], String::from(string)))
    );
}

#[test]
#[cfg_attr(miri, ignore)]
fn bytes() {
    let bytes = b"foo";

    let vec_rs: Vec<u8> = PodSerializer::serialize(Cursor::new(Vec::new()), bytes as &[u8])
        .unwrap()
        .0
        .into_inner();
    let mut vec_c: Vec<u8> = vec![0; 16];
    assert_eq!(
        unsafe { c::build_bytes(vec_c.as_mut_ptr(), vec_c.len(), bytes.as_ptr(), bytes.len()) },
        0
    );
    assert_eq!(vec_rs, vec_c);

    // Zero-copy deserializing.
    assert_eq!(
        PodDeserializer::deserialize_from(&vec_rs),
        Ok((&[] as &[u8], bytes as &[u8]))
    );

    // Deserializing by copying.
    assert_eq!(
        PodDeserializer::deserialize_from(&vec_rs),
        Ok((&[] as &[u8], Vec::from(bytes as &[u8])))
    );
}

#[test]
#[cfg_attr(miri, ignore)]
fn bytes_no_padding() {
    // The pod resulting from this byte array should have no padding bytes.
    let bytes = b"12345678";

    let vec_rs: Vec<u8> = PodSerializer::serialize(Cursor::new(Vec::new()), bytes as &[u8])
        .unwrap()
        .0
        .into_inner();
    let mut vec_c = vec![0; 16];
    assert_eq!(
        unsafe { c::build_bytes(vec_c.as_mut_ptr(), vec_c.len(), bytes.as_ptr(), bytes.len()) },
        0
    );
    assert_eq!(vec_rs, vec_c);

    // Zero-copy deserializing.
    assert_eq!(
        PodDeserializer::deserialize_from(&vec_rs),
        Ok((&[] as &[u8], bytes as &[u8]))
    );

    // Deserializing by copying.
    assert_eq!(
        PodDeserializer::deserialize_from(&vec_rs),
        Ok((&[] as &[u8], Vec::from(bytes as &[u8])))
    );
}

#[test]
#[cfg_attr(miri, ignore)]
fn bytes_empty() {
    let bytes = b"";

    let mut vec_c: Vec<u8> = vec![0; 8];
    let vec_rs: Vec<u8> = PodSerializer::serialize(Cursor::new(Vec::new()), bytes as &[u8])
        .unwrap()
        .0
        .into_inner();
    assert_eq!(
        unsafe { c::build_bytes(vec_c.as_mut_ptr(), vec_c.len(), bytes.as_ptr(), bytes.len()) },
        0
    );
    assert_eq!(vec_rs, vec_c);

    // Zero-copy deserializing.
    assert_eq!(
        PodDeserializer::deserialize_from(&vec_rs),
        Ok((&[] as &[u8], bytes as &[u8]))
    );

    // Deserializing by copying.
    assert_eq!(
        PodDeserializer::deserialize_from(&vec_rs),
        Ok((&[] as &[u8], Vec::from(bytes as &[u8])))
    );
}

#[test]
#[cfg_attr(miri, ignore)]
fn rectangle() {
    let rect = Rectangle {
        width: 640,
        height: 480,
    };

    let vec_rs = PodSerializer::serialize(Cursor::new(Vec::new()), &rect)
        .unwrap()
        .0
        .into_inner();
    let mut vec_c: Vec<u8> = vec![0; 16];
    unsafe { c::build_rectangle(vec_c.as_mut_ptr(), vec_c.len(), rect.width, rect.height) };
    assert_eq!(vec_rs, vec_c);

    assert_eq!(
        PodDeserializer::deserialize_from(&vec_rs),
        Ok((&[] as &[u8], rect))
    );
}

#[test]
#[cfg_attr(miri, ignore)]
fn fraction() {
    let fraction = Fraction { num: 16, denom: 9 };

    let vec_rs = PodSerializer::serialize(Cursor::new(Vec::new()), &fraction)
        .unwrap()
        .0
        .into_inner();
    let mut vec_c: Vec<u8> = vec![0; 16];
    unsafe {
        c::build_fraction(
            vec_c.as_mut_ptr(),
            vec_c.len(),
            fraction.num,
            fraction.denom,
        )
    };
    assert_eq!(vec_rs, vec_c);

    assert_eq!(
        PodDeserializer::deserialize_from(&vec_rs),
        Ok((&[] as &[u8], fraction))
    );
}

#[test]
#[cfg_attr(miri, ignore)]
fn array_i32() {
    // 3 elements, so the resulting array has 4 bytes padding.
    let array: Vec<i32> = vec![10, 15, 19];

    let vec_rs: Vec<u8> = PodSerializer::serialize(Cursor::new(Vec::new()), array.as_slice())
        .unwrap()
        .0
        .into_inner();
    let mut vec_c: Vec<u8> = vec![0; 32];
    unsafe {
        c::build_array(
            vec_c.as_mut_ptr(),
            vec_c.len(),
            <i32 as CanonicalFixedSizedPod>::SIZE,
            spa_sys::SPA_TYPE_Int,
            array.len() as u32,
            array.as_ptr() as *const u8,
        )
    };
    assert_eq!(vec_rs, vec_c);

    assert_eq!(
        PodDeserializer::deserialize_from(&vec_rs),
        Ok((&[] as &[u8], array))
    )
}

#[test]
#[cfg_attr(miri, ignore)]
fn array_empty() {
    let array: Vec<()> = Vec::new();

    let vec_rs: Vec<u8> = PodSerializer::serialize(Cursor::new(Vec::new()), array.as_slice())
        .unwrap()
        .0
        .into_inner();
    let mut vec_c: Vec<u8> = vec![0; 16];
    unsafe {
        c::build_array(
            vec_c.as_mut_ptr(),
            vec_c.len(),
            0,
            spa_sys::SPA_TYPE_None,
            array.len() as u32,
            array.as_ptr() as *const u8,
        )
    };
    assert_eq!(vec_rs, vec_c);

    assert_eq!(
        PodDeserializer::deserialize_from(&vec_rs),
        Ok((&[] as &[u8], array))
    )
}

#[derive(PartialEq, Eq, Debug)]
struct TestStruct<'s> {
    // Fixed sized pod with padding.
    int: i32,
    // Dynamically sized pod.
    string: &'s str,
    // Another nested struct.
    nested: NestedStruct,
}

#[derive(PartialEq, Eq, Debug)]
struct NestedStruct {
    rect: Rectangle,
}

impl<'s> PodSerialize for TestStruct<'s> {
    fn serialize<O: std::io::Write + std::io::Seek>(
        &self,
        serializer: PodSerializer<O>,
    ) -> Result<SerializeSuccess<O>, cookie_factory::GenError> {
        let mut struct_serializer = serializer.serialize_struct()?;

        struct_serializer.serialize_field(&self.int)?;
        struct_serializer.serialize_field(self.string)?;
        struct_serializer.serialize_field(&self.nested)?;

        struct_serializer.end()
    }
}

impl PodSerialize for NestedStruct {
    fn serialize<O: std::io::Write + std::io::Seek>(
        &self,
        serializer: PodSerializer<O>,
    ) -> Result<SerializeSuccess<O>, cookie_factory::GenError> {
        let mut struct_serializer = serializer.serialize_struct()?;

        struct_serializer.serialize_field(&self.rect)?;

        struct_serializer.end()
    }
}

impl<'a, 'de> PodDeserialize<'de> for TestStruct<'de> {
    fn deserialize(
        deserializer: PodDeserializer<'de>,
    ) -> Result<(Self, DeserializeSuccess<'de>), DeserializeError<&'de [u8]>>
    where
        Self: Sized,
    {
        struct TestVisitor;

        impl<'de> Visitor<'de> for TestVisitor {
            type Value = TestStruct<'de>;
            type ArrayElem = std::convert::Infallible;

            fn visit_struct(
                &self,
                struct_deserializer: &mut StructPodDeserializer<'de>,
            ) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
                Ok(TestStruct {
                    int: struct_deserializer
                        .deserialize_field()?
                        .expect("Input has too few fields"),
                    string: struct_deserializer
                        .deserialize_field()?
                        .expect("Input has too few fields"),
                    nested: struct_deserializer
                        .deserialize_field()?
                        .expect("Input has too few fields"),
                })
            }
        }

        deserializer.deserialize_struct(TestVisitor)
    }
}

impl<'a, 'de> PodDeserialize<'de> for NestedStruct {
    fn deserialize(
        deserializer: PodDeserializer<'de>,
    ) -> Result<(Self, DeserializeSuccess<'de>), DeserializeError<&'de [u8]>>
    where
        Self: Sized,
    {
        struct NestedVisitor;

        impl<'de> Visitor<'de> for NestedVisitor {
            type Value = NestedStruct;
            type ArrayElem = std::convert::Infallible;

            fn visit_struct(
                &self,
                struct_deserializer: &mut StructPodDeserializer<'de>,
            ) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
                Ok(NestedStruct {
                    rect: struct_deserializer
                        .deserialize_field()?
                        .expect("Input has too few fields"),
                })
            }
        }

        deserializer.deserialize_struct(NestedVisitor)
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn struct_() {
    let struct_ = TestStruct {
        int: 313,
        string: "foo",
        nested: NestedStruct {
            rect: Rectangle {
                width: 31,
                height: 14,
            },
        },
    };

    let vec_rs: Vec<u8> = PodSerializer::serialize(Cursor::new(Vec::new()), &struct_)
        .unwrap()
        .0
        .into_inner();
    let mut vec_c: Vec<u8> = vec![0; 64];
    let c_string = CString::new(struct_.string).unwrap();
    unsafe {
        c::build_test_struct(
            vec_c.as_mut_ptr(),
            vec_c.len(),
            struct_.int,
            c_string.as_bytes_with_nul().as_ptr(),
            struct_.nested.rect.width,
            struct_.nested.rect.height,
        )
    };
    assert_eq!(vec_rs, vec_c);

    assert_eq!(
        PodDeserializer::deserialize_from(&vec_rs),
        Ok((&[] as &[u8], struct_))
    )
}

#[test]
#[cfg_attr(miri, ignore)]
fn id() {
    let id = Id(7);

    let vec_rs: Vec<u8> = PodSerializer::serialize(Cursor::new(Vec::new()), &id)
        .unwrap()
        .0
        .into_inner();
    let mut vec_c: Vec<u8> = vec![0; 16];
    assert_eq!(
        unsafe { c::build_id(vec_c.as_mut_ptr(), vec_c.len(), id.0) },
        0
    );

    assert_eq!(vec_rs, vec_c);

    assert_eq!(
        PodDeserializer::deserialize_from(&vec_rs),
        Ok((&[] as &[u8], id))
    );
}

#[test]
#[cfg_attr(miri, ignore)]
fn fd() {
    let fd = Fd(7);

    let vec_rs: Vec<u8> = PodSerializer::serialize(Cursor::new(Vec::new()), &fd)
        .unwrap()
        .0
        .into_inner();
    let mut vec_c: Vec<u8> = vec![0; 16];
    assert_eq!(
        unsafe { c::build_fd(vec_c.as_mut_ptr(), vec_c.len(), fd.0) },
        0
    );

    assert_eq!(vec_rs, vec_c);

    assert_eq!(
        PodDeserializer::deserialize_from(&vec_rs),
        Ok((&[] as &[u8], fd))
    );
}
