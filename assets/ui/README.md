# omega-zero UI

The kit omega-zero's menus are drawn from: pixel-art components rebuilt in HTML, the reference for
how each screen looks before and while it is drawn on the device. Open
[`storybook.html`](storybook.html) straight from disk to see every component, what it does and how
to use it, and screens put together from them. [`flow.html`](flow.html) walks every menu screen
and the slides between them.

```
assets/ui/
├── storybook.html         every component and screen, live
├── flow.html              every menu screen, and the slides between them
├── flow/                  what flow.html is drawn and driven with
│   ├── flow.css           the page, and the screen in art pixels
│   ├── screens.js         FlowScreens: what each screen does with a button
│   ├── player.js          the slides, the keys and the URL's hash
│   ├── art.js             FlowArt: placeholders for what Rust draws, the arrows and the …
│   └── stubs.js           stand-ins for omega-row, omega-list-row and omega-bar, if missing
├── foundation/            what every component draws with
│   ├── fonts.css          Pixel Code Regular, cut to Latin-1 and inlined
│   ├── tokens.css         the art pixel (--u) and the shelf's backdrop
│   ├── pixel.js           OmegaPixel: sprites, 9-slices, letters, the base element
│   ├── backdrop.js        OmegaBackdrop: the patterns in Settings > Theme > Background
│   └── palette.js         OmegaPalette: colours from Settings > Theme > Colour
└── components/
    ├── omega-modal.js     <omega-modal>     the menu window, a frame that stacks its content
    ├── omega-panel.js     <omega-panel>     a titled section for inside it
    ├── omega-row.js       <omega-row>       one setting in a panel, lit under the cursor
    ├── omega-list-row.js  <omega-list-row>  a Wi-Fi network or Bluetooth device in a list
    ├── omega-bar.js       <omega-bar>       the bottom bar: two actions, key hints between them
    ├── omega-gauge.js     <omega-gauge>     a level on one line: a title tab, then cells and an icon
    ├── omega-toast.js     <omega-toast>     what just happened: a lone panel's tab and one line
    ├── omega-badge.js     <omega-badge>     what a game is doing: a key cap, a sprite and a word
    ├── omega-saves.js     <omega-saves>     which save state the switcher shows: its age, a dot each
    ├── omega-hints.js     <omega-hints>     keys on a line of their own: a title tab, then the keys
    ├── omega-link.js      <omega-link>      the link screen's art: the port, the plug, the adapter …
    ├── action-left.js     <action-left>     the bottom bar's left action
    ├── action-right.js    <action-right>    the same, mirrored
    ├── omega-pill.js      <omega-pill>      the bar's pill on its own: the shelf's system
    ├── omega-button.js    <omega-button>    a page's link or button, drawn as the bar's pill
    ├── omega-screen.js    <omega-screen>    a screenshot in a handheld's bezel
    ├── omega-card.js      <omega-card>      a feature: an icon, a title and a few lines
    ├── omega-code.js      <omega-code>      lines to read or type exactly, scrolled in a box
    └── omega-tabs.js      <omega-tabs>      a page's tabs, drawn as a row of panel titles
```

## Using it

A page loads the foundation, in this order, then the components it uses. Custom elements and
classic scripts, no build step, so it works from `file://`.

```html
<link rel="stylesheet" href="foundation/fonts.css">
<link rel="stylesheet" href="foundation/tokens.css">
<script src="foundation/pixel.js"></script>
<script src="foundation/backdrop.js"></script>
<script src="foundation/palette.js"></script>
<script src="components/omega-modal.js"></script>
<script src="components/omega-panel.js"></script>
<script src="components/omega-row.js"></script>   <!-- omega-list-row.js needs it first -->
<script src="components/omega-list-row.js"></script>
<script src="components/action-left.js"></script>
<script src="components/action-right.js"></script>
<script src="components/omega-bar.js"></script>
<script src="components/omega-pill.js"></script>
<script src="components/omega-gauge.js"></script>
<script src="components/omega-hints.js"></script>   <!-- after omega-gauge.js and omega-bar.js -->
<script src="components/omega-toast.js"></script>   <!-- after omega-panel.js, which it is drawn with -->
<script src="components/omega-badge.js"></script>
<script src="components/omega-saves.js"></script>  <!-- after omega-panel.js -->

<div class="omega-backdrop" data-style="plates" data-moving></div>  <!-- Grid without data-style -->

<omega-modal>
  <omega-panel label="BUTTON SETTING" active>
    <omega-row label="JUMP" value="A" lit carets></omega-row>
  </omega-panel>
  <omega-panel label="DEVICES">
    <omega-list-row label="8BitDo SN30 Pro" value="CONNECTED" device="gamepad"></omega-list-row>
  </omega-panel>
</omega-modal>
<omega-gauge label="VOLUME" value="55" max="100" cells="20" icon="volume"></omega-gauge>
<omega-toast label="SAVE STATE" text="State saved"></omega-toast>
<omega-badge icon="fastforward" text="×4" lit></omega-badge>
<omega-saves age="4 min ago" count="4" at="1"></omega-saves>
<omega-hints label="NETWORK" hint="X SCAN | Y FORGET"></omega-hints>
<omega-bar hint="Y RESET">
  <action-left slot="left" label="CONTROLS" hint="SELECT"></action-left>
  <action-right slot="right" label="SETTINGS" hint="START"></action-right>
</omega-bar>
<omega-bar>                                          <!-- the shelf's: its system, no hints -->
  <action-left slot="left" label="CONTROLS" hint="SELECT"></action-left>
  <omega-pill slot="middle" text="GBA" centred></omega-pill>
  <action-right slot="right" label="SETTINGS" hint="START"></action-right>
</omega-bar>

<script>OmegaPalette.apply('teal')</script>  <!-- Night unless told otherwise -->
```

## Over a game

`<omega-gauge>`, `<omega-toast>` and `<omega-badge>` are what omega-zero puts over a game, the
shelf or a menu, with nothing behind them but what is on the screen. A gauge (volume 20 cells,
brightness and blue light 9, rewind 20) or a toast stands centred at the top, 4 art pixels down,
or 2 under the top bar where the shelf shows it; only one at a time, a toast taking the gauge's
place. Each comes at once and goes in four even steps of fading out. A badge stays in the top
right corner while it lasts, level with a gauge's middle: fast forward, `×2` to `×6` as Settings
sets its speed, lit once latched, or the link, which outranks it.

## The shelf's pill

The shelf has no slot: a cart played rises and dives out of the bottom of the screen into the
handheld. The carts stand centred between the top bar and the bottom bar, and the shelf's system,
GBA, GB, GBC, SNES or PS1, is `<omega-pill>` in the bar's `middle`, between its two actions and in
the middle of the screen, `centred`. It is the bar's own button pill standing on its own, both ends
rounded, the bar's full height, 3 dark pixels either side of the word's ink, as `pill_face` in
`crates/omega-ui/src/kit/action.rs` draws it. It is always there on the shelf, even with only one
shelf, and sinks with the shelf's actions when the bar changes them. L1 and R1 change the system and
the pill rises into place with the new one over 220 ms; a letter jumped to with Up or Down shows in
it for about 2.2 s, then the system rises back. It is gone while the core picker is open.

## Pages around the menus

The last five components are for pages about omega-zero rather than its screens, from the site
upstream draws with them and the rest of the kit (Emellie Slot has none). Each holds the page's own element, so the
words, links, headings and alt text stay real HTML: `<omega-button>` a link or button in the
bar's pill, lit on hover and focus, with a ring for keyboard focus; `<omega-screen>` a screenshot
in a handheld's bezel, 2×2 art pixels kept 2×2 at its own size; `<omega-card>` a feature, its icon
an 11×11 sprite (`cart`, `chip`, `rewind`, `link`, `window`, `gamepad`, `wifi`, `bluetooth`,
`theme`) beside a slotted title; `<omega-code>` a `<pre>` in a dark box that scrolls sideways
instead of the page, with a label and a COPY key if asked; `<omega-tabs>` a row of links drawn as
panel titles, each tab 12 art pixels tall with the panel's slanted end, wrapping into rows on a
narrow page. They mark how they draw what they hold `!important`, so a page's own rules for
links, headings and `<pre>` stay off it.

`<omega-tabs>` is an ARIA tablist: each link becomes a tab and the part of the page it links to
its tabpanel, every part but the one shown hidden; Left and Right (round from the ends), Home and
End move between tabs and show the one moved to, and a click, Enter or Space shows the one
pressed. With `hash` the URL's hash follows the tab shown, none for the first, and the tab
follows the hash, so `index.html#install` opens on INSTALL, the back button goes back a tab, and a
link to something inside a part shows its tab and scrolls to it. Each change sends `omega-tab`.
With scripts off the links stay links and every part shows, one under the other.

## Screens and slides

`flow.html` puts every menu screen together from the kit (the shelf, Settings and everything
under it, CONTROLS, the power menu and the screens after it) and moves between them with the slides the device draws: 280 ms,
smootherstep, every frame on whole art pixels. It is the approved reference for the Rust screens.
Open it from disk like the storybook. What Rust draws itself (the carts and the top bar) is a
flat placeholder; the shelf's system is the kit's pill between the bar's actions, and L1 and R1
change it.

Walk it with the keyboard or the buttons under the screen:

| Button | Keys |
|---|---|
| D-pad | arrows |
| L1, R1 | `L`, `R` |
| A | `A` |
| B | `Z`, `B` or `Esc` |
| X | `X` or `S` |
| Y | `Y` |
| START | `Enter` |
| SELECT | `Space` or right `Shift` |
| POWER, held | `P` |

The sidebar jumps straight to a screen, sets Settings > Theme and the panel size, and has
**slow motion ×5** and **step one frame**, which holds a slide and moves it a frame at a time.
A key pressed during a slide finishes it, then does what it does.

Settings > Network never looks for networks on its own. AVAILABLE NETWORKS lists what is known
without a scan, the network joined (Home, CONNECTED, with no bars or padlock), and the note says
"Press X to look for networks.". X looks once: "Looking for networks…" for a moment, then every
network found with its bars and padlock, which stay until the next scan's answer or until
Network is left. The bar says `X SCAN` while Wi-Fi is on, and `X SCAN` `Y FORGET` on the network
joined. With Wi-Fi off the list is empty and the note says "Wi-Fi is off.".

NETWORK's three rows, WI-FI, NETWORK TIME and SSH, are switches, ON or OFF: Left, Right and A
turn each the other way. Y on one opens its own panel, a lone one like SSH's login, and B comes
back to the row. WI-FI's shows the network joined, its signal, the handheld's IP, host name and
MAC; NETWORK TIME's whether the clock is synced, with the date, time and zone Date & Time has;
SSH's the login. With nothing to show (Wi-Fi off, SSH off, Network Time off) a panel says what to
do first; NETWORK TIME's, with no Wi-Fi, keeps its rows and waits for one. On these rows the bar
says `X SCAN` `Y INFO`, or `Y INFO` alone with Wi-Fi off.

Settings > Bluetooth is a modal like Network's: BLUETOOTH, its one row switching the radio ON
or OFF with Left and Right, and DEVICES, while it is on the devices paired, then those a look
finds, each with its name in its own case, what it is to the handheld (CONNECTED, CONNECTING,
PAIRING, PAIRED, or nothing for one only found) and its kind's icon
(`<omega-list-row device>`). A device found that gives no name is never listed. Opening the
screen looks for nothing: a note under the paired devices says "Press X to look for devices.",
and X looks for a few seconds, the note saying "Looking for devices…" while what it finds joins
the list, then stops by itself; a look that finds nothing says "No devices found.". What a look
found stays until the screen is left. A on a found device pairs with it and connects (PAIRING,
then CONNECTED), ending a look under way, on a paired one connects and on a connected one
disconnects; Y forgets a paired device. The bar says `X SCAN` while the radio is on, and
`X SCAN` `Y FORGET` while the cursor is on a paired device. With the radio off the note says
"Bluetooth is off.". Settings' BLUETOOTH row shows the connected device's name in capitals,
else ON or OFF.

Settings > About is a modal like Settings, with nothing to pick: OMEGA-ZERO, as tall as its
rows, says the version, the build (the commit, `-DIRTY` after it for a build with changes not
committed), the author, the handheld as BaseOS names it and its battery, each a name and a value
as SSH's lines are; CREDITS under it says what omega-zero is made with and on, a line each in
a row name's colour, three pixels in as a note is. Both titles are lit and no row is.

Holding POWER opens POWER on any screen, a lone panel like THEME's with RESTART and POWER OFF,
deeper from the screen it was held on, which its B goes back to. A on either row goes deeper to
the screen shown until the handheld goes down: RESTART saying "Restarting…" or POWER OFF saying
"Turning off…", a note on its own as SSH's. Nothing can be pressed there, so the whole bar sinks
away as it slides in, `even(p·38)` down, as it does on the device when a cart goes in.

The URL's hash says what is shown, and the page keeps it up to date as you walk, so a link or a
headless Chrome screenshot lands on the same frame:

| Key | Values |
|---|---|
| `screen` | `shelf`, `settings`, `controls`, `network`, `wifi` (Y on WI-FI), `time` (Y on NETWORK TIME), `ssh` (Y on SSH), `keyboard`, `bluetooth`, `labels`, `datetime`, `background` (THEME), `about`, `power` (POWER held), `restarting`, `poweroff` |
| `from` | slide into `screen` from this one, deeper or back as the tree has it; `dir=in` or `dir=out` forces the way. For `power`, the screen POWER was held on (the shelf when there is none) |
| `t` | with `from`: hold the slide at this time, 0 to 1 |
| `size` | `640x480` (default), `720x480`, `720x720` |
| `colour` | `night` (default), `slate`, `navy`, `teal`, `green`, `plum`, `crimson`, `amber` |
| `style`, `moving` | the backdrop, as Settings > Theme sets it: `style=plates`, `moving=0` |
| `font` | `thin`: the bottom bar's labels in the thin font, as Settings > Theme > Font sets it |
| `wifi` | `off`: WI-FI and SSH show what they say offline, NETWORK TIME waits for Wi-Fi, and Network lists no networks |
| `ntp` | `off`: Network Time off, and NETWORK TIME's panel shows what it says offline |
| `scan` | Network's scan: `none` (default, as it opens), `running` (looking, standing still) or `done` (the networks found) |
| `bt` | Bluetooth: `on` (default) as the screen opens, before any look, with the two paired devices; `off`; `empty` (nothing paired, before any look); `looking` (a look under way, held, the keyboard found so far); `found` (after it, the band found too); or `pairing` (after it, the keyboard pairing, the cursor on it) |
| `system` | the shelf's, in its pill: `gba` (default), `gb`, `gbc` |
| `letter` | a letter jumped to on the shelf (`A` `C` `F` `K` `M` `P` `S` `Z`), held in the pill for a still |
| `row` | the cursor's row on `screen`, from 0 |
| `slow` | `1`: slides at a fifth of their speed |
| `shot` | `1`: the screen alone at the window's top left, for screenshots |

```
flow.html#screen=network&from=settings&t=0.5&colour=amber&shot=1
```

is Settings halfway through sliding out to the right for Network, in Amber, ready for a
screenshot the size of the panel.

## The save states

The save-state switcher, MENU ×2 in a game, is drawn from the kit over the screenshot of the
save it shows, which is drawn as the game is: the shelf's top bar, `<omega-saves>` at the top in
the middle 8 art pixels under it, and the bar with B RETURN and A LOAD and nothing between them:
a save state is saved and loaded, nothing more. Left and Right move between the saves; the panel
says how old the one shown is and lights its dot.

`<omega-hints>` is for any keys that would sit on a picture: it is `<omega-gauge>`'s chrome, the
frame, the title tab and its slant, and the well, with the keys in the well drawn as
`<omega-bar>` draws its hints, a dark cap and then the word, on the title's rows. The two share
their code: `OmegaGauge` in omega-gauge.js and `OmegaBar` in omega-bar.js, as `gauge_chrome`
and `draw_hints` do in Rust (crates/omega-ui/src/kit/hints.rs).

## The link screen

SELECT + MENU in a game opens the link screen over it, drawn in the kit like the menus: the
backdrop, a lone LINK panel at the top with ROLE (HOST or JOIN, lit with its carets while the
player picks) and MODE (CABLE or WIRELESS), and under them, as a note, what is happening
("Looking for the other player", "Nobody arrived", "Linked"); the art between it and the bar; and
the bar, B RETURN and A LINK while picking with SELECT MODE (where the game can switch) and
◀▶ SWAP between them, B CANCEL while linking, A OK when it failed, B RETURN and A END LINK over a
link under way. `<omega-link>` draws the art one piece at a time: the port's strip along the
bottom under the bar, the plug on its cable running up behind the panel, the wireless adapter and
its radio arcs, and the clicks of a plug going home. The device moves them a whole art pixel at a
time: the plug drops into the port and lifts out of it, a failed link shakes it a pixel either
way, and the arcs pulse in the four steps of a fade. The plug and the adapter keep their own
colours, the plug the purple of the real GBA link cable; everything else follows the palette.

## The DISC screen

On a PlayStation game, SELECT + MENU opens the DISC screen instead of the link screen, a card of
its own in the storybook: the backdrop, a lone DISC panel where the LINK panel goes with DISC (the
disc picked, of how many, lit with its carets), IN DRIVE (the disc in the drive) and the game's
title as a note; the game's discs side by side between it and the bar, the picked one 6 art pixels
higher and the others faded back a step; and the bar, B RETURN and A SWAP with ◀▶ PICK between
them. Left and Right pick, A swaps the picked disc in and a DISC toast says which ("Disc 2
inserted"), B, SELECT or SELECT + MENU close it. A game on one disc opens nothing and says "Only
one disc". Each disc is the shelf's face for it, drawn flat until the shelf's disc stack takes
its place.

## The rules

These are what make it look like one thing. Keep to them when changing or adding to the kit.

- **One art pixel is `--u`, 2px.** omega-zero draws pixel art 2×2, so a 640×480 panel is 320×240
  art pixels. Every size is a whole number of `--u`; nothing lands on a half pixel.
- **Every shape is a sprite**, written in its component as rows of colour keys, one character to an
  art pixel (see `CORNER` in `omega-modal.js`, `ARROW` in `action-left.js`), and painted by
  `OmegaPixel.sprite()` with `image-rendering: pixelated`. No `border-radius`, no smooth curves, no
  filters. A shape that fits any size is a 9-slice: draw its top-left quarter, `mirror()` the rest.
- **Text sits on whole pixels.** Pixel Code Regular at 18px is one face pixel to an art pixel; set
  it on a 20px line so its capitals land on rows 2 to 8. Pixel Code Bold is fattened off the grid:
  draw bold instead, the word again one pixel to the right. A word drawn bigger or stretched goes
  into a sprite through `OmegaPixel.letters()`, never through a CSS transform (a transformed line's
  baseline falls between pixels). Every component sets `text-rendering: geometricPrecision` on its
  host: Chrome's default hinting smears Pixel Code's edges by part of a pixel even where it sits on
  whole pixels, at 1× and on a 2× display alike.
- **Colours come from the backdrop colour.** Use only the `--oz-*` properties `palette.js` sets.
  Night (the default) gives plain greys, Amber gives Mega Man Zero's own colours, the rest turn
  that ramp to their hue. A component that paints sprites extends `OmegaPixel.Element` and draws in
  `paint()`, which runs again whenever the palette changes.
- **Pages put components on whole pixels too.** Centring something in a box with an odd number of
  pixels to spare puts it half a pixel off, and its text blurs (borders and sprites hide it, text
  does not). Round the offset, as `main` in `storybook.html` does.
- **Each area is its own div**, named with `part` so a page can reach it with `::part()`.
- **Space is even.** Text gets the same room above as below inside its box; a title starts from
  its tab's left edge.
- **One exception: the GAME BOY ADVANCE band round a game.** On the real console that strip is a
  label printed in high resolution, and the band imitates it, so it keeps the look of a
  high-resolution print and is never redrawn as pixel art. Everything else on screen is pixel art
  on this grid.

## Adding a component

1. Write `components/<name>.js` as a custom element extending `OmegaPixel.Element`, its shapes as
   sprite rows at the top of the file and its sizes in art pixels.
2. Draw its sprites in `paint()`; attributes that change its text call the part of `paint()` that
   draws it.
3. Give it a card in `storybook.html`: what it does, a live example (empty and filled when it takes
   text), its attributes and parts, and the markup to use it.
4. Check it zoomed in at 1× and 2×: every edge on a whole art pixel, nothing blurred.
