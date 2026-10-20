#!/bin/sh
# Runs a DS game in DSperate, which launch_frontend.sh hands the handheld over to once omega-zero
# has left. DSperate draws straight to fb0 through the mali-fbdev SDL2 beside it in dsperate/lib
# (BaseOS's own SDL2 cannot reach the panel), keeps its own battery saves, states and resume,
# and comes back here when MENU+START quits it.
SD="${OMEGA_ROOT:-/mnt/sdcard}"
SYS="$SD/System"
DIR="${AGS_DSPERATE_DIR:-$SYS/dsperate}"
RELEASE="${AGS_BASEOS_RELEASE:-/etc/baseos-release}"
CPU="${AGS_CPUFREQ:-/sys/devices/system/cpu/cpu0/cpufreq}"
LOADER="${AGS_LOADER:-/lib/ld-linux-aarch64.so.1}"
CONFIG_DIR="$SD/Config/dsperate"

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

# The sticks of each model, as spruceOS sorts the H700 handhelds; omega-power's PadLayout holds
# the same list for the controls omega-zero shows. A model BaseOS does not name has two.
pad_layout() {
	case "$1" in
	rg35xxsp | rg35xxplus | rg34xx | rgsp | rg28xx) echo nostick ;;
	rg40xxv) echo 1stick ;;
	*) echo 2stick ;;
	esac
}

# The pad as the mali SDL2 numbers it: the node's three phantom keys (ESC, VOL-, VOL+) come
# first, so A is b3; above MENU the stick clicks interleave with the triggers, one map for each
# layout. As spruceOS's AnbernicXXCommon.cfg, which measured them on an RG CubeXX.
pad_map() {
	base="19000000010000000100000000010000,ANBERNIC-keys,a:b3,b:b4,x:b6,y:b5,leftshoulder:b7,rightshoulder:b8,guide:b11,start:b10,back:b9,dpup:h0.1,dpdown:h0.4,dpleft:h0.8,dpright:h0.2"
	case "$1" in
	nostick)
		echo "$base,lefttrigger:b12,righttrigger:b13,leftx:a0,lefty:a1,platform:Linux," ;;
	1stick)
		echo "$base,lefttrigger:b13,righttrigger:b14,leftx:a0,lefty:a1,leftstick:b12,platform:Linux," ;;
	*)
		echo "$base,lefttrigger:b13,righttrigger:b14,leftx:a0,lefty:a1,leftstick:b12,rightx:a2,righty:a3,rightstick:b15,platform:Linux," ;;
	esac
}

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

export HOME="$DIR"
export XDG_CONFIG_HOME="$SD/Config"
export LD_LIBRARY_PATH="$DIR/lib:/usr/lib:/usr/lib/aarch64-linux-gnu"
# SDL reads the map from SDL_GAMECONTROLLERCONFIG; without it the pad is no game controller,
# and DSperate, which opens only those, hears no button at all. DSperate binds by position
# (SDL's a is the bottom face button), so A/B and X/Y are swapped from the printed names, as
# spruceOS's export_sdl_gamecontroller_map positional does.
export SDL_GAMECONTROLLERCONFIG="$(pad_map "$layout" |
	sed -e 's/a:\([^,]*\),b:\([^,]*\)/a:\2,b:\1/' -e 's/x:\([^,]*\),y:\([^,]*\)/x:\2,y:\1/')"
# the RG28XX's panel stands on its side
[ "$target" = "rg28xx" ] && export DS_ROTATE=270

# the DS wants every cycle the H700 has; the governor omega-zero ran under comes back after
governor="$(cat "$CPU/scaling_governor" 2>/dev/null)"
echo performance > "$CPU/scaling_governor" 2>/dev/null

cd "$DIR" || exit 2
$LOADER "$DIR/dsperate" "$rom" --fullscreen --config "$CONFIG" $resume
status=$?

[ -n "$governor" ] && echo "$governor" > "$CPU/scaling_governor" 2>/dev/null
exit "$status"
