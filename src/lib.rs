//! Rustic bindings to [libnotify](https://developer.gnome.org/libnotify/)
//!
//! ```rust
//! extern crate libnotify;
//!
//! fn main() {
//!     // Create a libnotify context
//!     let notify = libnotify::Context::new("myapp").unwrap();
//!     // Create a new notification and show it
//!     let n = notify.new_notification("Summary",
//!                                     Some("Optional Body"),
//!                                     None).unwrap();
//!     n.show().unwrap();
//!     // You can also use the .show() convenience method on the context
//!     notify.show("I am another notification", None, None).unwrap();
//! }
//!
//! ```

#![warn(missing_docs)]

extern crate libnotify_sys as sys;
extern crate glib_sys;
extern crate gtypes;

use std::ffi::{self, CStr, CString};
use std::os::raw::c_int;
use std::marker::PhantomData;
use std::fmt;
use std::error::Error;

use gtypes::{TRUE, FALSE};

/// Error that can happen on context creation
#[derive(Debug)]
pub enum ContextCreationError {
    /// Context already exists.
    AlreadyExists,
    /// Failed to initialize libnotify.
    InitError,
    /// A nul byte was found in the provided string.
    NulError(ffi::NulError),
}

impl fmt::Display for ContextCreationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ContextCreationError::*;
        match *self {
            AlreadyExists => write!(f, "A Libnotify context already exists."),
            InitError => write!(f, "Failed to initialize libnotify."),
            NulError(ref e) => write!(f, "{}", e),
        }
    }
}

impl From<ffi::NulError> for ContextCreationError {
    fn from(src: ffi::NulError) -> Self {
        ContextCreationError::NulError(src)
    }
}

#[derive(Debug)]
/// An error that can happen when attempting to create a notification.
pub enum NotificationCreationError {
    /// A nul byte was found in the provided string.
    NulError(ffi::NulError),
    /// An unknown error happened.
    Unknown,
    /// Invalid parameter passed to a glib function
    InvalidParameter,
}

impl fmt::Display for NotificationCreationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use NotificationCreationError::*;
        match *self {
            NulError(ref e) => write!(f, "{}", e),
            Unknown => write!(f, "Unknown error"),
            InvalidParameter => write!(f, "An invalid parameter was passed"),
        }
    }
}

impl From<ffi::NulError> for NotificationCreationError {
    fn from(src: ffi::NulError) -> Self {
        NotificationCreationError::NulError(src)
    }
}

impl Error for NotificationCreationError {
    fn description(&self) -> &str {
        "notification creation error"
    }
}

/// The context which within libnotify operates.
///
/// Only one context can exist at a time.
pub struct Context;

impl Context {
    /// Create a new context
    ///
    /// Arguments:
    ///
    /// - app_name: The name of the application using the context
    pub fn new(app_name: &str) -> Result<Context, ContextCreationError> {
        unsafe {
            if sys::notify_is_initted() == TRUE {
                return Err(ContextCreationError::AlreadyExists);
            }
            let app_name = try!(CString::new(app_name));
            if sys::notify_init(app_name.as_ptr()) == FALSE {
                return Err(ContextCreationError::InitError);
            }
        }
        Ok(Context)
    }
    /// Creates a new Notification.
    ///
    /// Arguments:
    ///
    /// - summary: Required summary text
    /// - body: Optional body text
    /// - icon: Optional icon theme icon name or filename
    pub fn new_notification(&self,
                            summary: &str,
                            body: Option<&str>,
                            icon: Option<&str>)
                            -> Result<Notification, NotificationCreationError> {
        let summary = try!(CString::new(summary));
        let body = match body {
            Some(body) => Some(try!(CString::new(body))),
            None => None,
        };
        let body_ptr = match body {
            Some(ref body) => body.as_ptr(),
            None => std::ptr::null(),
        };
        let icon = match icon {
            Some(icon) => Some(try!(CString::new(icon))),
            None => None,
        };
        let icon_ptr = match icon {
            Some(ref icon) => icon.as_ptr(),
            None => std::ptr::null(),
        };

        unsafe {
            let n = sys::notify_notification_new(summary.as_ptr(), body_ptr, icon_ptr);
            if n.is_null() {
                return Err(NotificationCreationError::Unknown);
            }

            Ok(Notification {
                handle: n,
                _phantom: PhantomData,
            })
        }
    }
    /// Show a notification.
    ///
    /// This is a convenience method that creates a new notification,
    /// and shows it in one step.
    pub fn show(&self,
                summary: &str,
                body: Option<&str>,
                icon: Option<&str>)
                -> Result<(), Box<Error>> {
        let notif = try!(self.new_notification(summary, body, icon));
        try!(notif.show());
        Ok(())
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            sys::notify_uninit();
        }
    }
}

/// A passive pop-up notification
pub struct Notification<'a> {
    handle: *mut sys::NotifyNotification,
    _phantom: PhantomData<&'a Context>,
}

impl<'a> Notification<'a> {
    /// Tells the notification server to display the notification
    /// on the screen.
    pub fn show(&'a self) -> Result<(), NotificationShowError> {
        unsafe {
            let mut err: *mut glib_sys::GError = std::ptr::null_mut();
            sys::notify_notification_show(self.handle, &mut err);
            if !err.is_null() {
                let result = Err(NotificationShowError {
                    message: CStr::from_ptr((*err).message).to_string_lossy().into_owned(),
                });
                glib_sys::g_error_free(err);
                return result;
            }
            Ok(())
        }
    }

    /// Set the notification timeout. Note that the server might ignore
    /// the timeout.
    pub fn set_notification_timeout(&self, timeout: i32) {
        let _timeout: c_int = From::from(timeout);

        unsafe {
            sys::notify_notification_set_timeout(self.handle,
                                                 _timeout)

        }
    }

    /// Updates the notification text and icon. This won't send the update
    /// out and display it on the screen. For that, you will need to
    /// call `.show()`.
    pub fn update(&self,
                  summary: &str,
                  body: Option<&str>,
                  icon: Option<&str>) -> Result<(), NotificationCreationError> {
        let summary = try!(CString::new(summary));
        let body = match body {
            Some(body) => Some(try!(CString::new(body))),
            None => None,
        };
        let body_ptr = match body {
            Some(ref body) => body.as_ptr(),
            None => std::ptr::null(),
        };
        let icon = match icon {
            Some(icon) => Some(try!(CString::new(icon))),
            None => None,
        };
        let icon_ptr = match icon {
            Some(ref icon) => icon.as_ptr(),
            None => std::ptr::null(),
        };

        unsafe {
            let b = sys::notify_notification_update(self.handle,
                                       summary.as_ptr(),
                                       body_ptr,
                                       icon_ptr);
            if b == FALSE {
                return Err(NotificationCreationError::InvalidParameter);
            }
        }

        return Ok(());
    }
}

/// An error that can happen when attempting to show a notification.
#[derive(Debug)]
pub struct NotificationShowError {
    message: String,
}

impl fmt::Display for NotificationShowError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error showing notification: {}", self.message)
    }
}

impl Error for NotificationShowError {
    fn description(&self) -> &str {
        "notification show error"
    }
}
