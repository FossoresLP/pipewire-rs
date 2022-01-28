// Copyright The pipewire-rs Contributors.
// SPDX-License-Identifier: MIT

use bitflags::bitflags;
use libc::{c_char, c_void};
use std::{
    ffi::{CStr, CString},
    rc::Rc,
};
use std::{fmt, mem, ptr};
use std::{ops::Deref, pin::Pin};

use crate::{
    proxy::{Proxy, ProxyT},
    registry::Registry,
    Error,
};
use spa::{dict::ForeignDict, result::SpaResult, spa_interface_call_method, AsyncSeq};

pub const PW_ID_CORE: u32 = pw_sys::PW_ID_CORE;
#[derive(Debug, Clone)]
pub struct Core {
    inner: Rc<CoreInner>,
}

impl Core {
    pub fn from_ptr(ptr: ptr::NonNull<pw_sys::pw_core>) -> Self {
        let inner = CoreInner::from_ptr(ptr);
        Self {
            inner: Rc::new(inner),
        }
    }
}

impl Deref for Core {
    type Target = CoreInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug)]
pub struct CoreInner {
    ptr: ptr::NonNull<pw_sys::pw_core>,
}

impl CoreInner {
    fn from_ptr(ptr: ptr::NonNull<pw_sys::pw_core>) -> Self {
        Self { ptr }
    }

    /// Get the underlying pointer for this `Core`.
    ///
    /// ## Safety
    ///
    /// The lifetime of the pointer should not exceed the lifetime of the `Core`
    /// object itself.
    pub(crate) fn as_ptr(&self) -> *mut pw_sys::pw_core {
        self.ptr.as_ptr()
    }

    // TODO: add non-local version when we'll bind pw_thread_loop_start()
    #[must_use]
    pub fn add_listener_local(&self) -> ListenerLocalBuilder {
        ListenerLocalBuilder {
            core: self,
            cbs: ListenerLocalCallbacks::default(),
        }
    }

    pub fn get_registry(&self) -> Result<Registry, Error> {
        let registry = unsafe {
            spa_interface_call_method!(
                self.as_ptr(),
                pw_sys::pw_core_methods,
                get_registry,
                pw_sys::PW_VERSION_REGISTRY,
                0
            )
        };
        let registry = ptr::NonNull::new(registry).ok_or(Error::CreationFailed)?;

        Ok(Registry::new(registry))
    }

    pub fn sync(&self, seq: i32) -> Result<AsyncSeq, Error> {
        let res = unsafe {
            spa_interface_call_method!(
                self.as_ptr(),
                pw_sys::pw_core_methods,
                sync,
                PW_ID_CORE,
                seq
            )
        };

        let res = SpaResult::from_c(res).into_async_result()?;
        Ok(res)
    }

    /// Create a new object on the PipeWire server from a factory.
    ///
    /// You will need specify what type you are expecting to be constructed by either using type inference or the
    /// turbofish syntax.
    ///
    /// # Parameters
    /// - `factory_name` the name of the factory to use
    /// - `properties` extra properties that the new object will have
    ///
    /// # Panics
    /// If `factory_name` contains a null byte.
    ///
    /// # Returns
    /// One of:
    /// - `Ok(P)` on success, where `P` is the newly created object
    /// - `Err(Error::CreationFailed)` if the object could not be created
    /// - `Err(Error::WrongProxyType)` if the created type does not match the type `P` that the user is trying to create
    ///
    /// # Examples
    /// Creating a new link:
    // Doctest ignored, as the factory name is hardcoded, but may be different on different systems.
    /// ```ignore
    /// use pipewire as pw;
    ///
    /// pw::init();
    ///
    /// let mainloop = pw::MainLoop::new().expect("Failed to create Pipewire Mainloop");
    /// let context = pw::Context::new(&mainloop).expect("Failed to create Pipewire Context");
    /// let core = context
    ///     .connect(None)
    ///     .expect("Failed to connect to Pipewire Core");
    ///
    /// // This call uses turbofish syntax to specify that we want a link.
    /// let link = core.create_object::<pw::link::Link, _>(
    ///     // The actual name for a link factory might be different for your system,
    ///     // you should probably obtain a factory from the registry.
    ///     "link-factory",
    ///     &pw::properties! {
    ///         "link.output.port" => "1",
    ///         "link.input.port" => "2",
    ///         "link.output.node" => "3",
    ///         "link.input.node" => "4"
    ///     },
    /// )
    /// .expect("Failed to create object");
    /// ```
    ///
    /// See `pipewire/examples/create-delete-remote-objects.rs` in the crates repository for a more detailed example.
    pub fn create_object<P: ProxyT, D: crate::spa::dict::ReadableDict>(
        &self,
        factory_name: &str,
        properties: &D,
    ) -> Result<P, Error> {
        let type_ = P::type_();
        let factory_name = CString::new(factory_name).expect("Null byte in factory_name parameter");
        let type_str = CString::new(type_.to_string())
            .expect("Null byte in string representation of type_ parameter");

        let res = unsafe {
            spa_interface_call_method!(
                self.as_ptr(),
                pw_sys::pw_core_methods,
                create_object,
                factory_name.as_ptr(),
                type_str.as_ptr(),
                type_.client_version(),
                properties.get_dict_ptr(),
                0
            )
        };

        let ptr = ptr::NonNull::new(res.cast()).ok_or(Error::CreationFailed)?;

        Proxy::new(ptr).downcast().map_err(|(_, e)| e)
    }

    /// Destroy the object on the remote server represented by the provided proxy.
    ///
    /// The proxy will be destroyed alongside the server side ressource, as it is no longer needed.
    pub fn destroy_object<P: ProxyT>(&self, proxy: P) -> Result<AsyncSeq, Error> {
        let res = unsafe {
            spa_interface_call_method!(
                self.as_ptr(),
                pw_sys::pw_core_methods,
                destroy,
                proxy.upcast_ref().as_ptr() as *mut c_void
            )
        };

        let res = SpaResult::from_c(res).into_async_result()?;
        Ok(res)
    }
}

#[derive(Default)]
struct ListenerLocalCallbacks {
    info: Option<Box<dyn Fn(&Info)>>,
    done: Option<Box<dyn Fn(u32, AsyncSeq)>>,
    #[allow(clippy::type_complexity)]
    error: Option<Box<dyn Fn(u32, i32, i32, &str)>>, // TODO: return a proper Error enum?
                                                     // TODO: ping, remove_id, bound_id, add_mem, remove_mem
}

pub struct ListenerLocalBuilder<'a> {
    core: &'a CoreInner,
    cbs: ListenerLocalCallbacks,
}

pub struct Listener {
    // Need to stay allocated while the listener is registered
    #[allow(dead_code)]
    events: Pin<Box<pw_sys::pw_core_events>>,
    listener: Pin<Box<spa_sys::spa_hook>>,
    #[allow(dead_code)]
    data: Box<ListenerLocalCallbacks>,
}

impl Listener {
    pub fn unregister(self) {
        // Consuming the listener will call drop()
    }
}

impl<'a> Drop for Listener {
    fn drop(&mut self) {
        spa::hook::remove(*self.listener);
    }
}

impl<'a> ListenerLocalBuilder<'a> {
    #[must_use]
    pub fn info<F>(mut self, info: F) -> Self
    where
        F: Fn(&Info) + 'static,
    {
        self.cbs.info = Some(Box::new(info));
        self
    }

    #[must_use]
    pub fn done<F>(mut self, done: F) -> Self
    where
        F: Fn(u32, AsyncSeq) + 'static,
    {
        self.cbs.done = Some(Box::new(done));
        self
    }

    #[must_use]
    pub fn error<F>(mut self, error: F) -> Self
    where
        F: Fn(u32, i32, i32, &str) + 'static,
    {
        self.cbs.error = Some(Box::new(error));
        self
    }

    #[must_use]
    pub fn register(self) -> Listener {
        unsafe extern "C" fn core_events_info(
            data: *mut c_void,
            info: *const pw_sys::pw_core_info,
        ) {
            let callbacks = (data as *mut ListenerLocalCallbacks).as_ref().unwrap();
            let info = Info::new(ptr::NonNull::new(info as *mut _).expect("info is NULL"));
            callbacks.info.as_ref().unwrap()(&info);
        }

        unsafe extern "C" fn core_events_done(data: *mut c_void, id: u32, seq: i32) {
            let callbacks = (data as *mut ListenerLocalCallbacks).as_ref().unwrap();
            callbacks.done.as_ref().unwrap()(id, AsyncSeq::from_raw(seq));
        }

        unsafe extern "C" fn core_events_error(
            data: *mut c_void,
            id: u32,
            seq: i32,
            res: i32,
            message: *const c_char,
        ) {
            let callbacks = (data as *mut ListenerLocalCallbacks).as_ref().unwrap();
            let message = CStr::from_ptr(message).to_str().unwrap();
            callbacks.error.as_ref().unwrap()(id, seq, res, message);
        }

        let e = unsafe {
            let mut e: Pin<Box<pw_sys::pw_core_events>> = Box::pin(mem::zeroed());
            e.version = pw_sys::PW_VERSION_CORE_EVENTS;

            if self.cbs.info.is_some() {
                e.info = Some(core_events_info);
            }
            if self.cbs.done.is_some() {
                e.done = Some(core_events_done);
            }
            if self.cbs.error.is_some() {
                e.error = Some(core_events_error);
            }

            e
        };

        let (listener, data) = unsafe {
            let ptr = self.core.as_ptr();
            let data = Box::into_raw(Box::new(self.cbs));
            let mut listener: Pin<Box<spa_sys::spa_hook>> = Box::pin(mem::zeroed());
            // Have to cast from pw-sys namespaced type to the equivalent spa-sys type
            // as bindgen does not allow us to generate bindings dependings of another
            // sys crate, see https://github.com/rust-lang/rust-bindgen/issues/1929
            let listener_ptr: *mut spa_sys::spa_hook = listener.as_mut().get_unchecked_mut();

            spa_interface_call_method!(
                ptr,
                pw_sys::pw_core_methods,
                add_listener,
                listener_ptr.cast(),
                e.as_ref().get_ref(),
                data as *mut _
            );

            (listener, Box::from_raw(data))
        };

        Listener {
            events: e,
            listener,
            data,
        }
    }
}

pub struct Info {
    ptr: ptr::NonNull<pw_sys::pw_core_info>,
    /// Can contain a Dict wrapping the raw spa_dict at (*ptr).props.
    ///
    /// Since it is our responsibility that it does not stay alive longer than the raw dict,
    /// we store it here and only hand out borrows to it.
    props: Option<ForeignDict>,
}

impl Info {
    fn new(info: ptr::NonNull<pw_sys::pw_core_info>) -> Self {
        let props_ptr = unsafe { info.as_ref().props };
        let props = ptr::NonNull::new(props_ptr).map(|ptr| unsafe { ForeignDict::from_ptr(ptr) });

        Self { ptr: info, props }
    }

    pub fn id(&self) -> u32 {
        unsafe { self.ptr.as_ref().id }
    }

    pub fn cookie(&self) -> u32 {
        unsafe { self.ptr.as_ref().cookie }
    }

    pub fn user_name(&self) -> &str {
        unsafe {
            CStr::from_ptr(self.ptr.as_ref().user_name)
                .to_str()
                .unwrap()
        }
    }

    pub fn host_name(&self) -> &str {
        unsafe {
            CStr::from_ptr(self.ptr.as_ref().host_name)
                .to_str()
                .unwrap()
        }
    }

    pub fn version(&self) -> &str {
        unsafe { CStr::from_ptr(self.ptr.as_ref().version).to_str().unwrap() }
    }

    pub fn name(&self) -> &str {
        unsafe { CStr::from_ptr(self.ptr.as_ref().name).to_str().unwrap() }
    }

    pub fn change_mask(&self) -> ChangeMask {
        let mask = unsafe { self.ptr.as_ref().change_mask };
        ChangeMask::from_bits(mask).expect("invalid change_mask")
    }

    pub fn props(&self) -> Option<&ForeignDict> {
        self.props.as_ref()
    }
}

impl fmt::Debug for Info {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CoreInfo")
            .field("id", &self.id())
            .field("cookie", &self.cookie())
            .field("user-name", &self.user_name())
            .field("host-name", &self.host_name())
            .field("version", &self.version())
            .field("name", &self.name())
            .field("change-mask", &self.change_mask())
            .field("props", &self.props())
            .finish()
    }
}

bitflags! {
    pub struct ChangeMask: u64 {
        const PROPS = pw_sys::PW_CORE_CHANGE_MASK_PROPS as u64;
    }
}
