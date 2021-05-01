use super::stream::Stream;

use std::ptr::NonNull;

pub struct Buffer<'s> {
    buf: NonNull<pw_sys::pw_buffer>,

    /// In Pipewire, buffers are owned by the stream that generated them.
    /// This reference ensures that this rule is respected.
    stream: &'s Stream,
}

impl Buffer<'_> {
    pub(crate) unsafe fn from_raw(
        buf: *mut pw_sys::pw_buffer,
        stream: &Stream,
    ) -> Option<Buffer<'_>> {
        NonNull::new(buf).map(|buf| Buffer { buf, stream })
    }
}

impl Drop for Buffer<'_> {
    fn drop(&mut self) {
        unsafe {
            self.stream.queue_raw_buffer(self.buf.as_ptr());
        }
    }
}
