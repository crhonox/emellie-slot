#!/bin/sh
# Runs a PICO-8 cart, or Splore, in the player's own PICO-8, which launch_frontend.sh hands the
# handheld over to once omega-zero has left, as it does a DS game to nds.sh. As spruceOS runs it.
#
#   pico8.sh CART       a cart in Games/PICO8, cart.p8 or cart.p8.png
#   pico8.sh X.splore   Splore, PICO-8's own cart browser, Games/PICO8/Splore.splore
#
# PICO-8 is not free to give away: it is the player's, the Raspberry Pi build's pico8_64 and
# pico8.dat in BIOS/. It draws through the mali-fbdev SDL2 PortMaster draws through, with a shim
# that keeps it from asking that SDL2 for the sensors it was built without. Its settings, the
# carts' saves and what Splore fetches live in Saves/PICO8/.lexaloffle/pico-8; Splore saves
# the carts it is asked to into Games/PICO8. MENU held a second leaves it (omega-zero
# --volume-keys --menu-quits), the volume keys stepping the sound card meanwhile, as in a port.
SD="${OMEGA_ROOT:-/mnt/sdcard}"
SYS="$SD/System"
DIR="$SYS/pico8"
BIOS="$SD/BIOS"
GAMES="$SD/Games/PICO8"
RELEASE="${AGS_BASEOS_RELEASE:-/etc/baseos-release}"
LOADER="${AGS_LOADER:-/lib/ld-linux-aarch64.so.1}"
FB="${AGS_FB:-/sys/class/graphics/fb0}"
NET_WAIT_S="${AGS_PICO8_NET_WAIT_S:-15}"
AMIXER="${AGS_AMIXER:-amixer}"

log() {
	echo "pico8: $*"
}

cart="$1"
if [ -z "$cart" ] || [ ! -f "$cart" ]; then
	log "no cart at '$cart'"
	exit 2
fi
bin="$BIOS/pico8_64"
if [ ! -f "$bin" ] || [ ! -f "$BIOS/pico8.dat" ]; then
	log "no PICO-8: put the Raspberry Pi build's pico8_64 and pico8.dat in $BIOS"
	exit 2
fi
[ -x "$bin" ] || chmod +x "$bin" 2>/dev/null

# Sets KEY to VALUE in PICO-8's config.txt, over what it says or after it
setting() {
	if grep -q "^$1 " "$CONFIG"; then
		awk -v k="$1" -v v="$2" '$1 == k { print k " " v; next } { print }' "$CONFIG" \
			> "$CONFIG.tmp" && mv -f "$CONFIG.tmp" "$CONFIG"
	else
		echo "$1 $2" >> "$CONFIG"
	fi
}

export HOME="$SD/Saves/PICO8"
CONFIG="$HOME/.lexaloffle/pico-8/config.txt"
mkdir -p "$GAMES" "$HOME/.lexaloffle/pico-8" "$HOME/cdata" "$SD/Screenshots/PICO8"
# PICO-8 writes the rest of its settings itself the first time it leaves; these are the card's.
# It is drawn the right way up (the RG28XX's SDL2 turns the panel itself), takes no keyboard
# keys for its buttons, and fetches through wget, which is curl here (pico8/bin/wget)
[ -f "$CONFIG" ] || : > "$CONFIG"
setting transform_screen 0
setting button_keys "0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0"
setting use_wget 1
setting root_path "$GAMES/"
setting cdata_path "$HOME/cdata/"
setting desktop_path "$SD/Screenshots/PICO8"

target="$(sed -n 's/^BASEOS_TARGET=//p' "$RELEASE" 2>/dev/null | tr -d '"' | tr -d ' ')"
# pad_layout and pad_map_positional
. "$SYS/pad.sh"

# The panel's size, from the mode in use (U:720x720p-60); fb0's virtual size holds its buffers
size="$(sed -n '1s/^[^:]*:\([0-9]*\)x\([0-9]*\).*/\1 \2/p' "$FB/modes" 2>/dev/null)"
[ -n "$size" ] || size="640 480"
width="${size% *}" height="${size#* }"

export SDL_VIDEODRIVER=mali
export LD_LIBRARY_PATH="$SYS/ports/lib:/usr/lib:/usr/lib/aarch64-linux-gnu"
export PATH="$DIR/bin:$PATH"
# By position, as spruceOS gives PICO-8 the pad: B, the lower button, is PICO-8's O and A its X,
# as Z and X lie on a keyboard
export SDL_GAMECONTROLLERCONFIG="$(pad_map_positional "$(pad_layout "$target")")"

case "$cart" in
*.splore)
	# Splore wants the network; one brought up from the card's networks when there is none
	if ! ip route 2>/dev/null | grep -q '^default'; then
		sh "$SYS/wifi.sh" up > /dev/null 2>&1
		i=0
		while [ "$i" -lt "$NET_WAIT_S" ] && ! ip route 2>/dev/null | grep -q '^default'; do
			sleep 1
			i=$((i + 1))
		done
	fi
	set -- -splore
	;;
*) set -- -run "$cart" ;;
esac

hw_volume="$($AMIXER sget 'lineout volume' 2>/dev/null | sed -n 's/.*: Playback \([0-9][0-9]*\) .*/\1/p' | head -n 1)"

# the shim under PICO-8 alone, not under what it runs
shim=
[ -f "$DIR/libsdl_sensor_shim.so" ] && shim="$DIR/libsdl_sensor_shim.so"
cd "$HOME" || exit 2
LD_PRELOAD="$shim" "$bin" -width "$width" -height "$height" -root_path "$GAMES/" "$@" &
game=$!
# the volume keys while it plays, and MENU held to end it
$LOADER "$SYS/emellie-slot" --volume-keys "$SD" --menu-quits "$game" &
keys=$!
wait "$game"
status=$?
kill "$keys" 2>/dev/null
[ -n "$hw_volume" ] && $AMIXER -q sset 'lineout volume' "$hw_volume" > /dev/null 2>&1
sync
exit "$status"
