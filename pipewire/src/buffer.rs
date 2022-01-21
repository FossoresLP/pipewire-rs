use super::stream::Stream;

use crate::data::Data;
use std::convert::TryFrom;
use std::ptr::NonNull;

pub struct Buffer<'s, D> {
    buf: NonNull<pw_sys::pw_buffer>,

    /// In Pipewire, buffers are owned by the stream that generated them.
    /// This reference ensures that this rule is respected.
    stream: &'s Stream<D>,

    /// An empty array of `Data`, that can be used to return an empty slice
    /// when a buffer has no data.
    empty_data: [Data; 0],
}

impl<D> Buffer<'_, D> {
    pub(crate) unsafe fn from_raw(
        buf: *mut pw_sys::pw_buffer,
        stream: &Stream<D>,
    ) -> Option<Buffer<'_, D>> {
        NonNull::new(buf).map(|buf| Buffer {
            buf,
            stream,
            empty_data: [],
        })
    }

    pub fn datas_mut(&mut self) -> &mut [Data] {
        let buffer: *mut spa_sys::spa_buffer = unsafe { self.buf.as_ref().buffer };

        let slice_of_data = if !buffer.is_null()
            && unsafe { (*buffer).n_datas > 0 && !(*buffer).datas.is_null() }
        {
            unsafe {
                let datas = (*buffer).datas as *mut Data;
                std::slice::from_raw_parts_mut(datas, usize::try_from((*buffer).n_datas).unwrap())
            }
        } else {
            &mut self.empty_data
        };

        slice_of_data
    }
}

impl<D> Drop for Buffer<'_, D> {
    fn drop(&mut self) {
        unsafe {
            self.stream.queue_raw_buffer(self.buf.as_ptr());
        }
    }
}
