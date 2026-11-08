/*
  <omega-gauge>: a level on one line, like the life gauge in Mega Man Zero: a title tab naming it
  on the left, a well of cells on the right, as many lit as the level reaches, and an icon at its
  end. It is a panel's header with the strip beside the tab made the gauge, so it sits with the
  menus' panels, and on its own over a game for the volume, the brightness, the sunlight on a solar sensor cartridge or the
  rewind left.

    <omega-gauge label="VOLUME" value="55" max="100" cells="20" icon="volume"></omega-gauge>

  label   the name in the tab, in capitals whatever case it is given in.
  value   the level, from 0 to max. Default 0.
  max     the level the gauge is full at. Default 100.
  cells   how many cells the well has: one to a step is clearest (9 for 0 to 9). Default 20.
  icon    a 7x7 sprite after the cells, the height of the capitals: volume, muted, headphones,
          brightness, sun, rewind, fastforward. None without it.

  Lit cells are round(value / max * cells) from the left, so 0 lights none and max lights them
  all. Lit, a cell is --oz-label; unlit, --oz-plate, the colour of whatever is off on a row.

  It is 15 art pixels tall, as tall as the bottom bar: a panel's frame line round it, and inside
  it 13 rows. The tab fills them: its frame line, two rows, the capitals, two rows, its frame line,
  so the title sits in the middle (a panel's tab keeps one of the lower rows for descenders; the
  gauge's names have none). The tab ends in the panel's 45 degree slant. Beside it, on the strip's
  bright colour, the well is inset a pixel above, below and at the end; its shadow line round it,
  then a pixel clear, then the cells, which stand on the capitals' rows, and three pixels on, the
  icon. It is as wide as all that; place it from outside. Areas, each a div that ::part()
  reaches: gauge, tab, label, slant, strip, well, cells, icon.

  Its chrome, everything round what the well holds, is OmegaGauge, which <omega-hints>
  (omega-hints.js) is drawn with too: OmegaGauge.STYLE and OmegaGauge.markup(part, well) for its
  shadow root, OmegaGauge.paint(root, label) for its sprites and its title.

  Needs everything in foundation/ on the page (see README.md); its colours follow palette.js.
*/
(() => {
  // The frame round it, as round a panel: its stripe, the corner pixel left out, round the frame
  // colour.
  const FRAME = [
    '.s',
    'sA',
  ];
  // The well's corner: its shadow line, the same way, round the well.
  const WELL = [
    '.e',
    'ew',
  ];

  // How tall the inside is in art pixels: the tab's frame lines, two rows either side of the
  // capitals, and the capitals.
  const H = 13;

  // Each icon as rows of '#', in the cream the title is lit in.
  const ICONS = {
    volume: [
      '...#...',
      '..##.#.',
      '####..#',
      '####..#',
      '####..#',
      '..##.#.',
      '...#...',
    ],
    muted: [
      '...#...',
      '..##...',
      '###.#.#',
      '###..#.',
      '###.#.#',
      '..##...',
      '...#...',
    ],
    headphones: [
      '.#####.',
      '#.....#',
      '#.....#',
      '#.....#',
      '##...##',
      '##...##',
      '##...##',
    ],
    brightness: [
      '#..#..#',
      '.#...#.',
      '..###..',
      '#.###.#',
      '..###..',
      '.#...#.',
      '#..#..#',
    ],
    sun: [
      '#..#..#',
      '.#...#.',
      '..###..',
      '.#####.',
      '.#####.',
      '.......',
      '#######',
    ],
    rewind: [
      '...#..#',
      '..##.##',
      '.######',
      '#######',
      '.######',
      '..##.##',
      '...#..#',
    ],
    fastforward: [
      '#..#...',
      '##.##..',
      '######.',
      '#######',
      '######.',
      '##.##..',
      '#..#...',
    ],
  };

  // The tab's slanted end, as a panel's: one pixel further right each row, its top and bottom
  // rows in the frame colour as the tab's outline, then a frame pixel and a stripe pixel, then
  // nothing, where the strip shows.
  function slant() {
    return Array.from({ length: H }, (_, y) =>
      (y === 0 || y === H - 1 ? 'A' : 't').repeat(y) + 'As' + '.'.repeat(H + 1 - y - 2));
  }

  // `n` cells three pixels wide with a pixel between, `lit` of them lit, as tall as the capitals.
  function cells(n, lit) {
    const row = Array.from({ length: n }, (_, i) => (i < lit ? 'LLL' : 'ppp')).join('.');
    return Array.from({ length: OmegaPixel.CAP }, () => row);
  }

  // The chrome's style: the frame, the tab and its title, the slant, the strip, and the well
  // inset in it, for whatever the well holds to lay itself out in.
  const STYLE = `
      /* text-rendering: Pixel Code drawn without the browser's hinting, as in a panel */
      :host { display: inline-flex; text-rendering: geometricPrecision; }
      :host([hidden]) { display: none; }

      .gauge {
        display: flex;
        border: var(--u) solid transparent;
        border-image-slice: 1 fill;
        border-image-repeat: stretch;
        image-rendering: pixelated;
      }
      /* the title 4 pixels in from either end of the tab, on a line of 7 rows, its capitals' */
      .tab {
        flex: none;
        box-sizing: border-box;
        height: calc(${H} * var(--u));
        padding: calc(2 * var(--u)) calc(4 * var(--u));
        background: var(--oz-tab);
        border: var(--u) solid var(--oz-frame);
        border-right: 0;
      }
      /* Pixel Code one face pixel to an art pixel on a line of 10, raised two rows inside a box of
         7, so the line's capitals, its rows 2 to 8, are all that shows */
      .label {
        height: calc(${OmegaPixel.CAP} * var(--u));
        overflow: hidden;
        white-space: nowrap;
        text-transform: uppercase;
        color: var(--oz-cream);
        font: 400 calc(9 * var(--u)) / calc(10 * var(--u)) var(--oz-font);
      }
      .label span { display: block; margin-top: calc(-${OmegaPixel.ACCENT} * var(--u)); }
      .slant {
        flex: none;
        width: calc(${H + 1} * var(--u));
        height: calc(${H} * var(--u));
        background: var(--oz-frame-hi) no-repeat 0 0 / 100% 100%;
        image-rendering: pixelated;
      }
      .strip {
        display: flex;
        height: calc(${H} * var(--u));
        box-sizing: border-box;
        padding: var(--u) var(--u) var(--u) 0;
        background: var(--oz-frame-hi);
      }
      .well {
        display: flex;
        padding: 0 calc(2 * var(--u));
        border: var(--u) solid transparent;
        border-image-slice: 1 fill;
        border-image-repeat: stretch;
        image-rendering: pixelated;
      }`;

  // The chrome's markup, the outermost area named `part`, round `well`, what the well holds.
  function markup(part, well) {
    return `
    <div class="gauge" part="${part}">
      <div class="tab" part="tab"><div class="label" part="label"><span></span></div></div>
      <div class="slant" part="slant"></div>
      <div class="strip" part="strip">
        <div class="well" part="well">${well}</div>
      </div>
    </div>`;
  }

  // The chrome's sprites in `root`'s host's colours, and `label` in its tab.
  function paint(root, label) {
    const { sprite, mirror } = OmegaPixel;
    const part = name => root.querySelector(`.${name}`);
    part('gauge').style.borderImageSource = sprite(mirror(FRAME), root.host);
    part('well').style.borderImageSource = sprite(mirror(WELL), root.host);
    part('slant').style.backgroundImage = sprite(slant(), root.host);
    part('label').firstElementChild.textContent = label;
  }

  window.OmegaGauge = { STYLE, markup, paint };

  const template = document.createElement('template');
  template.innerHTML = `
    <style>${STYLE}
      .well { align-items: center; gap: calc(3 * var(--u)); }
      .cells, .icon {
        flex: none;
        height: calc(${OmegaPixel.CAP} * var(--u));
        background: no-repeat 0 0 / 100% 100%;
        image-rendering: pixelated;
      }
      .icon { width: calc(7 * var(--u)); }
      .icon[hidden] { display: none; }
    </style>${markup('gauge', `
          <div class="cells" part="cells"></div>
          <div class="icon" part="icon"></div>
        `)}`;

  customElements.define('omega-gauge', class extends OmegaPixel.Element {
    static observedAttributes = ['label', 'value', 'max', 'cells', 'icon'];

    constructor() {
      super();
      this.attachShadow({ mode: 'open' }).append(template.content.cloneNode(true));
    }

    paint() {
      const { sprite } = OmegaPixel;
      const part = name => this.shadowRoot.querySelector(`.${name}`);
      paint(this.shadowRoot, this.getAttribute('label') ?? '');

      const n = Math.max(1, Math.round(+(this.getAttribute('cells') ?? 20)) || 20);
      const max = +(this.getAttribute('max') ?? 100) || 100;
      const value = Math.min(max, Math.max(0, +(this.getAttribute('value') ?? 0) || 0));
      const lit = Math.round((value / max) * n);
      const c = part('cells');
      c.style.width = `calc(${n * 4 - 1} * var(--u))`;
      c.style.backgroundImage = sprite(cells(n, lit), this);

      const rows = ICONS[this.getAttribute('icon')];
      const i = part('icon');
      i.hidden = !rows;
      if (rows) i.style.backgroundImage = sprite(rows.map(r => r.replace(/#/g, 'c')), this);
    }

    attributeChangedCallback() {
      if (this.isConnected) this.paint();
    }
  });
})();
