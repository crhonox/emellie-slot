#!/bin/sh
# pico8.sh, off-device: PICO-8, omega-zero, amixer and the dynamic loader are stand-ins that
# note how they were run.
set -eu

HERE="$(cd "$(dirname "$0")" && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT HUP INT TERM

fail() {
	echo "$*" >&2
	exit 1
}

CARD="$TMP/card"
CART="$CARD/Games/PICO8/celeste.p8.png"
CONFIG="$CARD/Saves/PICO8/.lexaloffle/pico-8/config.txt"

setup() {
	rm -rf "$CARD" "$TMP"/*.log
	mkdir -p "$CARD/System/pico8/bin" "$CARD/Games/PICO8" "$CARD/BIOS" "$TMP/fb0"
	cp "$HERE/System/pico8.sh" "$HERE/System/pad.sh" "$CARD/System/"
	cp "$HERE/System/pico8/bin/wget" "$CARD/System/pico8/bin/"
	: > "$CARD/System/pico8/libsdl_sensor_shim.so"
	: > "$CART"
	: > "$CARD/Games/PICO8/Splore.splore"
	: > "$CARD/BIOS/pico8.dat"
	printf 'BASEOS_TARGET=rg35xxsp\n' > "$TMP/release"
	printf 'U:720x720p-60\n' > "$TMP/fb0/modes"
	# PICO-8: where and how it ran, then however long it is told to
	cat > "$CARD/BIOS/pico8_64" <<-'SH'
		#!/bin/sh
		{
			echo "args=$*"
			echo "cwd=$(pwd)"
			echo "home=$HOME"
			echo "video=$SDL_VIDEODRIVER"
			echo "lib=$LD_LIBRARY_PATH"
			echo "preload=$LD_PRELOAD"
			echo "map=$SDL_GAMECONTROLLERCONFIG"
			echo "wget=$(command -v wget)"
		} > "$TMP/pico8.log"
		sleep "$(cat "$TMP/plays" 2>/dev/null || echo 0)"
		exit 3
	SH
	chmod +x "$CARD/BIOS/pico8_64"
	# omega-zero's volume keys: what they were given; MENU held when $TMP/hold is there
	cat > "$CARD/System/emellie-slot" <<-'SH'
		echo "$*" >> "$TMP/volume.log"
		[ -f "$TMP/hold" ] && { kill "$4"; exit 0; }
		exec sleep 30
	SH
	cat > "$TMP/amixer" <<-'SH'
		#!/bin/sh
		[ "$1" = sget ] && { echo "  Mono: Playback 25 [80%]"; exit 0; }
		echo "$*" >> "$TMP/amixer.log"
	SH
	chmod +x "$TMP/amixer"
}

pico8() {
	TMP="$TMP" OMEGA_ROOT="$CARD" AGS_LOADER=sh AGS_BASEOS_RELEASE="$TMP/release" \
		AGS_FB="$TMP/fb0" AGS_PICO8_NET_WAIT_S=0 AGS_AMIXER="$TMP/amixer" \
		sh "$CARD/System/pico8.sh" "$@"
}

# --- a cart: PICO-8 runs it at the panel's size, with the card's settings and the pad
setup
status=0
pico8 "$CART" > /dev/null || status=$?
[ "$status" = 3 ] || fail "pico8.sh did not leave with PICO-8's status: $status"
grep -qx "args=-width 720 -height 720 -root_path $CARD/Games/PICO8/ -run $CART" "$TMP/pico8.log" \
	|| fail "PICO-8 was not run with the cart at the panel's size: [$(grep args= "$TMP/pico8.log")]"
grep -qx "cwd=$CARD/Saves/PICO8" "$TMP/pico8.log" || fail "PICO-8 did not run in its HOME"
grep -qx "home=$CARD/Saves/PICO8" "$TMP/pico8.log" || fail "PICO-8's HOME is not Saves/PICO8"
grep -qx 'video=mali' "$TMP/pico8.log" || fail "PICO-8 does not draw through mali"
grep -qx "lib=$CARD/System/ports/lib:/usr/lib:/usr/lib/aarch64-linux-gnu" "$TMP/pico8.log" \
	|| fail "PICO-8 does not find the mali SDL2 first"
grep -qx "preload=$CARD/System/pico8/libsdl_sensor_shim.so" "$TMP/pico8.log" \
	|| fail "the sensor shim is not preloaded"
grep -q 'ANBERNIC-keys,a:b4,b:b3,x:b5,y:b6,' "$TMP/pico8.log" || fail "the pad is not mapped by position"
grep -qx "wget=$CARD/System/pico8/bin/wget" "$TMP/pico8.log" || fail "PICO-8 does not fetch through curl"
grep -qx 'transform_screen 0' "$CONFIG" || fail "the picture is turned"
grep -qx 'use_wget 1' "$CONFIG" || fail "PICO-8 does not fetch with wget"
grep -qx "root_path $CARD/Games/PICO8/" "$CONFIG" || fail "the carts are not in Games/PICO8"
grep -qx "cdata_path $CARD/Saves/PICO8/cdata/" "$CONFIG" || fail "the saves are not in Saves/PICO8"
grep -q "^--volume-keys $CARD --menu-quits [0-9][0-9]*$" "$TMP/volume.log" \
	|| fail "MENU held does not end PICO-8: [$(cat "$TMP/volume.log")]"
[ "$(cat "$TMP/amixer.log")" = "-q sset lineout volume 25" ] \
	|| fail "the sound card's volume was not given back: [$(cat "$TMP/amixer.log" 2>/dev/null)]"

# --- the player's own settings are kept, the card's written over theirs
printf 'volume 128\ntransform_screen 135\n' > "$CONFIG"
pico8 "$CART" > /dev/null || true
grep -qx 'volume 128' "$CONFIG" || fail "the player's own setting was lost"
grep -qx 'transform_screen 0' "$CONFIG" || fail "the card's setting was not written over the player's"
[ "$(grep -c '^transform_screen' "$CONFIG")" = 1 ] || fail "a setting is in config.txt twice"

# --- Splore
setup
pico8 "$CARD/Games/PICO8/Splore.splore" > /dev/null || true
grep -qx "args=-width 720 -height 720 -root_path $CARD/Games/PICO8/ -splore" "$TMP/pico8.log" \
	|| fail "Splore was not opened: [$(grep args= "$TMP/pico8.log")]"

# --- MENU held: PICO-8 ends at once
setup
echo 30 > "$TMP/plays"
: > "$TMP/hold"
start="$(date +%s)"
pico8 "$CART" > /dev/null || true
[ $(($(date +%s) - start)) -lt 10 ] || fail "MENU held did not end PICO-8"

# --- no PICO-8 in BIOS/: nothing runs
setup
rm -f "$CARD/BIOS/pico8.dat"
status=0
pico8 "$CART" > /dev/null || status=$?
[ "$status" = 2 ] || fail "pico8.sh ran with no PICO-8: $status"
[ ! -f "$TMP/pico8.log" ] || fail "PICO-8 ran without its pico8.dat"

# --- a cart that is not there
setup
status=0
pico8 "$CARD/Games/PICO8/none.p8" > /dev/null || status=$?
[ "$status" = 2 ] || fail "a missing cart did not fail: $status"

echo "test-pico8: ok"
