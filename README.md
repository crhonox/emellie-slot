# Emellie Slot

A Game Boy-centric frontend for Anbernic H700 handhelds running
[BaseOS](https://github.com/pvaibhav/BaseOS): GBA, GBC and GB games, and SNES, PlayStation and
Nintendo DS games too, on a shelf of carts and discs that go down into the handheld when you
play them.

Emellie Slot is a personal fork of [omega-zero](https://github.com/oauramos/omega-zero) by
[@oauramos](https://github.com/oauramos), itself built on
[slot](https://github.com/BrandonKowalski/slot) by Brandon T. Kowalski. What it adds is in the
[changelog](CHANGELOG.md):

- **Nintendo DS**, in [DSperate](https://github.com/beebono/DSperate): the frontend hands the
  handheld over to it and comes back when the game is quit, with DS game cards on their own shelf.
- **RetroAchievements** for every system but the DS (which keeps DSperate's own), signed in from
  Settings → ACHIEVEMENTS.
- **Overlays made for RetroArch**, with the game placed where each overlay expects it.
- No boot animation: the shelf comes straight up.

## Supported devices

One build runs on all of them: it reads the model from BaseOS at boot and lays itself out for its
panel.

| Device | Panel | Status |
|---|---|---|
| RG35XX SP | 640x480 | Main target |
| RG CubeXX | 720x720 | In use |
| RG SP, RG34XX SP, RG34XX | 720x480 | Untested |
| RG35XX Plus / 2024, H, Pro, RG40XX H, V | 640x480 | Untested |
| RG28XX | 480x640, turned | Experimental |

## Quick start

1. Write the [BaseOS image](https://github.com/pvaibhav/BaseOS/releases/latest) for your model to a
   microSD card, put it in **SD1** and boot once.
2. Build the card tree (below) and copy the contents of `dist-device/` to the root of that card.
3. Put your games in `Games/GBA`, `Games/GB`, `Games/GBC`, `Games/SNES`, `Games/PS1` and
   `Games/NDS`. A PlayStation game is a `.cue` with its `.bin`, a `.chd`, a `.pbp` or an `.m3u`;
   a real PlayStation BIOS, such as `scph5501.bin`, is used from `BIOS/` when it is there. A DS
   game is an `.nds`; DS BIOS and firmware dumps (`bios9.bin`, `bios7.bin`, `firmware.bin`) are
   used from `BIOS/nds/` when they are there, and are not needed.
4. Boot. On the shelf, START opens Settings and a tap of SELECT shows every button.

A card that ran omega-zero keeps its settings: `Config/omega-zero.state` is taken on as
`Config/emellie-slot.state` at the first boot.

## Building

Needs Docker with arm64 emulation (`qemu-user-static-binfmt` on Arch) and
[Task](https://taskfile.dev) (`go-task`). The build runs in an arm64 Debian bullseye container,
so nothing else is installed on the host.

```sh
go-task dist:device          # the card tree in dist-device/
go-task deploy:device        # or straight onto a handheld over adb
```

- DSperate's SDL2 is taken from a [spruceOS](https://github.com/spruceUI/spruceOS) checkout at
  `../spruceOS` (`SPRUCE=` to move it).
- Files for the card beyond the build (overlays, configs) go in `../sd-extras`, laid over the tree
  at every build (`SD_EXTRAS=` to move it).
- Labels need ScreenScraper developer credentials compiled in: export `OMEGA_SS_DEVID` and
  `OMEGA_SS_DEVPASSWORD` before building.

## Updates

Emellie Slot updates itself over Wi-Fi from this repository's releases, tagged `es-x.y.z`.

- A boot that comes up on the shelf or the menu, with Wi-Fi on, asks GitHub for the latest
  release once a network is joined, in the background; a toast says when there is a newer one and
  Settings' ABOUT row names it. A boot that puts a game straight back in does not look.
- Settings → About, then A, opens UPDATE: it looks (again) and installs. The release's zip is
  fetched into `.update/` on the card, checked against its SHA-256 and unpacked; Emellie Slot then
  restarts into it. It needs 30% battery or the charger, and some 400 MB free on the card.
- Only `System/` is replaced: games, saves, states, labels and settings stay. The old one is kept
  as `System.old`; a new one that falls over within 30 s of its first start is swapped back out
  (kept as `System.failed`).

To publish one: set `version` in `crates/omega-zero/Cargo.toml` to `x.y.z`, commit, and push the
tag `es-x.y.z`. `.github/workflows/release.yml` builds the card tree on an arm64 runner and
publishes it with its SHA-256 (it refuses a tag that is not the crate's version).

## Credits and licence

- Based on [omega-zero](https://github.com/oauramos/omega-zero) by
  **Otávio ([@oauramos](https://github.com/oauramos))**, itself based on
  [slot](https://github.com/BrandonKowalski/slot) by
  [Brandon T. Kowalski](https://github.com/BrandonKowalski) (GPL-3.0). The code that hosts the
  cores, with their saves, sound and input, is still largely theirs.
- [BaseOS](https://github.com/pvaibhav/BaseOS) by pvaibhav.
- Emulation by [mGBA](https://mgba.io) (MPL-2.0),
  [gpSP](https://github.com/libretro/gpsp) (GPL-2.0) and
  [armGBA](https://github.com/Jacob-Matthew-Cook/armGBA) (GPL-2.0, single-player only) for the
  Game Boys; [Snes9x](https://github.com/libretro/snes9x) (free for
  non-commercial use only) and [Supafaust](https://github.com/libretro/supafaust), Mednafen's
  snes_faust (GPL-2.0), for the SNES; and
  [PCSX ReARMed](https://github.com/libretro/pcsx_rearmed) (GPL-2.0) for the PlayStation; and
  [DSperate](https://github.com/beebono/DSperate) (GPL-3.0) for the DS, drawing through the
  mali-fbdev SDL2 (zlib) built for the H700 by [spruceOS](https://github.com/spruceUI/spruceOS),
  whose pad mapping it also uses. Their licences and patches, and the source of all but mGBA,
  ship in `System/licenses/`.
- Type is [Pixel Code](https://github.com/qwerasd205/PixelCode) by Qwerasd (SIL OFL 1.1); every
  icon is drawn as pixel art. The LCD mask is derived from GigaHerz's LCD3x.
- Cart labels come from [ScreenScraper](https://www.screenscraper.fr), fetched with your own
  account.
- RetroAchievements through [rcheevos](https://github.com/RetroAchievements/rcheevos) (MIT),
  ported from [slot-plus](https://github.com/nickclyde/slot-plus) by nickclyde.

Emellie Slot is licensed under the GPL-3.0-or-later; see [LICENSE](LICENSE). It is written with
Claude, and reviewed and tested by its maintainer.

Game Boy, Game Boy Color, Game Boy Advance, Nintendo DS, Super Nintendo Entertainment System and
Super Famicom are trademarks of Nintendo. PlayStation is a trademark of Sony Interactive
Entertainment. Anbernic is a trademark of its owner. This project is not affiliated with any of
them.
