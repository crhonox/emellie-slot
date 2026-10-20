use omega_cheevos::{game, memory};
use omega_retro::{CoreMemory, MemoryDescriptor};

fn gba(iwram: &mut [u8], ewram: &mut [u8]) -> Vec<MemoryDescriptor> {
    vec![
        MemoryDescriptor {
            ptr: iwram.as_mut_ptr() as usize,
            start: 0x3000000,
            len: iwram.len(),
            ..Default::default()
        },
        MemoryDescriptor {
            ptr: ewram.as_mut_ptr() as usize,
            start: 0x2000000,
            len: ewram.len(),
            ..Default::default()
        },
    ]
}

#[test]
fn reads_gba_ram_through_the_retroachievements_address_map() {
    let mut iwram = vec![0u8; 0x8000];
    let mut ewram = vec![0u8; 0x40000];
    iwram[0x10] = 0xab;
    iwram[0x7fff] = 0x11;
    ewram[0] = 0x22;
    ewram[0x100] = 0xcd;
    assert!(memory::refresh(
        &gba(&mut iwram, &mut ewram),
        CoreMemory::default(),
        game::GBA
    ));
    let mut b = [0u8; 1];
    memory::read(0x10, &mut b);
    assert_eq!(b, [0xab]);
    memory::read(0x8100, &mut b);
    assert_eq!(b, [0xcd]);
    let mut span = [0u8; 2];
    assert_eq!(memory::read(0x7fff, &mut span), 2);
    assert_eq!(span, [0x11, 0x22]);
    memory::clear();
    assert!(!memory::ready());
    assert_eq!(memory::read(0x10, &mut b), 0);
}

#[test]
fn an_empty_map_is_not_ready_yet() {
    assert!(!memory::refresh(&[], CoreMemory::default(), game::GBA));
}

#[test]
fn regions_belong_to_the_emulator_thread_that_built_them() {
    let mut iwram = vec![0u8; 0x8000];
    let mut ewram = vec![0u8; 0x40000];
    assert!(memory::refresh(
        &gba(&mut iwram, &mut ewram),
        CoreMemory::default(),
        game::GBA
    ));
    let elsewhere = std::thread::spawn(|| {
        let mut b = [0u8; 1];
        (memory::ready(), memory::read(0x10, &mut b))
    })
    .join()
    .unwrap();
    assert_eq!(elsewhere, (false, 0));
    memory::clear();
}

#[test]
fn rebuilds_when_the_core_moves_its_memory() {
    let mut iwram = vec![1u8; 0x8000];
    let mut ewram = vec![0u8; 0x40000];
    memory::refresh(
        &gba(&mut iwram, &mut ewram),
        CoreMemory::default(),
        game::GBA,
    );
    let mut moved = vec![2u8; 0x8000];
    memory::refresh(
        &gba(&mut moved, &mut ewram),
        CoreMemory::default(),
        game::GBA,
    );
    let mut b = [0u8; 1];
    memory::read(0, &mut b);
    assert_eq!(b, [2]);
    memory::clear();
}

#[test]
fn a_core_with_no_map_is_read_through_its_system_ram() {
    // Snes9x declares no map; rcheevos lays its 128 KB of WRAM at 0 for the SNES
    let mut wram = vec![0u8; 0x20000];
    wram[0x1234] = 0x5a;
    let ram = CoreMemory {
        system_ram: (wram.as_mut_ptr() as usize, wram.len()),
        save_ram: (0, 0),
    };
    assert!(memory::refresh(&[], ram, game::SNES));
    let mut b = [0u8; 1];
    memory::read(0x1234, &mut b);
    assert_eq!(b, [0x5a]);
    memory::clear();
}
