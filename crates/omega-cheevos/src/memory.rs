use std::cell::RefCell;
use std::ffi::{c_uint, c_void};

use omega_retro::{CoreMemory, MemoryDescriptor};

use crate::ffi;

/// Where rcheevos reads the game's memory from: the map the core declared, or, for a core that
/// declares none (Snes9x), the system and save RAM `retro_get_memory_data` hands out.
struct Regions {
    map: Vec<MemoryDescriptor>,
    ram: CoreMemory,
    console: u32,
    handle: *mut c_void,
}

impl Drop for Regions {
    fn drop(&mut self) {
        unsafe { ffi::omega_regions_free(self.handle) };
    }
}

thread_local! {
    static REGIONS: RefCell<Option<Regions>> = const { RefCell::new(None) };
}

/// Builds the regions again when the core's map, its RAM or the console changed, and says
/// whether there are any to read. A core that has declared nothing yet (mGBA declares its map
/// on its first frame) has none.
pub fn refresh(map: &[MemoryDescriptor], ram: CoreMemory, console: u32) -> bool {
    REGIONS.with_borrow_mut(|current| {
        let same = current
            .as_ref()
            .is_some_and(|r| r.console == console && r.map == map && r.ram == ram);
        if !same && (!map.is_empty() || !ram.is_empty()) {
            *current = Some(Regions {
                map: map.to_vec(),
                ram,
                console,
                handle: build(map, ram, console),
            });
        }
        current.as_ref().is_some_and(|r| !r.handle.is_null())
    })
}

pub fn ready() -> bool {
    REGIONS.with_borrow(|r| r.as_ref().is_some_and(|r| !r.handle.is_null()))
}

pub fn clear() {
    REGIONS.with_borrow_mut(|r| *r = None);
}

pub fn read(address: u32, buffer: &mut [u8]) -> u32 {
    REGIONS.with_borrow(|r| match r {
        Some(r) if !r.handle.is_null() => unsafe {
            ffi::omega_regions_read(r.handle, address, buffer.as_mut_ptr(), buffer.len() as u32)
        },
        _ => 0,
    })
}

fn build(map: &[MemoryDescriptor], ram: CoreMemory, console: u32) -> *mut c_void {
    let raw: Vec<ffi::RetroMemoryDescriptor> = map
        .iter()
        .map(|d| ffi::RetroMemoryDescriptor {
            flags: d.flags,
            ptr: d.ptr as *mut c_void,
            offset: d.offset,
            start: d.start,
            select: d.select,
            disconnect: d.disconnect,
            len: d.len,
            addrspace: std::ptr::null(),
        })
        .collect();
    let core = ffi::OmegaCoreMemory {
        system_ram: ram.system_ram.0 as *mut u8,
        system_ram_size: ram.system_ram.1,
        save_ram: ram.save_ram.0 as *mut u8,
        save_ram_size: ram.save_ram.1,
    };
    unsafe { ffi::omega_regions_new(raw.as_ptr(), raw.len() as c_uint, console, &core) }
}

pub(crate) unsafe extern "C" fn read_memory(
    address: u32,
    buffer: *mut u8,
    n: u32,
    _client: *mut ffi::rc_client_t,
) -> u32 {
    if buffer.is_null() {
        return 0;
    }
    read(address, std::slice::from_raw_parts_mut(buffer, n as usize))
}
