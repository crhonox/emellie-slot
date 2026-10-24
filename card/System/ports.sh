#!/bin/sh
# Runs a port from PortMaster, or PortMaster itself, which launch_frontend.sh hands the handheld
# over to once omega-zero has left, as it does a DS game to nds.sh.
#
#   ports.sh SCRIPT      a port, its .sh in Games/PORTS; Games/PORTS/PortMaster.sh, PortMaster
#   ports.sh --manager   PortMaster, to get ports
#
# PortMaster and the Python it runs on are spruceOS's bundle, System/ports/portmaster.7z, laid
# out in Tools/portmaster the first time and again when the bundle is newer: there, System/ can
# be replaced whole without losing what PortMaster has installed or updated since. Nothing of it
# runs before a port does. Ports keep their files in Games/PORTS, their settings in Saves/PORTS.
SD="${OMEGA_ROOT:-/mnt/sdcard}"
SYS="$SD/System"
DIR="$SYS/ports"
TOOLS="$SD/Tools/portmaster"
PM="$TOOLS/PortMaster"
PORTS="$SD/Games/PORTS"
BUNDLE="$DIR/portmaster.7z"
SEVENZIP="${AGS_7Z:-$DIR/bin/7zzs}"
BASH_BIN="${AGS_BASH:-/bin/bash}"
RUN="${AGS_RUN:-/run}"
RELEASE="${AGS_BASEOS_RELEASE:-/etc/baseos-release}"
CPU="${AGS_CPUFREQ:-/sys/devices/system/cpu/cpu0/cpufreq}"
LOADER="${AGS_LOADER:-/lib/ld-linux-aarch64.so.1}"
NET_WAIT_S="${AGS_PORTS_NET_WAIT_S:-15}"
AMIXER="${AGS_AMIXER:-amixer}"

log() {
	echo "ports: $*"
}

# 7-Zip laying the bundle out with ARGS, its progress drawn on the panel as it goes: omega-zero's
# own progress screen reads the NN% 7-Zip writes. 7-Zip's status, through the pipe.
lay_out() {
	{
		"$SEVENZIP" x -y -bso0 -bsp1 -o"$SD/Tools" "$BUNDLE" "$@"
		echo $? > "$RUN/ports-7z.status"
	} | {
		$LOADER "$SYS/emellie-slot" --progress "$SD" "INSTALLING PORTMASTER" > /dev/null 2>&1
		# a screen that failed must not take 7-Zip down with it
		cat > /dev/null
	}
	status="$(cat "$RUN/ports-7z.status" 2> /dev/null)"
	rm -f "$RUN/ports-7z.status"
	return "${status:-1}"
}

# The bundle's PortMaster in Tools/portmaster, unless this very bundle was laid there already or
# PortMaster has since updated itself past it. Versions are YYYY.MM.DD-HHMM, so they sort as
# they read. Tools/portmaster/.bundle names the bundle last laid out whole; without it (never, or
# cut short) everything is laid out, with it only PortMaster/, the player's config/ kept.
install_portmaster() {
	want="$("$SEVENZIP" e -so "$BUNDLE" portmaster/PortMaster/version 2> /dev/null | tr -d '\r\n')"
	laid="$(cat "$TOOLS/.bundle" 2> /dev/null)"
	if [ -z "$want" ]; then
		[ -n "$laid" ] && return 0
		log "no PortMaster at $BUNDLE"
		return 1
	fi
	[ "$want" = "$laid" ] && return 0
	have="$(cat "$PM/version" 2> /dev/null | tr -d '\r\n')"
	if [ -n "$laid" ] && [ -n "$have" ] \
		&& [ "$(printf '%s\n%s\n' "$have" "$want" | sort | tail -n 1)" = "$have" ]; then
		echo "$want" > "$TOOLS/.bundle"
		return 0
	fi
	log "laying out PortMaster $want over '${have:-nothing}'"
	mkdir -p "$SD/Tools"
	if [ -n "$laid" ]; then
		lay_out portmaster/PortMaster '-x!portmaster/PortMaster/config'
	else
		lay_out
	fi
	status=$?
	sync
	if [ "$status" != 0 ]; then
		log "PortMaster could not be laid out, status $status"
		return 1
	fi
	# a self-update's pylibs.zip is older than what was just written over it
	rm -f "$PM/pylibs.zip"
	echo "$want" > "$TOOLS/.bundle"
}

# Ports and PortMaster are bash scripts, and BaseOS's userland is BusyBox: a static aarch64 bash
# goes in, as spruceOS puts it there too
install_bash() {
	[ -x "$BASH_BIN" ] && return 0
	cp "$DIR/bin/bash" "$BASH_BIN" 2> /dev/null && chmod +x "$BASH_BIN" 2> /dev/null && return 0
	log "no bash at $BASH_BIN and it could not be put there"
	return 1
}

# Emellie Slot's control.txt and mod into the PortMaster folder, which ports find as
# $XDG_DATA_HOME/PortMaster, written again at each launch over whatever an update left. The mod
# goes by the name device_info.txt gives this firmware, which ports source it by.
stage() {
	sed "s|@SD@|$SD|g" "$DIR/control.txt" > "$PM/control.txt"
	cfw="$("$BASH_BIN" -c '. "$1/device_info.txt" > /dev/null 2>&1; printf %s "$CFW_NAME"' _ "$PM")"
	sed "s|@SD@|$SD|g" "$DIR/mod.txt" > "$PM/mod_${cfw:-Unknown}.txt"
}

# PortMaster wants the network; one brought up from the card's networks when there is none, as
# when PortMaster is run before omega-zero
network() {
	ip route 2> /dev/null | grep -q '^default' && return 0
	sh "$SYS/wifi.sh" up > /dev/null 2>&1
	i=0
	while [ "$i" -lt "$NET_WAIT_S" ]; do
		ip route 2> /dev/null | grep -q '^default' && return 0
		sleep 1
		i=$((i + 1))
	done
	log "no network for PortMaster"
}

case "${1:-}" in
--manager) port= ;;
'')
	echo "usage: ports.sh SCRIPT|--manager" >&2
	exit 2
	;;
*)
	port="$1"
	# the shelf's PortMaster, which the build puts among the ports
	[ "$(basename "$port")" = PortMaster.sh ] && port=
	if [ -n "$port" ] && [ ! -f "$port" ]; then
		log "no port at '$port'"
		exit 2
	fi
	;;
esac

install_bash || exit 2
install_portmaster || exit 2
stage

target="$(sed -n 's/^BASEOS_TARGET=//p' "$RELEASE" 2> /dev/null | tr -d '"' | tr -d ' ')"
# pad_layout and pad_map
. "$SYS/pad.sh"

mkdir -p "$PORTS" "$SD/Saves/PORTS"
export HOME="$SD/Saves/PORTS"
export XDG_DATA_HOME="$TOOLS"
export HM_TOOLS_DIR="$TOOLS"
export HM_PORTS_DIR="$PORTS"
export HM_SCRIPTS_DIR="$PORTS"
export PATH="$DIR/bin:$PATH"
export LC_ALL=C
# The mali SDL2 first, the one that reaches the panel and classifies the pad; the vendor one
# last, for the ports mod.txt moves it ahead for
export LD_LIBRARY_PATH="$DIR/lib:/usr/lib:/usr/lib/aarch64-linux-gnu:$DIR/sdl2"
# By the printed names, Nintendo's layout: A is the right face button, as on the pad
export SDL_GAMECONTROLLERCONFIG="$(pad_map "$(pad_layout "$target")")"

governor="$(cat "$CPU/scaling_governor" 2> /dev/null)"
echo performance > "$CPU/scaling_governor" 2> /dev/null

# The volume keys, which nothing else hears while a port plays, step the sound card's own volume
# from omega-zero's level (omega-zero --volume-keys); the control gets back its level after
hw_volume="$($AMIXER sget 'lineout volume' 2> /dev/null | sed -n 's/.*: Playback \([0-9][0-9]*\) .*/\1/p' | head -n 1)"
$LOADER "$SYS/emellie-slot" --volume-keys "$SD" &
volume_keys=$!

# Whatever way this leaves: the keys stopped, the sound card's volume and the governor back.
# A port that leaves its keyboard behind would have it typing into omega-zero.
finish() {
	killall gptokeyb gptokeyb2 > /dev/null 2>&1
	kill "$volume_keys" 2> /dev/null
	[ -n "$hw_volume" ] && $AMIXER -q sset 'lineout volume' "$hw_volume" > /dev/null 2>&1
	sync
	[ -n "$governor" ] && echo "$governor" > "$CPU/scaling_governor" 2> /dev/null
}
trap finish EXIT

if [ -z "$port" ]; then
	network
	script="$RUN/portmaster.sh"
	sed "s|@SD@|$SD|g" "$DIR/PortMaster.txt" > "$script"
	cd "$PM" || exit 2
	"$BASH_BIN" "$script"
	status=$?
	rm -f "$script"
else
	cd "$(dirname "$port")" || exit 2
	"$BASH_BIN" "$port"
	status=$?
fi

exit "$status"
