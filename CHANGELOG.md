# Changelog

## es-1.0.17 (2026-11-08)

- Rumble works again on rumble cartridges, and only on them. The motor is on while the game asks
  for any rumble at all, with 150 ms plays renewed every 100 ms (as es-1.0.14, which was felt on
  the SP). es-1.0.16 turned a weaker rumble into one-frame flicks the motor never spun up for,
  and every core reports a cartridge's rumble as a share of the frame.
- Only games whose cartridge has a motor rumble, read from the ROM header: on GBA Drill Dozer and
  WarioWare: Twisted! (every region), on GB/GBC every MBC5 + Rumble cartridge (Pokémon Pinball,
  Perfect Dark, ...). No other game moves the motor, whatever its core asks.
- Settings > Rumble is gone. An old `rumble=` line in the state file is ignored.

## es-1.0.16 (2026-11-08)

- No Game Boy Player on any core: games play as on a handheld. armGBA's Game Boy Player from
  es-1.0.15 is gone, gpSP no longer picks it from its game list (`cores/gpsp/no-gb-player.patch`;
  its rumble fix from es-1.0.15 went with it) and mGBA's `mgba_force_gbp` is off. Rumble
  cartridges still rumble (Drill Dozer, WarioWare: Twisted!, Game Boy rumble carts).
- The motor is driven as muOS drives it: one force feedback effect at full, playing until
  stopped, switched on and off. A weaker rumble is the motor on for that share of the frames, as
  a rumble cartridge's motor is switched by the game; the 150 ms pulses and the 0x8000 floor
  from es-1.0.10 are gone. Before the motor starts, an effect the kernel dropped is uploaded
  again.
- The rumble log lines from es-1.0.13 are gone.

## es-1.0.15 (2026-11-08)

- armGBA has the Game Boy Player, as mGBA emulates it (`cores/armgba/emellie-gbplayer.patch`):
  found when a game shows the player's logo, it answers the game's transfers, and its rumble
  reaches the motor, so Pokémon Pinball: Ruby & Sapphire vibrates on armGBA too. Not while a link
  cable or the Wireless Adapter is on the port. Its state sits in padding the console struct
  already had, so save states keep their layout and older ones still load.
- gpSP's Game Boy Player rumble no longer runs on or gets stuck
  (`cores/gpsp/gbp-rumble-command.patch`): it counted the player's 16-word exchange round and
  round, drifted whenever a frame had another number of transfers, and read handshake words as
  "start". The exchange now starts over every frame, and only the start command (0x22) starts
  the motor, as in mGBA.

## es-1.0.14 (2026-11-08)

- Rumble works: the force feedback effect is uploaded again before every pulse. The kernel drops
  a file's effects whenever any copy of it is closed, and every program the frontend starts
  (curl, amixer, the Wi-Fi scripts) closes the copy it inherited, so the effect from boot was
  gone by the time a game rumbled and updating it failed with "Invalid argument".
- mGBA plays Game Boy Player games as on one (`mgba_force_gbp`), so Pokémon Pinball: Ruby &
  Sapphire and the others that rumble through it vibrate; it stays off over the link cable,
  which uses the same port. armGBA has no Game Boy Player.

## es-1.0.13 (2026-11-08)

- Rumble shows in the log (`emellie-slot.log`), to find why games with a rumble cart do not
  vibrate: when the core asks for rumble, when it starts and stops (six lines a game at most),
  and when it is asked for with Settings > Rumble off.

## es-1.0.12 (2026-11-08)

- DS_Overlay, the single-screen grid overlay from es-1.0.11, fine-tuned.

## es-1.0.11 (2026-11-08)

- DSperate's single-screen grid overlay (`System/dsperate/grid_s0.bmp`) is now DS_Overlay, in
  place of the 2.5x DS_3x_Grid_25 grid. It shows in the same cases as before: integer scale off,
  seam blend or blend_linear, the screen filling the 640x480 panel. The original PNG is kept in
  `emellie_new_assets/ds-grid-640x480.png`.

## es-1.0.10 (2026-11-07)

- Rumble works the way spruceOS drives the Anbernic XX motor: 150 ms force feedback pulses,
  started again every 100 ms while the core asks, at 0x8000 to 0xFFFF (the core's strength
  scaled into that range). The old single effect of length 0 never moved the motor on the RG SP
  or the CubeXX.

## es-1.0.9 (2026-11-07)

- DSperate is at `a165843` (3.3.0: slot-2 GBA cartridge and Rumble Pak, analog patches, the 3D
  layer on OpenGL ES 3.1, translations), with `emellie-hotkeys.patch` brought onto it.
  `pad.stylus_dpad` is now `pad.stylus_hold` in `System/dsperate/pad-*.ini`; DSperate still reads
  the old name in a player's config.
- `sdl-2.0.14-no-kmsdrm-wminfo.patch` also covers DSperate's Rumble Pak check, which needs
  SDL 2.0.18's `SDL_GameControllerHasRumble`; built against bullseye's SDL 2.0.14 it asks with a
  zero-length rumble instead.
- armGBA is at `334c20f` (games patched for SRAM reproduction cartridges get SRAM).

## es-1.0.8 (2026-11-07)

- A 2.5x subpixel grid over DSperate's single screen with integer scale off and seam blend or
  blend_linear, where the screen fills the 640x480 panel (`System/dsperate/grid_s0.bmp`, set by
  `DS_SINGLE_GRID` in `nds.sh`). `cores/dsperate/emellie-single-bezel.patch` has the screen scaled
  into a side buffer and writes it with the grid into the frame. The frame from es-1.0.6 still
  shows only with integer scale on. The original PNG is kept in `emellie_new_assets/ds-grid-640x480.png`.
- DS games, ports and PICO-8 carts go first in Recents when they are handed over to their own
  emulator; they never reached the "playing" state that recorded the others.

## es-1.0.7 (2026-11-05)

- The single-screen DS frame (`System/dsperate/bg_s0.bmp`) is now steward-fu's DraStic Advanced
  background, edited, as packaged in Bright-04/muos-nds-overlay; its window is exactly the 2x
  screen. It is no longer credited to spruceOS.

## es-1.0.6 (2026-11-05)

- A DS frame round the screen in DSperate's single-screen layout with integer scale on, where the
  screen is shown at twice its size (spruceOS's DS background, `System/dsperate/bg_s0.bmp`).
  `cores/dsperate/emellie-single-bezel.patch` paints it into the letterbox as it is cleared, so
  it costs no frame anything; any other layout or scale leaves the letterbox black as before.

## es-1.0.5 (2026-11-05)

- The main menu's Collections icon is named "Colls.", so its name fits its column on 640x480.

## es-1.0.4 (2026-11-05)

A new look for the menus and the boot.

- New icons for the main menu's Recents, Favorites, Collections, Games and Settings: one set,
  96x96, for every screen (also in `System/menu-template`).
- The main menu's and the console grid's names are written in Pixel Code, as the rest of Emellie
  Slot is, in place of spruce's font; at 18 px on every screen so they stay sharp. The menu
  template's `font.ttf` is Pixel Code too.
- A new boot logo, on a dithered grey: drawn for 640x480 and laid out again on the same pattern
  for 720x480 and 720x720 (`examples/bootlogos.rs` now repeats a 2x2 background).

## es-1.0.3 (2026-11-05)

armGBA is back as the GBA's third core, to try games on: single-player only.

- The GBA's cores are mGBA, gpSP and armGBA ([armGBA](https://github.com/Jacob-Matthew-Cook/armGBA)
  @ e4b8d8e, built from source by `task core:armgba`, GPL-2.0, its source in `System/licenses/`).
  The core picker keeps its two sockets: Right from gpSP turns the cores along, gpSP into the
  first socket and armGBA into the second; Left from the first turns them back.
- No link on armGBA: its link mode is off, and SELECT + MENU says there is no link support.
- armGBA's colour correction showed washed out, much paler than mGBA's: its table mixed
  Pokefan531's primaries at a gamma of 2.2 in and out, which only takes saturation away.
  `cores/armgba/emellie-gba-colours.patch` reads the screen a gamma of 1.0 darker at 93%
  luminance, as his gba-color shader does, which lands within a few levels of mGBA's.

omega-zero is now **Emellie Slot**, a personal fork of it.

Updates over the air: Emellie Slot finds its own releases (`es-x.y.z` on GitHub) and installs them.

- A boot to the shelf or the menu with Wi-Fi on looks for a newer release once a network is
  joined, in the background, and a toast says when there is one; Settings' ABOUT row names it.
  A boot that puts a game straight back in, or a return from a DS game, port or PICO-8 cart,
  does not look.
- A on About opens UPDATE: the version installed, the latest, and Check for Update or Install
  Update. An install downloads the release's zip into `.update/`, checks its SHA-256, unpacks it
  and restarts; it needs 30% battery or the charger, and B waits until it is done.
- `launch_frontend.sh` swaps the new `System/` in, keeping the old as `System.old`, and goes back
  to it when the new one cannot run or falls over within 30 s of its first start
  (`card/test-update.sh`). Nothing outside `System/` is touched.
- The version is Emellie Slot's own, starting at es-1.0.0; `release.yml` publishes on an
  `es-*` tag that matches it, with the zip's SHA-256 beside it.

- The program on the card is `System/emellie-slot`, its log `emellie-slot.log` and its state
  `Config/emellie-slot.state`; a card's `Config/omega-zero.state` is taken on at the first boot,
  so nothing set is lost. About, the start notice, the boot logo, ScreenScraper's software name
  and RetroAchievements' user agent say Emellie Slot.
- On a handheld with sticks (RG CubeXX, RG40XX V) the left stick works the d-pad in every
  system the frontend runs, PS1 included, as well as the d-pad itself: pushed over half its
  reach it holds the direction, and lets go back under a third.
- Over the selected cart on every shelf stands its file name, without the extension, so two
  games wearing the same label (a game and a romhack of it) can be told apart.
- SELECT+L1 and SELECT+R1 no longer load and save a state; MENU twice still opens the save
  states.
- No opening at boot: the shelf comes straight up after the boot logo. A game that took the
  frontend down is still named over the shelf at the next boot.
- The boot logo is EMELLIE SLOT in Pixel Code, for each panel.

- In the core picker, B closes the picker; it no longer took the shelf back to the console grid
  with the picker left on the screen.
- DSperate is at `e04b806`, with the hotkeys patch brought onto it.

The NES, the Mega Drive and PICO-8 join the shelf, as spruceOS runs them.

- NES and Famicom games go in `Games/FC` (`.nes`, `.fds`, `.unf`, `.unif`), Mega Drive and
  Genesis games in `Games/MD` (`.md`, `.gen`, `.smd`, `.bin`), the folders spruceOS keeps them
  in. The NES plays on FCEUmm and the Mega Drive on PicoDrive, as in spruceOS; X on the shelf
  picks Nestopia UE or Genesis Plus GX instead. A Famicom Disk System game wants
  `BIOS/disksys.rom`.
- A NES game takes X and Y as B and A, as the GBA does. A Mega Drive game's A, B and C are Y, B
  and A, and a six-button game's X, Y and Z are L1, X and R1.
- Both keep square pixels as the SNES does: the NES 512x480 on 640x480 and 720x675 across the
  RG CubeXX's 720x720, the Mega Drive 640x448 and 720x504.
- Their carts stand in the SNES's shells for now, a NES cart in the Famicom's in grey, a Mega
  Drive cart in the American one in black. Labels come from ScreenScraper, a NES dump looked up
  with and without its iNES header; RetroAchievements counts both.
- PICO-8 carts go in `Games/PICO8` (`.p8`, `.p8.png`), and run in the player's own PICO-8: the
  Raspberry Pi build's `pico8_64` and `pico8.dat` in `BIOS/`, which Emellie Slot cannot ship.
  `Games/PICO8/Splore.splore` opens Splore, PICO-8's own cart browser, over Wi-Fi. As a DS game
  is, a cart is handed over to `System/pico8.sh`; MENU held a second goes back to the shelf,
  the volume keys step the volume, and B and A are PICO-8's O and X. Its settings and the carts'
  saves are in `Saves/PICO8`. PICO-8 carts stand as DS cards with their titles.
- The console grid has spruceOS's icons for the three, and a card's own menu theme can give
  them as `fc`, `md` and `pico`.
- `task core:picodrive`, `core:gpgx`, `core:fceumm` and `core:nestopia` build the four cores
  from their pinned sources, and `task vendor:pico8` the shim PICO-8 is run with; their
  sources and licences ship in `System/licenses`.

The DS joins the shelf: its games run in DSperate, which omega-zero hands the handheld over to.

- DS games go in `Games/NDS` (`.nds`). They stand on a shelf of their own, with their labels from
  ScreenScraper, looked up by the game
  code in the cart's header rather than by a CRC of hundreds of megabytes.
- A DS game is not a libretro core's: once its cart is in, omega-zero writes what to run to
  `$OMEGA_HANDOFF` and leaves, and `launch_frontend.sh` runs DSperate through `System/nds.sh`.
  When DSperate quits, omega-zero starts again without its opening, the game selected on the
  shelf. Switched off in a DS game, the handheld goes back into it at the next boot, as it does
  for any game.
- DSperate draws straight to the panel through the mali-fbdev SDL2 in `System/dsperate/lib/`,
  under the performance governor, 640x480, 720x480 and the RG CubeXX's 720x720 alike. Its
  settings are seeded once for each kind of pad into `Config/dsperate/nds-nostick.ini`,
  `nds-1stick.ini` or `nds-2stick.ini`, so one card played in an RG35XX SP and an RG CubeXX
  keeps the settings of each; battery saves go to `Saves/NDS` and states to `States/NDS` for
  both, its own resume included.
- In a DS game, MENU+START opens DSperate's pause menu, and MENU held or MENU+SELECT goes back
  to the shelf. With no sticks (RG35XX SP), MENU+Y gives the d-pad to the stylus until pressed
  again and R2 touches; with two (RG CubeXX), the left stick is a d-pad, the right one moves the
  stylus, and R2 or R3 touches. L2 swaps the two screens, MENU+R1 fast-forwards, MENU+B is the
  microphone and MENU+X closes the lid. These hotkeys are put back at every start, so one bound
  in DSperate's own menu lasts that game. SELECT on the DS shelf shows all of it, for the
  handheld it is on.
- The DS shelf stands DS game cards, drawn from a picture of a real one: the game's label
  fills the well under the NINTENDO DS band, a label from ScreenScraper of the whole sticker
  keeping only its art, and a game with no label wears its title there.
- RetroAchievements for every game omega-zero runs itself: GBA and Game Boy on mGBA and gpSP,
  SNES on Snes9x, and PlayStation discs as a `.cue` with its `.bin`, or an `.m3u`. Settings
  has an ACHIEVEMENTS screen to turn it on or off and to sign in, the user name and password
  typed on the keyboard as the Labels account is; once signed in, a token is kept in
  `Config/retroachievements.txt` in place of the password. A banner over the game says what
  happens: signed in, the game found and how many of its achievements are earned, each one
  earned, the game mastered, and when the handheld goes offline and back. Softcore only, so
  save states, rewind and fast forward stay; a state loaded or a rewind starts each
  achievement's wait over. Supafaust lets no frontend read the SNES's memory, so its games
  earn none, nor do `.chd` and `.pbp` discs yet; the DS keeps DSperate's own achievements.
  Ported from [slot-plus](https://github.com/nickclyde/slot-plus) by nickclyde, on
  [rcheevos](https://github.com/RetroAchievements/rcheevos) (MIT).
- An overlay can say where the game goes under it, as RetroArch's custom viewport does: an
  `Overlays/<system>/<w>x<h>.ini` beside its png, with `x`, `y`, `w` and `h`, draws the game
  there through the sharp filter, whatever its picture mode. So a Game Boy can stand at 4x,
  640x576, under an overlay for that on the RG CubeXX's 720x720.
- An overlay is refused only when solid pixels hide more than 1% of the game: an LCD grid or a
  screen's shadow laid over it is drawn, as is a frame a pixel over the picture's edge.
- `task core:dsperate` builds DSperate from its pinned source and stages the SDL2 from a spruceOS
  checkout (`SPRUCE=`, `../spruceOS` by default). The device build's container is now asked for
  as arm64, and the build installs its packages from Debian's snapshot of bullseye once the
  mirrors have dropped it.

## omega-zero 1.2.0

Supafaust runs SNES games, a game that takes the console down can no longer trap it at boot, and
labels take a quarter of the room and no longer hold the boot.

- SNES games run on Supafaust. Chosen with X, Supafaust took omega-zero down on a game's first
  frame and left the console on its boot screen: it forgets the defaults of its own options once
  a game has loaded, and read an empty one back. omega-zero now answers every option a core
  declares, with the core's own default where omega-zero sets none, as any libretro frontend does.
- A game whose core takes omega-zero down as it starts is no longer opened again at every boot.
  The next boot leaves it on the shelf, selected, rather than resume it, so the console can no
  longer get stuck on its boot screen over one game. That boot says so after the opening, on its
  terminal: which game went down, on which system and core, and how to try it another way. A, B
  or START goes on to the shelf once its prompt is up.
- When omega-zero goes down, the card keeps what it last said in `crash/omega-zero.log` and its
  whole memory as it went down in `crash/omega-zero.core.gz`, for a debugger. It starts again once
  the core is written, some seconds later. A crash within ten minutes of the last keeps the first
  core, and none is written when it would leave the card less than 512 MB.
- Labels take about a quarter of the room on the card. Once a label is cut to its cart, the
  label itself is shrunk to the smallest size that still covers every face its system draws it
  on, since no face is drawn bigger than that; until now each one was kept 600 pixels wide. Every
  cart looks as it did. Labels already on the card, or copied there by hand, are shrunk once in
  the background after the boot.
- Labels fetches each label at about two thirds of the width it did, so a sync downloads about
  half as much.
- The boot no longer reads any label before the shelf comes up. Every cart starts in its
  generated label and is drawn in its own on threads that stand back from the shelf and the
  game, the carts on screen first, a few a frame, while the opening plays.

## omega-zero 1.1.0

The SNES and the PlayStation join the Game Boys: their games run, and they stand on shelves of
their own, SNES carts and PlayStation discs modelled like the Game Boy carts.

- SNES games go in `Games/SNES` (`.sfc`, `.smc`). A game from the USA stands as the American SNES
  cart; one from Japan, Europe or anywhere else stands as the Super Famicom cart, as it was sold.
  The region comes from the ROM's header, a tag in the file name such as `(USA)` or `(Japan)`
  wins over it, and `cart_shell.ini` can set a cart's shape to `ntsc` or `sfc`.
- Two SNES cores, Snes9x and Supafaust, chosen per cart with X, as for the GBA's two. Snes9x is
  the default.
- PlayStation games go in `Games/PS1` as `.cue` with their `.bin`, `.chd`, `.pbp` or `.m3u`, also
  one folder down. They run on PCSX ReARMed, which carries its own BIOS; a real one in `BIOS/` is
  used when it is there.
- A PlayStation game stands on its shelf as its disc. A game on several discs is one entry, its
  discs stacked one behind another with a gap between them, and the whole stack goes down into
  the handheld. Files named `(Disc 1)`, `(Disc 2)` and so on are put together as one game, with
  a playlist made for them under `Config/playlists/PS1/`. In a game, SELECT+MENU opens DISC to
  put another disc in, and the disc in the drive is the one there when the game is resumed.
- In a SNES or PlayStation game, X, Y, L1 and R1 go to the game. A PlayStation game takes L2 and
  R2 too: fast-forward is SELECT+R2 held, and it has no rewind. The SNES keeps L2 rewind and R2
  fast-forward.
- The SNES picture is drawn at twice its size; the PlayStation's fills the screen at 4:3. A
  game made for 50 Hz runs at 50 Hz.
- Labels fetches SNES labels, and the label of each PlayStation disc.
- Snes9x is free for non-commercial use only; Supafaust and PCSX ReARMed are GPL-2.0. Their
  licences and source are in `System/licenses/`.

## omega-zero 1.0.1

The boot no longer waits on the labels: each one is cut to its cart once, as it is fetched.

- A card with hundreds of labels boots as quickly as one with none. Each label is cut to its
  cart's face once, right after Labels fetches it, and kept in `Labels/<system>/.cut/`; a boot
  draws the carts from those small cuts, on every core. Until now each boot decoded every
  full-size label before its first frame, and with a few hundred of them the boot logo stayed up
  long enough to look frozen. Labels already on the card, or copied there by hand, are cut once
  in the background after the boot, the carts on screen first, and wear a generated label until
  then.

## omega-zero 1.0.0

Labels fetch in the build you download: omega-zero now has its own ScreenScraper key.

- Labels, in Settings, fetches the scanned label of every cart that has none in the release
  build too. Until now a release carried no ScreenScraper developer key, so the screen said it
  could not reach ScreenScraper. You still sign in with your own ScreenScraper account.

## omega-zero 0.0.4

The slot under the shelf is gone: carts go down into the handheld, and the game comes on like a
tube TV.

- The slot under the shelf is gone: a cart goes into the handheld itself. From the middle of the
  shelf it rises a little, then is pulled down out of the bottom of the screen, and the game
  comes on as a tube TV does: a dot, a line across, then the picture opening white and fading to
  its colours. Leaving a game plays it backwards, and a cart the core turns away goes back out
  the same way under its warning sign.
- The carts stand in the middle between the top bar and the bottom bar, at their usual sizes. A
  Game Boy cart no longer grows on its way in.
- The shelf's system, GBA, GB or GBC, is in a pill between the bottom bar's buttons, always. It
  rises in as L1 or R1 changes shelf, and a letter jumped to with Up or Down shows in it for two
  seconds.
- Config/theme.txt's `recess` and `opening` coloured only the slot and are no longer read.

## omega-zero 0.0.3

Settings in the style of Mega Man Zero's menus, with Network, Bluetooth, Theme and a POWER menu;
every screen drawn in the UI kit's 8/16-bit pixel art; carts as low-poly models drawn the way a
PlayStation drew 3D; and an opening after Mega Man X5's boot terminal at power-on.

- Everything around the shelf is on the menus' 2-pixel grid: the slot's edges and its scoop, the
  slot's name in the menus' type and the theme's dim colour, the top bar's clock, figure and
  "ssh" (drawn bold the kit's way), and a generated label's title, now always Pixel Code at its
  own size. A cart the core turns away comes out under a pixel-art warning sign in the theme's
  colour. The background drifts two pixels at a time, a refused cart shakes a whole pixel at a
  time, and the boot screen fades out in four steps.
- The save states, MENU ×2 in a game, are drawn in the menus' pixel art. The shelf's top bar
  stands over the screenshot in place of a dark strip of their own; a SAVE STATES panel at the
  top in the middle says how long ago the save shown was made, with a dot for each save under
  it, the one shown lit; and the bar along the bottom has B RETURN and A LOAD. Nothing over the
  screenshot is see-through any more.
- Save states only save and load: Y no longer deletes the save shown, and X no longer undoes the
  last save or load.
- `<omega-hints>` joins the UI kit: a screen's keys on one line of the kit, on a ground of their
  own, for where the bar's hints would sit on a picture.
- The link screen (SELECT + MENU in a game) is drawn in the menus' kit: the theme's backdrop, a
  LINK panel with ROLE (HOST or JOIN, Left and Right swap it) and MODE (CABLE or WIRELESS,
  SELECT switches it where the game can), what is happening as a note under them, and the
  bottom bar with RETURN and LINK, SELECT MODE and ◀▶ SWAP between them. The plug, its cable,
  the port, the wireless adapter, its radio arcs and the clicks of a plug going home are pixel
  art on the 2-pixel grid, the plug still the purple of the real cable. A link that fails lifts
  the plug out and shakes it a pixel either way instead of tilting it, and the arcs pulse in four
  steps.
- The carts are low-poly models drawn the way a PlayStation drew 3D: a face and a side for
  every cut of the outline, each lit on its own, corners that land on the 2-pixel grid as they
  turn, textures mapped without perspective, and colour in 15 bits through the PlayStation's
  dither. The one you are on sways twenty degrees either way, seen from closer than before, so
  its side and the step under a GBA cart's grip show. The cart going into the slot and the core
  picker's lid are the same model. A cart is one draw instead of up to seventeen.
- The volume, brightness, blue light and rewind show as a gauge drawn like the menus: the
  level's name in a tab, then a cell for each step, lit as far as the level reaches, and its
  icon, on the 2-pixel grid in the colours of Settings > Theme, with no dark band behind it. It
  stands at the top of the screen, on the shelf just under the top bar, and fades out in four
  steps. What just happened shows the same way, a small panel titled SAVE STATE, LINK or COLOUR
  over "State saved", "Link ended", "Correction on" and the rest.
- Fast forward's badge in the corner is a key cap holding ▶▶ and the speed, ×2 to ×6, lit
  while it is latched. A link's badge shows a link, lit while hosting and dark when joined, and
  broken once the other handheld is lost.
- The core picker is pixel art on the 2-pixel grid: the open cart in its own plastic, the board,
  the ROM chip marked with the game's title, the sockets and the chip, with outlines and flat
  colours. The title and the cores' names are in Pixel Code at its own size and spacing. The
  chip hops between the sockets a pixel at a time, level, and the shadows under it and under
  the lid are checker dither.
- About is a window like Settings, drawn in the menus' kit in place of the sticker. OMEGA-ZERO
  lists the version, the build (its commit, with -DIRTY after it for a build with changes not
  committed), the author, @oauramos, the handheld as BaseOS names it, and how full its
  battery is and whether it is charging. CREDITS under it says what omega-zero is made with and
  on, a line each. The barcode, the ANBERNIC wordmark and the made-up rating lines are gone.
- A bar along the bottom of the shelf: SELECT CONTROLS on the left, START SETTINGS on the
  right. The slot sits a bar higher to make room for it.
- Game Boy carts stand at 80% of their size on the shelf and grow to full size as they go into
  the slot.
- The handheld starts on an old PC's BIOS screen, after Award's, in place of the logo: the
  OmegaZero BIOS, "An Anbernic Ally", oauramos Software Inc., the H700, a memory test, BaseOS's
  Plug and Play Extension and Press START to enter SETTINGS, with the OmegaZero logo in 16
  colours where Award put its Energy Star. It is set in the opening's letters, where its
  terminal prints. `task bootlogos` renders it; `task bootlogos:logo` renders the old logo.
- An opening after Mega Man X5's NOM ENGINEER WORK SYSTEM boot screen, told about the handheld
  it runs on. omega-zero's first frames show the boot screen where the handheld drew it, and it
  fades to black. Then a terminal fades in, in X5's wide mint letters and its beeps: the model,
  the memory counted up, the H700, the Mali-G31, the battery and its charge, a login as root,
  `>omega -boot -a` typed out and three files read. After ten seconds it fades out and the shelf
  fades in. Any button cuts it short, and that press does nothing on the shelf. Once the
  shelf shows through, a press ends the opening at once and works as usual. POWER and the lid
  end it at once. A boot back into a game, or to the clock, cuts as before.
- START on the shelf opens Settings, a window in the style of Mega Man Zero's menus, in place of
  the old quick menu. CONFIGS holds Fast Forward (2X, 3X, 4X or 6X), Fast Forward Sound, Colour
  Correction, Rumble and Labels; GENERAL holds Date & Time, Network, Theme and About.
- A row that Left and Right change shows ◀ ▶ while it is lit. A row that opens a screen ends
  in ›.
- A tap of SELECT on the shelf opens CONTROLS: what every button does, on the shelf and in a
  game, on one panel. SELECT held with another button works as before.
- The Wi-Fi screen is now Network, under Settings. Its NETWORK panel switches Wi-Fi, Network
  Time and SSH. AVAILABLE NETWORKS shows each network's signal, a padlock on those with a
  passphrase, and CONNECTED, JOINING or SAVED. Y forgets a saved network.
- Network looks for networks only when X is pressed, once each time, and the bar says X SCAN.
  Until then it lists the network joined and the saved ones, with a note to press X. It no
  longer scans every 12 seconds while it is open, which cost battery and, on the RG35XX SP,
  came before reboots that sometimes hung.
- Wi-Fi, Network Time and SSH are each ON or OFF, and Y on any of them opens its panel. Wi-Fi's
  shows the network joined, its signal, the handheld's address, `<name>.local` and the radio's
  MAC address. Network Time's shows whether a time server has set the clock, and the date, time
  and zone. SSH's shows how to log in from a computer: the address, `<name>.local`, user `root`,
  the password and port 22. SFTP uses the same login.
- SSH ON starts the SSH server and OFF stops it, leaving a login already made to carry on; the
  card remembers which.
- The first time omega-zero runs on a card whose `baseos.conf` sets no `ssh_password`, it gives
  root a password of six random digits, at once and in `baseos.conf`, so BaseOS keeps it. A
  password you set there yourself is kept.
- Cart Labels is no longer on the Wi-Fi screen: it is Labels, in Settings.
- Theme, Date & Time and Labels are panels over the background, and the passphrase
  keyboard is a window.
- In the menus the bottom bar says B RETURN and A CONFIRM. A screen's own keys sit in the middle
  of it: Y INFO and Y FORGET on Network, START FETCH and Y SIGN OUT on Labels. Between its two
  ends there is nothing but those keys, over the background.
- Every move between screens slides, over a background that keeps moving. Going into a screen,
  the one you leave goes out to the right and the new one comes in from the left; coming back is
  the mirror. Leaving the shelf, the top bar goes up, the carts sideways and the slot down.
  Settings, from the shelf, slides the other way round: the carts go out to the left as it comes
  in from the right, and it goes back out to the right. Inside Settings every screen slides the
  usual way.
- The menus, both bars and the slot take their colours from Settings > Theme > Colour.
  Night is grey; Amber is Mega Man Zero's own colours.
- Theme is what Background was: the background's pattern, its motion and its colour. It also
  picks the font of the bottom bar's big words, Bold or Thin.
- Restart no longer freezes the console after Settings > Network has been open. The RTL8821CS's
  Wi-Fi driver hung the kernel's shutdown when it had just gone into its power saving, as it does
  after scanning for networks, so the console stayed frozen until it was reset. omega-zero now
  lets go of the driver itself before a restart or power off: wpa_supplicant and the DHCP client
  stop, the interfaces go down, and the device is unbound from its driver.
- The top bar lights Wi-Fi and ssh within a second of a network being joined, and puts them out
  as quickly when it is lost. After boot they stayed dark until a menu was opened.
- Bluetooth, under Network in Settings, on handhelds that have it. Its screen switches it on and
  off and, while it is on, lists the devices paired, then those a look finds. A pairs with a new
  device and connects it, connects a paired one, or disconnects one connected; Y forgets a
  paired one. Settings names the device connected. The card remembers whether Bluetooth is on,
  bringing it up at boot, and the top bar lights its icon while it is. Pairing asks nothing on
  screen, which most gamepads and headphones take. Game sound over Bluetooth headphones and
  Bluetooth gamepads as controllers are still to come.
- The Bluetooth screen looks for devices only when asked: it opens on the paired devices and
  "Press X to look for devices.", and X SCAN looks for 12 seconds, saying "Looking for devices…"
  while the devices it finds join the list, which keeps them until the screen is left. Devices
  are listed by name: one found that gives none, most often a beacon nobody can pair with, is
  left out rather than shown by its address.
- Holding POWER opens POWER, a panel like the other menus over the background, with RESTART and
  POWER OFF and B RETURN and A CONFIRM on the bar; B goes back to where you were, a game
  included. It comes down from the top as the screen under it drops away, and goes back up. After
  a choice, in place of the black screen, a RESTART or POWER OFF panel comes down the same way and
  says "Restarting…" or "Turning off…" until the handheld goes down, and the bar sinks away.

### For developers

- `assets/ui/flow.html` walks every new screen with the slides the device draws, and is the
  reference they are built to. Keys stand in for the buttons, slow motion runs the slides five
  times slower, and the URL's hash picks the screen, colour, panel size and any moment of a
  slide: `flow.html#screen=network&from=settings&t=0.5&colour=amber&shot=1`.
- The storybook has three new components: `<omega-row>`, `<omega-list-row>` and `<omega-bar>`.
- And four for pages, which the site is now drawn with: `<omega-button>`, `<omega-screen>`, `<omega-card>`, `<omega-code>`.
- The kit is in Rust too, in `crates/omega-ui/src/kit/` (palette, pixel, modal, panel, action,
  row, list_row, bar, text, layout). `tests/suite/kit/` holds it to Chrome's screenshots of the
  HTML kit pixel for pixel; `make-refs.sh` takes them again.
- Each menu screen's input and drawing is a file of its own in `crates/omega-zero/src/screens/`,
  its faces are made in `frontend/sync_*.rs`, and `nav.rs` slides between them.
- `card/System/bluetooth.sh` drives the radio for omega-zero (up, down, status, scan on or off,
  pair, connect, disconnect, forget) and prints each device as a tab-separated line;
  `card/test-bluetooth.sh` tests it against fakes of BlueZ's tools.
- The daemons wifi.sh and bluetooth.sh leave running, and a dropbear omega-zero starts again,
  no longer hold the framebuffer open or share omega-zero's session and signals.
- Now that root's password is random, `task deploy:wifi` and `task log:wifi` log in with
  `CONSOLE_PASSWORD` or, without it, the first line of a git-ignored `.console-password` at the
  top of the repo.

## omega-zero 0.0.2

- A cart leaning half off the screen is drawn whole on every GPU: some drivers dropped it for a
  frame. Turning carts go through a shader of their own and everything else through the one it
  always had, so the shelf costs no more to draw than before carts turned.

### For developers

- `task deploy:wifi` builds omega-zero, copies it onto a console on the home network over SSH
  and reboots into it; `task log:wifi` prints its log. `PROFILE=device-dev` builds in seconds.
- Each crate's tests build as one binary, and the suite runs in about 42 s instead of 130 s.

## omega-zero 0.0.1

The first release, for the RG35XX SP and the other H700 handhelds BaseOS runs on.

### Every panel, natively

- omega-zero lays itself out for the panel it finds, from the model BaseOS names: 640x480 on
  the RG35XX SP instead of the RG SP's 720x480 picture shrunk to fit.
- On a 640x480 panel, GBA games fill the width (640x427) through a sharp scaler, over the
  console's body carrying the GAME BOY ADVANCE logo. Game Boy games stay at 3x through the LCD
  mask.
- Your own overlays: a `.png` the size of the screen in `Overlays/GBA`, `Overlays/GB` or
  `Overlays/GBC`, laid over the game. One that would cover the game is ignored.
- The boot logo is only ever replaced with one the size of the device's own. Logos ship for
  640x480, 720x480 and 720x720 panels.
- Rumble works on devices whose motor is the PMIC's `moto` pin rather than a force feedback
  node.
- Hosting a link works on devices whose Wi-Fi driver gives no second interface.
- A panel mounted on its side is turned in software (the RG28XX).
- `System/probe.sh` writes a hardware report, also from an empty `RUN_PROBE` file at boot, for
  device reports.

### Wi-Fi, SSH and labels

- Wi-Fi in the quick menu: switch it on, pick a network and type its passphrase on an
  on-screen keyboard. Networks are remembered in `Config/wifi.conf` and joined again at boot.
- Once joined, the Wi-Fi screen shows the address and name to reach the handheld over SSH and
  SFTP.
- Network Time keeps the clock right through BaseOS whenever Wi-Fi is joined.
- A link takes the radio over from home Wi-Fi and gives it back when it ends.
- Handhelds without Wi-Fi don't show the row.
- Cart Labels, on the Wi-Fi screen: sign in with your ScreenScraper account and fetch the
  scanned label of every cart that has none, from the ROM's own region first. Labels go on the
  shelf as they arrive. Yours are never replaced.

### A new look

- Every word on screen is in Pixel Code, a pixel font set on its own grid so it stays sharp.
- A top bar over the shelf: Wi-Fi, Bluetooth and ssh on the left, lit while they are up; the
  clock in the middle; the battery on the right.
- The shelf's background is yours to pick under START > Background: a grid, raised plates
  (riveted or not), dotted lines, a dithered grid, bits or a plain colour, drifting or still, in
  eight colours, every pattern in the same 2x2 pixels as the words over it. A wallpaper on the
  card still works.
- The slot below the shelf is bigger: a cart slides into it at full size instead of shrinking.
- The quick menu scrolls when its rows outgrow the screen.
- On the shelf START opens the quick menu and X the core picker; MENU opens nothing there.
- The carts on the shelf are solid. The one you are on floats and sways ten degrees either way,
  showing its edge, and every cart leans into a scroll. It squares up as it slides into the
  slot or opens for the core picker.
- The boot logo is the OMEGA ZERO title in pixel art, edged in white, over the shelf's grid.
  Its source is `assets/logo/omega-zero-logo.html`; `task bootlogos` renders it for every
  panel, and `STYLE=smooth` renders the drawn-straight version instead.
