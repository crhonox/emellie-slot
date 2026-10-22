use std::ffi::{c_char, c_void, CString};
use std::mem::ManuallyDrop;

use libloading::Library;

use crate::canvas::{out_h, out_w};
use crate::fbdev::egl_error;
use crate::surface::{GfxError, Surface};

const EGL_NONE: i32 = 0x3038;
const EGL_ALPHA_SIZE: i32 = 0x3021;
const EGL_BLUE_SIZE: i32 = 0x3022;
const EGL_GREEN_SIZE: i32 = 0x3023;
const EGL_RED_SIZE: i32 = 0x3024;
const EGL_SURFACE_TYPE: i32 = 0x3033;
const EGL_HEIGHT: i32 = 0x3056;
const EGL_WIDTH: i32 = 0x3057;
const EGL_RENDERABLE_TYPE: i32 = 0x3040;
const EGL_CONTEXT_CLIENT_VERSION: i32 = 0x3098;
const EGL_OPENGL_ES_API: u32 = 0x30A0;
const EGL_PBUFFER_BIT: i32 = 0x0001;
const EGL_OPENGL_ES2_BIT: i32 = 0x0004;
const EGL_PLATFORM_SURFACELESS_MESA: u32 = 0x31DD;

type Ptr = *mut c_void;
type GetDisplay = unsafe extern "C" fn(Ptr) -> Ptr;
type GetPlatformDisplay = unsafe extern "C" fn(u32, Ptr, *const i32) -> Ptr;
type Initialize = unsafe extern "C" fn(Ptr, *mut i32, *mut i32) -> u32;
type BindApi = unsafe extern "C" fn(u32) -> u32;
type ChooseConfig = unsafe extern "C" fn(Ptr, *const i32, *mut Ptr, i32, *mut i32) -> u32;
type CreatePbufferSurface = unsafe extern "C" fn(Ptr, Ptr, *const i32) -> Ptr;
type CreateContext = unsafe extern "C" fn(Ptr, Ptr, Ptr, *const i32) -> Ptr;
type MakeCurrent = unsafe extern "C" fn(Ptr, Ptr, Ptr, Ptr) -> u32;
type GetProcAddress = unsafe extern "C" fn(*const c_char) -> *const c_void;
type GetError = unsafe extern "C" fn() -> i32;
type DestroySurface = unsafe extern "C" fn(Ptr, Ptr) -> u32;
type DestroyContext = unsafe extern "C" fn(Ptr, Ptr) -> u32;

/// An offscreen GLES2 context through EGL, for the examples and GL tests away from macOS.
/// It asks for Mesa's surfaceless platform first, so no X or Wayland session is needed. The
/// display is shared by every surface in the process, so dropping one never terminates it, and
/// the libraries stay loaded: Mesa leaves per-thread destructors behind that run at thread exit.
pub struct HeadlessSurface {
    make_current: MakeCurrent,
    get_proc_address: GetProcAddress,
    get_error: GetError,
    destroy_surface: DestroySurface,
    destroy_context: DestroyContext,
    display: Ptr,
    surface: Ptr,
    context: Ptr,
    gles: Option<ManuallyDrop<Library>>,
    _egl: ManuallyDrop<Library>,
}

fn open(name: &str) -> Result<Library, GfxError> {
    unsafe { Library::new(name) }.map_err(|e| GfxError::Context(format!("{name}: {e}")))
}

unsafe fn sym<T: Copy>(lib: &Library, name: &str) -> Result<T, GfxError> {
    lib.get::<T>(name.as_bytes())
        .map(|s| *s)
        .map_err(|e| GfxError::Context(format!("{name}: {e}")))
}

impl HeadlessSurface {
    pub fn new() -> Result<Self, GfxError> {
        let egl = open("libEGL.so.1").or_else(|_| open("libEGL.so"))?;
        unsafe {
            let get_display: GetDisplay = sym(&egl, "eglGetDisplay")?;
            let initialize: Initialize = sym(&egl, "eglInitialize")?;
            let bind_api: BindApi = sym(&egl, "eglBindAPI")?;
            let choose_config: ChooseConfig = sym(&egl, "eglChooseConfig")?;
            let create_pbuffer: CreatePbufferSurface = sym(&egl, "eglCreatePbufferSurface")?;
            let create_context: CreateContext = sym(&egl, "eglCreateContext")?;
            let make_current: MakeCurrent = sym(&egl, "eglMakeCurrent")?;
            let get_proc_address: GetProcAddress = sym(&egl, "eglGetProcAddress")?;
            let get_error: GetError = sym(&egl, "eglGetError")?;
            let destroy_surface: DestroySurface = sym(&egl, "eglDestroySurface")?;
            let destroy_context: DestroyContext = sym(&egl, "eglDestroyContext")?;
            let fail = |what: &str| egl_error(what, get_error());

            let display = surfaceless_display(get_proc_address, initialize)
                .or_else(|| {
                    let display = get_display(std::ptr::null_mut());
                    let ok = !display.is_null()
                        && initialize(display, std::ptr::null_mut(), std::ptr::null_mut()) != 0;
                    ok.then_some(display)
                })
                .ok_or_else(|| fail("eglInitialize"))?;
            if bind_api(EGL_OPENGL_ES_API) == 0 {
                return Err(fail("eglBindAPI"));
            }
            let attrs = [
                EGL_SURFACE_TYPE,
                EGL_PBUFFER_BIT,
                EGL_RED_SIZE,
                8,
                EGL_GREEN_SIZE,
                8,
                EGL_BLUE_SIZE,
                8,
                EGL_ALPHA_SIZE,
                8,
                EGL_RENDERABLE_TYPE,
                EGL_OPENGL_ES2_BIT,
                EGL_NONE,
            ];
            let mut config: Ptr = std::ptr::null_mut();
            let mut found = 0;
            if choose_config(display, attrs.as_ptr(), &mut config, 1, &mut found) == 0 {
                return Err(fail("eglChooseConfig"));
            }
            if found == 0 {
                return Err(GfxError::Context(
                    "eglChooseConfig: no es2 pbuffer config with 8/8/8/8 colour".into(),
                ));
            }
            let size = [
                EGL_WIDTH,
                out_w() as i32,
                EGL_HEIGHT,
                out_h() as i32,
                EGL_NONE,
            ];
            let surface = create_pbuffer(display, config, size.as_ptr());
            if surface.is_null() {
                return Err(fail("eglCreatePbufferSurface"));
            }
            let context_attrs = [EGL_CONTEXT_CLIENT_VERSION, 2, EGL_NONE];
            let context = create_context(
                display,
                config,
                std::ptr::null_mut(),
                context_attrs.as_ptr(),
            );
            if context.is_null() {
                destroy_surface(display, surface);
                return Err(fail("eglCreateContext"));
            }
            if make_current(display, surface, surface, context) == 0 {
                destroy_context(display, context);
                destroy_surface(display, surface);
                return Err(fail("eglMakeCurrent"));
            }
            let gles = open("libGLESv2.so.2")
                .or_else(|_| open("libGLESv2.so"))
                .ok()
                .map(ManuallyDrop::new);
            Ok(HeadlessSurface {
                make_current,
                get_proc_address,
                get_error,
                destroy_surface,
                destroy_context,
                display,
                surface,
                context,
                gles,
                _egl: ManuallyDrop::new(egl),
            })
        }
    }
}

unsafe fn surfaceless_display(
    get_proc_address: GetProcAddress,
    initialize: Initialize,
) -> Option<Ptr> {
    let name = c"eglGetPlatformDisplayEXT";
    let f = get_proc_address(name.as_ptr());
    if f.is_null() {
        return None;
    }
    let get_platform_display: GetPlatformDisplay = std::mem::transmute(f);
    let display = get_platform_display(
        EGL_PLATFORM_SURFACELESS_MESA,
        std::ptr::null_mut(),
        std::ptr::null(),
    );
    let ok =
        !display.is_null() && initialize(display, std::ptr::null_mut(), std::ptr::null_mut()) != 0;
    ok.then_some(display)
}

impl Surface for HeadlessSurface {
    fn make_current(&mut self) -> Result<(), GfxError> {
        let ok =
            unsafe { (self.make_current)(self.display, self.surface, self.surface, self.context) };
        match ok {
            0 => Err(egl_error("eglMakeCurrent", unsafe { (self.get_error)() })),
            _ => Ok(()),
        }
    }

    fn window_size(&self) -> (u32, u32) {
        (out_w(), out_h())
    }

    fn swap(&mut self) -> Result<(), GfxError> {
        unsafe { gl::Flush() };
        Ok(())
    }

    fn proc_address(&self, name: &str) -> *const c_void {
        let Ok(c) = CString::new(name) else {
            return std::ptr::null();
        };
        let exported = self
            .gles
            .as_ref()
            .and_then(|lib| unsafe { lib.get::<unsafe extern "C" fn()>(name.as_bytes()) }.ok());
        match exported {
            Some(f) => *f as *const c_void,
            None => unsafe { (self.get_proc_address)(c.as_ptr()) },
        }
    }
}

impl Drop for HeadlessSurface {
    fn drop(&mut self) {
        unsafe {
            (self.make_current)(
                self.display,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            (self.destroy_context)(self.display, self.context);
            (self.destroy_surface)(self.display, self.surface);
        }
    }
}
