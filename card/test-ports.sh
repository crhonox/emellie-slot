#!/bin/sh
# ports.sh, off-device: 7-Zip, the ports, PortMaster and the dynamic loader are stand-ins that
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
TOOLS="$CARD/Tools/portmaster"
PM="$TOOLS/PortMaster"
PORT="$CARD/Games/PORTS/Cave Story.sh"

setup() {
	rm -rf "$CARD" "$TMP/run" "$TMP/bin" "$TMP/cpu" "$TMP"/*.log
	mkdir -p "$CARD/System/ports/bin" "$CARD/Games/PORTS" "$TMP/run" "$TMP/bin" "$TMP/cpu"
	cp "$HERE/System/ports.sh" "$HERE/System/pad.sh" "$CARD/System/"
	cp "$HERE/System/ports/control.txt" "$HERE/System/ports/mod.txt" "$CARD/System/ports/"
	: > "$CARD/System/ports/portmaster.7z"
	printf '#!/bin/sh\nexec bash "$@"\n' > "$CARD/System/ports/bin/bash"
	# PortMaster's launcher: where it ran and what it was given
	cat > "$CARD/System/ports/PortMaster.txt" <<-'SH'
		{
			echo "cwd=$(pwd)"
			echo "controlfolder=@SD@/Tools/portmaster/PortMaster"
			echo "hm=$HM_TOOLS_DIR $HM_PORTS_DIR $HM_SCRIPTS_DIR"
		} > "$TMP/portmaster.log"
	SH
	echo schedutil > "$TMP/cpu/scaling_governor"
	printf 'BASEOS_TARGET=rg35xxsp\n' > "$TMP/release"
	echo 2026.10.01-0433 > "$TMP/bundle-version"
	rm -f "$TMP/7z-fail"
	# 7-Zip: the bundle's version, and its tree laid out with every run noted
	cat > "$TMP/7zzs" <<-'SH'
		#!/bin/sh
		[ "$1 $2" = "e -so" ] && { cat "$TMP/bundle-version"; exit 0; }
		[ "$1" = e ] && exit 0
		echo "$*" >> "$TMP/7z.log"
		[ -f "$TMP/7z-fail" ] && exit 2
		out="${3#-o}"
		mkdir -p "$out/portmaster/PortMaster" "$out/portmaster/bin"
		cp "$TMP/bundle-version" "$out/portmaster/PortMaster/version"
		echo 'export CFW_NAME="Unknown"' > "$out/portmaster/PortMaster/device_info.txt"
		: > "$out/portmaster/PortMaster/pylibs.zip"
		[ -n "${5:-}" ] || : > "$out/portmaster/bin/python3.10"
		exit 0
	SH
	chmod +x "$TMP/7zzs"
	# the port: what it was run with
	cat > "$PORT" <<-'SH'
		{
			echo "cwd=$(pwd)"
			echo "home=$HOME"
			echo "xdg=$XDG_DATA_HOME"
			echo "lib=$LD_LIBRARY_PATH"
			echo "map=$SDL_GAMECONTROLLERCONFIG"
			echo "gov=$(cat "$TMP/cpu/scaling_governor")"
			echo "bash=${BASH_VERSION:+yes}"
		} > "$TMP/port.log"
		exit 4
	SH
}

ports() {
	TMP="$TMP" OMEGA_ROOT="$CARD" AGS_RUN="$TMP/run" AGS_LOADER=sh AGS_7Z="$TMP/7zzs" \
		AGS_BASH="$TMP/bin/bash" AGS_CPUFREQ="$TMP/cpu" AGS_BASEOS_RELEASE="$TMP/release" \
		AGS_PORTS_NET_WAIT_S=0 sh "$CARD/System/ports.sh" "$@"
}

# --- the first port: PortMaster laid out whole, Emellie Slot's files in it, the port run
setup
status=0
ports "$PORT" > /dev/null || status=$?
[ "$status" = 4 ] || fail "ports.sh did not leave with the port's status: $status"
[ "$(cat "$TMP/7z.log")" = "x -y -o$CARD/Tools $CARD/System/ports/portmaster.7z" ] \
	|| fail "the bundle was not laid out whole: [$(cat "$TMP/7z.log")]"
[ "$(cat "$TOOLS/.bundle")" = 2026.10.01-0433 ] || fail "the bundle laid out was not noted"
[ ! -f "$PM/pylibs.zip" ] || fail "a stale pylibs.zip was left to unpack over the bundle"
[ -x "$TMP/bin/bash" ] || fail "bash was not put in"
grep -qx "export controlfolder=\"$PM\"" "$PM/control.txt" || fail "control.txt was not written with the card"
grep -qx "export directory=\"$CARD/Games/\"" "$PM/control.txt" || fail "the ports are not in Games/"
grep -q '@SD@' "$PM/control.txt" && fail "control.txt has a path left unwritten"
[ -f "$PM/mod_Unknown.txt" ] || fail "the mod was not written by device_info.txt's name for the firmware"
grep -q "LD_LIBRARY_PATH=$CARD/System/ports/lib:" "$PM/mod_Unknown.txt" \
	|| fail "gptokeyb does not get the mali SDL2"
grep -qx "cwd=$CARD/Games/PORTS" "$TMP/port.log" || fail "the port did not run in its folder"
grep -qx "home=$CARD/Saves/PORTS" "$TMP/port.log" || fail "the port's HOME is not Saves/PORTS"
grep -qx "xdg=$TOOLS" "$TMP/port.log" || fail "the port does not find PortMaster by XDG_DATA_HOME"
grep -qx "lib=$CARD/System/ports/lib:/usr/lib:/usr/lib/aarch64-linux-gnu:$CARD/System/ports/sdl2" \
	"$TMP/port.log" || fail "the port does not find the mali SDL2 first: [$(grep lib= "$TMP/port.log")]"
grep -q 'ANBERNIC-keys,a:b4,b:b3,x:b5,y:b6,' "$TMP/port.log" || fail "the pad is not mapped by position"
grep -q 'lefttrigger:b12,righttrigger:b13' "$TMP/port.log" || fail "the RG35XX SP's triggers are not b12/b13"
grep -qx 'gov=performance' "$TMP/port.log" || fail "the port did not run under performance"
grep -qx 'bash=yes' "$TMP/port.log" || fail "the port was not run by bash"
[ "$(cat "$TMP/cpu/scaling_governor")" = schedutil ] || fail "the governor was not given back"

# --- the same bundle again: nothing laid out
rm -f "$TMP/7z.log"
ports "$PORT" > /dev/null || true
[ ! -f "$TMP/7z.log" ] || fail "the bundle was laid out again: [$(cat "$TMP/7z.log")]"

# --- the player's settings in control.txt's place are written over all the same
echo "# old" > "$PM/control.txt"
ports "$PORT" > /dev/null || true
grep -q '# old' "$PM/control.txt" && fail "control.txt was left as an update had it"

# --- a newer bundle: PortMaster/ over what is there, its config/ kept
echo 2026.11.02-1200 > "$TMP/bundle-version"
ports "$PORT" > /dev/null || true
[ "$(cat "$TMP/7z.log")" = "x -y -o$CARD/Tools $CARD/System/ports/portmaster.7z portmaster/PortMaster -x!portmaster/PortMaster/config" ] \
	|| fail "a newer bundle was not laid over PortMaster/: [$(cat "$TMP/7z.log")]"
[ "$(cat "$TOOLS/.bundle")" = 2026.11.02-1200 ] || fail "the newer bundle was not noted"

# --- PortMaster updated itself past a newer bundle: left as it is
echo 2026.12.24-0000 > "$PM/version"
echo 2026.12.01-0000 > "$TMP/bundle-version"
rm -f "$TMP/7z.log"
ports "$PORT" > /dev/null || true
[ ! -f "$TMP/7z.log" ] || fail "an updated PortMaster was laid over by an older bundle"
[ "$(cat "$PM/version")" = 2026.12.24-0000 ] || fail "the updated PortMaster was touched"
[ "$(cat "$TOOLS/.bundle")" = 2026.12.01-0000 ] || fail "the bundle passed over was not noted"

# --- a layout cut short: laid out whole again
rm -f "$TOOLS/.bundle" "$TMP/7z.log"
ports "$PORT" > /dev/null || true
[ "$(cat "$TMP/7z.log")" = "x -y -o$CARD/Tools $CARD/System/ports/portmaster.7z" ] \
	|| fail "a layout cut short was not done whole: [$(cat "$TMP/7z.log")]"

# --- a layout that fails: no port, nothing noted
setup
: > "$TMP/7z-fail"
status=0
ports "$PORT" > /dev/null || status=$?
[ "$status" = 2 ] || fail "a failed layout did not fail: $status"
[ ! -f "$TMP/port.log" ] || fail "a port ran over a failed layout"
[ ! -f "$TOOLS/.bundle" ] || fail "a failed layout was noted as done"

# --- no bundle and nothing laid out
setup
: > "$TMP/bundle-version"
status=0
ports "$PORT" > /dev/null || status=$?
[ "$status" = 2 ] || fail "ports.sh ran with no PortMaster: $status"
[ ! -f "$TMP/port.log" ] || fail "a port ran with no PortMaster"

# --- a port that is not there
setup
status=0
ports "$CARD/Games/PORTS/None.sh" > /dev/null || status=$?
[ "$status" = 2 ] || fail "a missing port did not fail: $status"
[ ! -f "$TMP/7z.log" ] || fail "PortMaster was laid out for a missing port"

# --- PortMaster itself: its launcher with the card's paths, in its folder
setup
ports --manager > /dev/null || fail "ports.sh --manager failed"
grep -qx "cwd=$PM" "$TMP/portmaster.log" || fail "PortMaster did not run in its folder"
grep -qx "controlfolder=$PM" "$TMP/portmaster.log" || fail "PortMaster's launcher was not given the card"
grep -qx "hm=$TOOLS $CARD/Games/PORTS $CARD/Games/PORTS" "$TMP/portmaster.log" \
	|| fail "PortMaster does not install into Games/PORTS: [$(grep hm= "$TMP/portmaster.log")]"
[ ! -f "$TMP/run/portmaster.sh" ] || fail "PortMaster's launcher was left in RUN"

# --- the shelf's PortMaster.sh: PortMaster, not a port
setup
printf '#!/bin/bash\n' > "$CARD/Games/PORTS/PortMaster.sh"
ports "$CARD/Games/PORTS/PortMaster.sh" > /dev/null || fail "the shelf's PortMaster failed"
grep -qx "cwd=$PM" "$TMP/portmaster.log" || fail "the shelf's PortMaster.sh did not run PortMaster"

# --- the RG CubeXX: two sticks
setup
printf 'BASEOS_TARGET=rgcubexx\n' > "$TMP/release"
ports "$PORT" > /dev/null || true
grep -q 'rightx:a2,righty:a3,rightstick:b15' "$TMP/port.log" || fail "the CubeXX's right stick is not mapped"

echo "test-ports: ok"
