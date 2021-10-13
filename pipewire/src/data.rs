use std::convert::TryFrom;

#[repr(transparent)]
pub struct Data(spa_sys::spa_data);
#[repr(transparent)]
pub struct Chunk(spa_sys::spa_chunk);

impl Data {
    pub fn get_mut(&mut self) -> &mut [u8] {
        unsafe {
            std::slice::from_raw_parts_mut(
                self.0.data as *mut u8,
                usize::try_from(self.0.maxsize).unwrap(),
            )
        }
    }

    pub fn chunk(&mut self) -> &mut Chunk {
        assert_ne!(self.0.chunk, std::ptr::null_mut());
        unsafe {
            let chunk: *mut spa_sys::spa_chunk = self.0.chunk;
            &mut *(chunk as *mut Chunk)
        }
    }
}

impl Chunk {
    pub fn size_mut(&mut self) -> &mut u32 {
        &mut self.0.size
    }
    pub fn offset_mut(&mut self) -> &mut u32 {
        &mut self.0.offset
    }
    pub fn stride_mut(&mut self) -> &mut i32 {
        &mut self.0.stride
    }
}
