#!/bin/sh
# Runs a DS game in DSperate, which launch_frontend.sh hands the handheld over to once omega-zero
# has left. DSperate draws straight to fb0 through the mali-fbdev SDL2 beside it in dsperate/lib
# (BaseOS's own SDL2 cannot reach the panel), keeps its own battery saves, states and resume,
# and comes back here when MENU held, or MENU+SELECT, quits it. VOL+/VOL- are its volume, SELECT
# with them the panel's light, which brightness.sh keeps in the frontend's steps.
SD="${OMEGA_ROOT:-/mnt/sdcard}"
SYS="$SD/System"
DIR="${AGS_DSPERATE_DIR:-$SYS/dsperate}"
RELEASE="${AGS_BASEOS_RELEASE:-/etc/baseos-release}"
CPU="${AGS_CPUFREQ:-/sys/devices/system/cpu/cpu0/cpufreq}"
LOADER="${AGS_LOADER:-/lib/ld-linux-aarch64.so.1}"
CONFIG_DIR="$SD/Config/dsperate"
RUN="${AGS_RUN:-/run}"

rom="$1"
# held A on the shelf: started afresh, DSperate's own resume left where it is
resume=
[ "${2:-}" = --clean ] && resume=--no-autoload
if [ -z "$rom" ] || [ ! -f "$rom" ]; then
	echo "nds: no game at '$rom'"
	exit 2
fi
if [ ! -x "$DIR/dsperate" ]; then
	echo "nds: no emulator at $DIR/dsperate"
	exit 2
fi

target="$(sed -n 's/^BASEOS_TARGET=//p' "$RELEASE" 2>/dev/null | tr -d '"' | tr -d ' ')"

# pad_layout and pad_map_positional
. "$SYS/pad.sh"

layout="$(pad_layout "$target")"
CONFIG="$CONFIG_DIR/nds-$layout.ini"

mkdir -p "$CONFIG_DIR" "$SD/Saves/NDS" "$SD/States/NDS" "$SD/BIOS/nds" "$SD/Screenshots"
# The settings are seeded once for each kind of pad from the copy beside the emulator, the
# card's paths and the pad's stylus written in; from then on they are the player's, DSperate's
# own menu saving to them. One card keeps a set for each handheld it is played in.
if [ ! -f "$CONFIG" ]; then
	awk -v sd="$SD" -v pad="$DIR/pad-$layout.ini" '
		/^@PAD@$/ { while ((getline line < pad) > 0) print line; next }
		{ gsub(/@SD@/, sd); print }
	' "$DIR/nds.ini" > "$CONFIG.tmp" && mv -f "$CONFIG.tmp" "$CONFIG"
fi
# The hotkeys are the card's, not the player's: [hotkeys] and [padhotkeys] are taken again from
# the copy beside the emulator at every start, as X on the DS's Controls does (ds_hotkeys.rs),
# so one bound in DSperate's menu lasts until the game is left.
awk -v shipped="$DIR/nds.ini" '
	function hot(line, t) {
		t = line; gsub(/^[ \t]+|[ \t]+$/, "", t)
		if (t ~ /^\[.*\]$/) { t = substr(t, 2, length(t) - 2); gsub(/^[ \t]+|[ \t]+$/, "", t); inside = t == "hotkeys" || t == "padhotkeys" }
		return inside
	}
	!hot($0) { print; last = $0 }
	END {
		if (NR && last != "") print ""
		inside = 0
		while ((getline line < shipped) > 0) if (hot(line)) print line
	}
' "$CONFIG" > "$CONFIG.tmp" && mv -f "$CONFIG.tmp" "$CONFIG"

export HOME="$DIR"
export XDG_CONFIG_HOME="$SD/Config"
export LD_LIBRARY_PATH="$DIR/lib:/usr/lib:/usr/lib/aarch64-linux-gnu"
# SDL reads the map from SDL_GAMECONTROLLERCONFIG; without it the pad is no game controller,
# and DSperate, which opens only those, hears no button at all. DSperate binds by position.
export SDL_GAMECONTROLLERCONFIG="$(pad_map_positional "$layout")"
# the RG28XX's panel stands on its side
[ "$target" = "rg28xx" ] && export DS_ROTATE=270

# the DS wants every cycle the H700 has; the governor omega-zero ran under comes back after
governor="$(cat "$CPU/scaling_governor" 2>/dev/null)"
echo performance > "$CPU/scaling_governor" 2>/dev/null

# The frontend's volume, the speaker's, as DSperate's own to start from: it steps it by itself
state="$SD/Config/emellie-slot.state"
volume="$(sed -n 's/^volume=//p' "$state" 2>/dev/null)"
case "$volume" in '' | *[!0-9]*) volume=60 ;; esac
[ "$(sed -n 's/^muted=//p' "$state" 2>/dev/null)" = true ] && volume=0
# VOL+/VOL- come from an input node of their own, which SDL opens no controller for: DSperate
# reads them itself (cores/dsperate/emellie-hotkeys.patch), and SELECT with them runs this
export DS_VOLUME_KEYS=1
export DS_BRIGHTNESS="sh $SYS/brightness.sh"
# POWER held 2 s, as spruceOS's default: DSperate leaves with the game saved and this file
# written, and launch_frontend.sh switches the handheld off
export DS_POWER_OFF="$RUN/omega-power-off"
export DS_POWER_HOLD_MS=2000

# MENU held a second quits, over whatever the player's settings say
cd "$DIR" || exit 2
$LOADER "$DIR/dsperate" "$rom" --fullscreen --config "$CONFIG" $resume \
	--volume "$volume" --set padhotkeys.quit_hold_ms=1000
status=$?

[ -n "$governor" ] && echo "$governor" > "$CPU/scaling_governor" 2>/dev/null
exit "$status"
