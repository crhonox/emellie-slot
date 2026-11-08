#!/bin/bash
# Screenshots kit.html's components into ref/, one PNG each at one pixel to an art pixel, for
# kit.rs to hold the Rust faces to: each component, lines of text as flow.html sets them, and
# flow.html's lone panels. Run it again after changing a component in assets/ui or flow/; the
# faces must then change to match, pixel for pixel.
#
#   crates/omega-ui/tests/suite/kit/make-refs.sh
#
# macOS smooths DOM text (CoreText font smoothing), which rings each glyph with a faint halo the
# canvas words never get; kit.html turns it off, so the hints, titles, rows and notes are drawn
# on whole pixels as the faces are.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
chrome="${CHROME:-/Applications/Google Chrome.app/Contents/MacOS/Google Chrome}"
mkdir -p "$here/ref"

# name, width and height in art pixels, then the query kit.html draws it from
shoot() {
  "$chrome" --headless=new --disable-gpu --hide-scrollbars --force-device-scale-factor=1 \
    --default-background-color=00000000 --virtual-time-budget=3000 \
    --window-size="$2,$3" --screenshot="$here/ref/$1.png" "file://$here/kit.html?$4" 2>/dev/null
}

shoot modal 200 120 'c=modal&w=200&h=120'
shoot modal-panels 200 121 'c=modal&w=200&h=121&weights=1,1'
shoot modal-panels-4-3 200 120 'c=modal&w=200&h=120&weights=4,3'
shoot modal-panels-1-1 200 120 'c=modal&w=200&h=120&weights=1,1'
shoot panel-button-setting-active 160 60 'c=panel&w=160&h=60&label=BUTTON%20SETTING&active'
shoot panel-button-setting 160 60 'c=panel&w=160&h=60&label=BUTTON%20SETTING'
shoot panel-configs-active 160 60 'c=panel&w=160&h=60&label=CONFIGS&active'
shoot panel-configs 160 60 'c=panel&w=160&h=60&label=CONFIGS'
shoot action-left-controls 115 19 'c=action-left&label=CONTROLS&hint=SELECT'
shoot action-right-settings 109 19 'c=action-right&label=SETTINGS&hint=START'
shoot action-left-return 90 19 'c=action-left&label=RETURN&hint=B'
shoot action-right-confirm 97 19 'c=action-right&label=CONFIRM&hint=A'
shoot action-left-empty 75 19 'c=action-left'
shoot action-right-empty 75 19 'c=action-right'
# given the room of the action beside them: SETTINGS's pill as wide as SELECT's, RETURN's word
# box as wide as CONFIRM's
shoot action-right-settings-paired 115 19 'c=action-right&label=SETTINGS&hint=START&room=61%2041'
shoot action-left-return-paired 97 19 'c=action-left&label=RETURN&hint=B&room=54%2030'
# in the thin font, paired: the shelf's two 107 wide, a menu's 90
shoot action-left-controls-thin 107 19 'c=action-left&label=CONTROLS&hint=SELECT&font=thin&room=53%2041'
shoot action-right-settings-thin 107 19 'c=action-right&label=SETTINGS&hint=START&font=thin&room=53%2041'
shoot action-left-return-thin 90 19 'c=action-left&label=RETURN&hint=B&font=thin&room=47%2030'
shoot action-right-confirm-thin 90 19 'c=action-right&label=CONFIRM&hint=A&font=thin&room=47%2030'

# rows: every colour state, carets kept unlit, the opens chevron, a label cut where it runs out
shoot row-plain 150 13 'c=row&w=150&label=FAST%20FORWARD&value=6X&carets'
shoot row-lit-carets 150 13 'c=row&w=150&label=FAST%20FORWARD&value=6X&carets&lit'
shoot row-lit 150 13 'c=row&w=150&label=RUMBLE&value=ON&lit'
shoot row-opens 150 13 'c=row&w=150&label=LABELS&opens'
shoot row-opens-lit 150 13 'c=row&w=150&label=NETWORK&value=HOME&opens&lit'
shoot row-carets-opens-lit 150 13 'c=row&w=150&label=SHADER&value=SHARP&carets&opens&lit'
shoot row-dim 150 13 'c=row&w=150&label=WI-FI&value=OFF&carets&dim'
shoot row-dim-lit 150 13 'c=row&w=150&label=WI-FI&value=OFF&carets&dim&lit'
shoot row-dim-opens 150 13 'c=row&w=150&label=SSH&opens&dim'
shoot row-value 150 13 'c=row&w=150&label=IP&value=192.168.1.20'
shoot row-case 150 13 'c=row&w=150&label=Host&value=rg35xxsp.local'
shoot row-name 150 13 'c=row&w=150&label=FETCH%20LABELS'
shoot row-cut 60 13 'c=row&w=60&label=FAST%20FORWARD%20SOUND&value=OFF&lit'
shoot row-cut-carets 50 13 'c=row&w=50&label=FAST%20FORWARD&value=6X&carets&lit'
shoot row-squeezed 30 13 'c=row&w=30&label=RUMBLE&value=OFF'

# network rows: the bars 0 to 3, the padlock, their room kept without, a name in its own case
shoot list-home-lit 150 13 'c=list-row&w=150&label=Home&value=CONNECTED&signal=3&lock&lit'
shoot list-cafe 150 13 'c=list-row&w=150&label=Caf%C3%A9%20Wi-Fi&signal=2'
shoot list-cafe-lit 150 13 'c=list-row&w=150&label=Caf%C3%A9%20Wi-Fi&signal=2&lit'
shoot list-upstairs 150 13 'c=list-row&w=150&label=Upstairs%205G&signal=1&lock'
shoot list-attic 150 13 'c=list-row&w=150&label=Attic&signal=0&lock'
shoot list-saved 150 13 'c=list-row&w=150&label=Office&value=SAVED&signal=1&lock'
shoot list-dim 150 13 'c=list-row&w=150&label=Home&signal=3&lock&dim'
shoot list-dim-lit 150 13 'c=list-row&w=150&label=Home&value=CONNECTED&signal=3&lock&dim&lit'
shoot list-bare 150 13 'c=list-row&w=150&label=Hidden'
shoot list-cut 100 13 'c=list-row&w=100&label=A%20very%20long%20network%20name&signal=2&lock&lit'

# device rows: each kind's icon, unlit and lit, in the middle of its room where the bars end and
# no room kept for a padlock; a device with no name shows its MAC; dim; a name cut short
shoot list-gamepad 150 13 'c=list-row&w=150&label=8BitDo%20SN30%20Pro&value=CONNECTED&device=gamepad'
shoot list-gamepad-lit 150 13 'c=list-row&w=150&label=8BitDo%20SN30%20Pro&value=CONNECTED&device=gamepad&lit'
shoot list-audio 150 13 'c=list-row&w=150&label=WH-1000XM4&value=PAIRED&device=audio'
shoot list-audio-lit 150 13 'c=list-row&w=150&label=WH-1000XM4&value=PAIRED&device=audio&lit'
shoot list-keyboard 150 13 'c=list-row&w=150&label=Keychron%20K3&device=keyboard'
shoot list-keyboard-lit 150 13 'c=list-row&w=150&label=Keychron%20K3&device=keyboard&lit'
shoot list-other 150 13 'c=list-row&w=150&label=4C%3A4C%3A6B%3A1A%3A80%3AD3&device=other'
shoot list-other-lit 150 13 'c=list-row&w=150&label=4C%3A4C%3A6B%3A1A%3A80%3AD3&device=other&lit'
shoot list-device-dim 150 13 'c=list-row&w=150&label=Keychron%20K3&value=PAIRING&device=keyboard&dim'
shoot list-device-cut 100 13 'c=list-row&w=100&label=8BitDo%20SN30%20Pro&value=CONNECTING&device=gamepad&lit'

# the bar at 640 and 720 (320 and 360 art pixels): the shelf's, a menu's, with one hint and two;
# its two actions one width
for w in 320 360; do
  shoot bar-shelf-$w $w 19 "c=bar&w=$w&left=CONTROLS&left-hint=SELECT&right=SETTINGS&right-hint=START"
  shoot bar-menu-$w $w 19 "c=bar&w=$w&left=RETURN&left-hint=B&right=CONFIRM&right-hint=A"
  shoot bar-forget-$w $w 19 "c=bar&w=$w&left=RETURN&left-hint=B&right=CONFIRM&right-hint=A&hint=Y%20FORGET"
  shoot bar-two-$w $w 19 "c=bar&w=$w&left=RETURN&left-hint=B&right=CONFIRM&right-hint=A&hint=Y%20FORGET%20%7C%20X%20RENAME"
done
shoot bar-fetch-320 320 19 'c=bar&w=320&left=RETURN&left-hint=B&right=CONFIRM&right-hint=A&hint=START%20FETCH'
# the first-boot clock's: CONFIRM, and the empty action as wide beside it
shoot bar-confirm-320 320 19 'c=bar&w=320&right=CONFIRM&right-hint=A'
# in the thin font, which the bar gives its actions
shoot bar-shelf-thin-320 320 19 'c=bar&w=320&font=thin&left=CONTROLS&left-hint=SELECT&right=SETTINGS&right-hint=START'
shoot bar-menu-thin-320 320 19 'c=bar&w=320&font=thin&left=RETURN&left-hint=B&right=CONFIRM&right-hint=A&hint=Y%20FORGET'

# the save-state switcher's: its bar, B RETURN and A LOAD with nothing between them; its keys on a
# line of their own above it, SAVE with Y DELETE and X UNDO SAVE or X UNDO LOAD, or Y DELETE alone,
# as wide as it needs or held as wide as both; and its panel, as wide as the title, the dots or the
# date, or held wider
for w in 320 360; do
  shoot bar-switcher-$w $w 19 "c=bar&w=$w&left=RETURN&left-hint=B&right=LOAD&right-hint=A"
done
shoot hints-switcher 176 15 'c=hints&label=SAVE&hint=Y%20DELETE%20%7C%20X%20UNDO%20SAVE'
shoot hints-switcher-load 176 15 'c=hints&label=SAVE&hint=Y%20DELETE%20%7C%20X%20UNDO%20LOAD'
shoot hints-switcher-delete 103 15 'c=hints&label=SAVE&hint=Y%20DELETE'
shoot hints-switcher-held 176 15 'c=hints&label=SAVE&hint=Y%20DELETE&w=176'
# keys on a line: none, a label in lower case and three keys, the last with no word
shoot hints-empty 56 15 'c=hints&label=KEYS'
shoot hints-three 235 15 'c=hints&label=keys&hint=START%20FETCH%20%7C%20Y%20SIGN%20OUT%20%7C%20SELECT'
shoot saves-just-now 90 41 'c=saves&age=Just%20now'
shoot saves-four 90 41 'c=saves&age=4%20min%20ago&count=4&at=1'
shoot saves-ten 90 41 'c=saves&age=3%20hr%20ago&count=10&at=9'
shoot saves-date 108 41 'c=saves&age=2026-08-08%2020%3A00&count=3&at=2'
shoot saves-held 108 41 'c=saves&w=108&age=Just%20now&count=2'

# lines of text as flow.html sets them, the Controls panel's arrows as flow/art.js draws them
shoot text-sftp 144 11 'c=text&tone=dim&text=SFTP%20uses%20the%20same%20login'
shoot text-offline 162 11 'c=text&tone=dim&text=Join%20a%20Wi-Fi%20network%20first.'
shoot text-bluetooth-off 102 11 'c=text&tone=dim&text=Bluetooth%20is%20off.'
shoot text-looking 120 11 'c=text&tone=dim&text=Looking%20for%20devices%E2%80%A6'
shoot text-labels 264 21 'c=text&tone=dim&text=Sign%20in%20with%20your%20free%20ScreenScraper%20account%0Ato%20fetch%20the%20real%20labels%20of%20your%20carts.'
shoot text-heading 42 11 'c=text&tone=dim&text=IN%20GAME'
shoot text-select-up-down 54 11 'c=text&tone=frame-hi&text=SELECT%2B%E2%96%B2%E2%96%BC'
shoot text-left-right 12 11 'c=text&tone=frame-hi&text=%E2%97%80%E2%96%B6'
shoot text-menu-twice 42 11 'c=text&tone=frame-hi&text=MENU%20%C3%972'
shoot text-does 90 11 'c=text&tone=cream&text=LOAD%2FSAVE%20STATE'

# flow.html's lone panels, rows and lines placed in their wells as it places them
shoot flow-ssh 200 104 'c=flow&screen=ssh'
shoot flow-ssh-offline 200 42 'c=flow&screen=ssh-offline'
shoot flow-ssh-switched-off 200 42 'c=flow&screen=ssh-switched-off'
shoot flow-labels 280 75 'c=flow&screen=labels'
shoot flow-controls 304 157 'c=flow&screen=controls'

# the gauges over a game, each with its icon, and one without; 20 cells for 0 to 100, 9 for 0 to 9
shoot gauge-volume 157 15 'c=gauge&label=VOLUME&value=55&max=100&cells=20&icon=volume'
shoot gauge-muted 157 15 'c=gauge&label=VOLUME&value=0&max=100&cells=20&icon=muted'
shoot gauge-headphones 157 15 'c=gauge&label=VOLUME&value=35&max=100&cells=20&icon=headphones'
shoot gauge-brightness 137 15 'c=gauge&label=BRIGHTNESS&value=6&max=9&cells=9&icon=brightness'
shoot gauge-sun 125 15 'c=gauge&label=SUNLIGHT&value=5&max=10&cells=10&icon=sun'
shoot gauge-rewind 157 15 'c=gauge&label=REWIND&value=70&max=100&cells=20&icon=rewind'
shoot gauge-speed 103 15 'c=gauge&label=SPEED&value=4&max=8&cells=8&icon=fastforward'
shoot gauge-empty 97 15 'c=gauge&label=LEVEL&value=0&max=9&cells=9'
shoot gauge-full 97 15 'c=gauge&label=LEVEL&value=9&max=9&cells=9'

# toasts: the tab wider than the line, the line wider than the tab, a descender, a label in
# lower case
shoot toast-saved 84 34 'c=toast&label=SAVE%20STATE&text=State%20saved'
shoot toast-ended 96 34 'c=toast&label=LINK&text=Link%20was%20ended'
shoot toast-gpsp 138 34 'c=toast&label=LINK&text=Please%20switch%20to%20gpSP'
shoot toast-colour 96 34 'c=toast&label=colour&text=Correction%20off'

# badges: fast forward held and latched at each speed's width, the link's and a lost one, lit and
# not, and a sprite with no word
shoot badge-ff-2 25 9 'c=badge&icon=fastforward&text=%C3%972'
shoot badge-ff-6-lit 25 9 'c=badge&icon=fastforward&text=%C3%976&lit'
shoot badge-link 11 9 'c=badge&icon=link'
shoot badge-link-lit 11 9 'c=badge&icon=link&lit'
shoot badge-unlinked 11 9 'c=badge&icon=unlinked'
shoot badge-unlinked-lit 11 9 'c=badge&icon=unlinked&lit'
shoot badge-word 37 9 'c=badge&icon=fastforward&text=HOST'
# the link screen's art, each piece of omega-link: the plugs, a stretch of the cable, the adapter,
# the port's strip at 640 and 720, the radio arcs (and one turned to the left) and the clicks
shoot link-plug-host 30 54 'c=link&piece=plug&role=host'
shoot link-plug-join 36 54 'c=link&piece=plug&role=join'
shoot link-cable 6 4 'c=link&piece=cable&length=4'
shoot link-adapter 80 52 'c=link&piece=adapter'
shoot link-port-320 320 36 'c=link&piece=port&width=320&height=36'
shoot link-port-360 360 36 'c=link&piece=port&width=360&height=36'
shoot link-arc-0 5 14 'c=link&piece=arc&ring=0'
shoot link-arc-1 7 22 'c=link&piece=arc&ring=1'
shoot link-arc-2 9 30 'c=link&piece=arc&ring=2'
shoot link-arc-2-left 9 30 'c=link&piece=arc&ring=2&side=left'
shoot link-clicks 52 10 'c=link&piece=clicks'
