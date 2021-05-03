use std::{
    collections::VecDeque,
    ffi::c_void,
    os::unix::prelude::*,
    sync::{Arc, Mutex},
};

use crate::{IoSource, Loop};
use spa::flags::IoFlags;

/// A receiver that has not been attached to a loop.
///
/// Use its [`attach`](`Self::attach`) function to receive messages by attaching it to a loop.
pub struct Receiver<T: 'static> {
    channel: Arc<Mutex<Channel<T>>>,
}

impl<T: 'static> Receiver<T> {
    /// Attach the receiver to a loop with a callback.
    ///
    /// This will make the loop call the callback with any messages that get sent to the receiver.
    #[must_use]
    pub fn attach<F, L>(self, loop_: &L, callback: F) -> AttachedReceiver<T, L>
    where
        F: Fn(T) + 'static,
        L: Loop,
    {
        let channel = self.channel.clone();
        let eventfd = channel.lock().expect("Channel mutex lock poisoned").eventfd;

        // Attach the eventfd as an IO source to the loop.
        // Whenever the eventfd is signaled, call the users callback with each message in the queue.
        let iosource = loop_.add_io(eventfd, IoFlags::IN, move |_| {
            let mut channel = channel.lock().expect("Channel mutex lock poisoned");

            // Read from the eventfd to make it block until written to again.
            unsafe {
                let mut _eventnum: u64 = 0;
                libc::read(
                    channel.eventfd,
                    &mut _eventnum as *mut u64 as *mut c_void,
                    std::mem::size_of::<u64>(),
                );
            }

            channel.queue.drain(..).for_each(&callback);
        });

        AttachedReceiver {
            _source: iosource,
            receiver: self,
        }
    }
}

/// A [`Receiver`] that has been attached to a loop.
///
/// Dropping this will cause it to be deattached from the loop, so no more messages will be received.
pub struct AttachedReceiver<'l, T, L>
where
    T: 'static,
    L: Loop,
{
    _source: IoSource<'l, RawFd, L>,
    receiver: Receiver<T>,
}

impl<'l, T, L> AttachedReceiver<'l, T, L>
where
    T: 'static,
    L: Loop,
{
    /// Deattach the receiver from the loop.
    ///
    /// No more messages will be received until you attach it to a loop again.
    #[must_use]
    pub fn deattach(self) -> Receiver<T> {
        self.receiver
    }
}

#[derive(Clone)]
/// A `Sender` can be used to send messages to its associated [`Receiver`].
///
/// It can be freely cloned, so you can send messages from multiple  places.
pub struct Sender<T> {
    channel: Arc<Mutex<Channel<T>>>,
}

impl<T> Sender<T> {
    /// Send a message to the associated receiver.
    ///
    /// On any errors, this returns the message back to the caller.
    pub fn send(&self, t: T) -> Result<(), T> {
        // Lock the channel.
        let mut channel = match self.channel.lock() {
            Ok(chan) => chan,
            Err(_) => return Err(t),
        };

        // If no messages are waiting already, signal the receiver to read some.
        // Because the channel mutex is locked, it is alright to do this before pushing the message.
        if channel.queue.is_empty() {
            let res = unsafe {
                libc::write(
                    channel.eventfd,
                    &1u64 as *const u64 as *const c_void,
                    std::mem::size_of::<u64>(),
                )
            };
            if res == -1 {
                // Eventfd write failed.
                return Err(t);
            }
        }

        // Push the new message into the queue.
        channel.queue.push_back(t);

        Ok(())
    }
}

/// Shared state between the [`Sender`]s and the [`Receiver`].
struct Channel<T> {
    /// A raw eventfd used to signal the loop the receiver is attached to that messages are waiting.
    eventfd: RawFd,
    /// Queue of any messages waiting to be received.
    queue: VecDeque<T>,
}

impl<T> Drop for Channel<T> {
    fn drop(&mut self) {
        unsafe {
            // We do not error check here, because the eventfd does not contain any data that might be lost,
            // and because there is no way to handle an error in a `Drop` implementation anyways.
            libc::close(self.eventfd);
        }
    }
}

/// Create a Sender-Receiver pair, where the sender can be used to send messages to the receiver.
///
/// This functions similar to [`std::sync::mpsc`], but with a receiver that can be attached to any
/// [`Loop`](`crate::Loop`) to have the loop invoke a callback with any new messages.
///
/// This can be used for inter-thread communication without shared state and where [`std::sync::mpsc`] can not be used
/// because the receiving thread is running the pipewire loop.
pub fn channel<T>() -> (Sender<T>, Receiver<T>)
where
    T: 'static,
{
    // Manually open an eventfd that we can use to signal the loop thread to check for messages
    // via an IoSource.
    let eventfd = unsafe { libc::eventfd(0, libc::EFD_CLOEXEC) };
    if eventfd == -1 {
        panic!("Failed to create eventfd: {}", errno::errno())
    }

    let channel: Arc<Mutex<Channel<T>>> = Arc::new(Mutex::new(Channel {
        eventfd,
        queue: VecDeque::new(),
    }));

    (
        Sender {
            channel: channel.clone(),
        },
        Receiver { channel },
    )
}
