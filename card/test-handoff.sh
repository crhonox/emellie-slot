#!/bin/sh
# launch_frontend.sh's handoff loop and nds.sh, off-device: omega-zero, DSperate and the dynamic
# loader are stand-ins that note how they were run.
set -eu

HERE="$(cd "$(dirname "$0")" && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT HUP INT TERM

fail() {
	echo "$*" >&2
	exit 1
}

setup() {
	rm -rf "$TMP/card" "$TMP/run"
	mkdir -p "$TMP/card/System/dsperate" "$TMP/card/Games/NDS" "$TMP/run"
	cp "$HERE/System/launch_frontend.sh" "$HERE/System/nds.sh" "$TMP/card/System/"
	cp "$HERE/System/dsperate/"*.ini "$TMP/card/System/dsperate/"
	printf '#!/bin/sh\nexit 0\n' > "$TMP/card/System/led.sh"
	: > "$TMP/card/Games/NDS/Game (USA).nds"
}

# omega-zero as a script: each start notes OMEGA_RETURNED, and the first STARTS of them each
# write the handoff for the next line of $TMP/handoffs
omega() {
	cat > "$TMP/card/System/emellie-slot" <<-'SH'
		#!/bin/sh
		echo "returned=$OMEGA_RETURNED" >> "$TMP/omega.log"
		n="$(wc -l < "$TMP/omega.log")"
		line="$(sed -n "${n}p" "$TMP/handoffs" 2>/dev/null)"
		[ -n "$line" ] && printf '%b' "$line" > "$OMEGA_HANDOFF"
		exit 0
	SH
}

launch() {
	TMP="$TMP" OMEGA_ROOT="$TMP/card" AGS_RUN="$TMP/run" AGS_LOADER=sh \
		sh "$TMP/card/System/launch_frontend.sh"
}

# --- no handoff: omega-zero runs once and the script leaves with it
setup
omega
: > "$TMP/handoffs"
rm -f "$TMP/omega.log"
launch || fail "launch_frontend.sh failed with no handoff"
[ "$(cat "$TMP/omega.log")" = "returned=0" ] \
	|| fail "omega-zero did not run once, not returned: [$(cat "$TMP/omega.log")]"

# --- a DS game: nds.sh runs it, then omega-zero starts again, returned
setup
omega
printf '#!/bin/sh\necho "$@" >> "$TMP/nds.log"\nexit 0\n' > "$TMP/card/System/nds.sh"
printf '%s\n' "platform=NDS\\nstem=Game (USA)\\nrom=$TMP/card/Games/NDS/Game (USA).nds\\n" > "$TMP/handoffs"
rm -f "$TMP/omega.log" "$TMP/nds.log"
launch || fail "launch_frontend.sh failed over a handoff"
[ "$(cat "$TMP/nds.log")" = "$TMP/card/Games/NDS/Game (USA).nds" ] \
	|| fail "nds.sh was not run with the game: [$(cat "$TMP/nds.log" 2>/dev/null)]"

# --- held A: the game starts afresh
setup
omega
printf '#!/bin/sh\necho "$@" >> "$TMP/nds.log"\nexit 0\n' > "$TMP/card/System/nds.sh"
printf '%s\n' "platform=NDS\\nrom=/g.nds\\nclean=1\\n" > "$TMP/handoffs"
rm -f "$TMP/omega.log" "$TMP/nds.log"
launch || fail "launch_frontend.sh failed over a clean handoff"
[ "$(cat "$TMP/nds.log")" = "/g.nds --clean" ] \
	|| fail "a clean start did not reach nds.sh: [$(cat "$TMP/nds.log" 2>/dev/null)]"
[ "$(tr '\n' ' ' < "$TMP/omega.log")" = "returned=0 returned=1 " ] \
	|| fail "omega-zero did not start again returned: [$(cat "$TMP/omega.log")]"
[ ! -f "$TMP/run/omega-handoff" ] || fail "the handoff was left behind"

# --- a system nothing runs: omega-zero just starts again
setup
omega
printf '#!/bin/sh\necho "$@" >> "$TMP/nds.log"\n' > "$TMP/card/System/nds.sh"
printf '%s\n' 'platform=N64\nrom=/x.z64\n' > "$TMP/handoffs"
rm -f "$TMP/omega.log" "$TMP/nds.log"
launch || fail "launch_frontend.sh failed over a handoff nothing runs"
[ ! -f "$TMP/nds.log" ] || fail "nds.sh ran for an N64 game"
[ "$(wc -l < "$TMP/omega.log" | tr -d ' ')" = 2 ] || fail "omega-zero did not start again"

# --- a handoff left in RUN from before is not acted on
setup
omega
: > "$TMP/handoffs"
printf 'platform=NDS\nrom=/stale.nds\n' > "$TMP/run/omega-handoff"
printf '#!/bin/sh\necho "$@" >> "$TMP/nds.log"\n' > "$TMP/card/System/nds.sh"
rm -f "$TMP/omega.log" "$TMP/nds.log"
launch || fail "launch_frontend.sh failed over a stale handoff"
[ ! -f "$TMP/nds.log" ] || fail "a stale handoff was acted on"

# --- nds.sh: seeds the settings, maps the pad for the model, runs DSperate, restores the governor
setup
mkdir -p "$TMP/cpu"
echo schedutil > "$TMP/cpu/scaling_governor"
printf 'BASEOS_TARGET=rg35xxsp\n' > "$TMP/release"
cat > "$TMP/card/System/dsperate/dsperate" <<-'SH'
	#!/bin/sh
	{
		echo "args=$*"
		echo "map=$SDL_GAMECONTROLLERCONFIG"
		echo "lib=$LD_LIBRARY_PATH"
		echo "gov=$(cat "$CPUDIR/scaling_governor")"
		echo "rotate=${DS_ROTATE:-}"
	} > "$TMP/dsperate.log"
	exit 3
SH
chmod +x "$TMP/card/System/dsperate/dsperate"
rom="$TMP/card/Games/NDS/Game (USA).nds"
status=0
TMP="$TMP" CPUDIR="$TMP/cpu" OMEGA_ROOT="$TMP/card" AGS_LOADER=sh AGS_CPUFREQ="$TMP/cpu" \
	AGS_BASEOS_RELEASE="$TMP/release" sh "$TMP/card/System/nds.sh" "$rom" || status=$?
[ "$status" = 3 ] || fail "nds.sh did not leave with DSperate's status: $status"
grep -qx "args=$rom --fullscreen --config $TMP/card/Config/dsperate/nds-nostick.ini" "$TMP/dsperate.log" \
	|| fail "DSperate was not run with the game and the settings: [$(cat "$TMP/dsperate.log")]"
grep -q 'lefttrigger:b12,righttrigger:b13' "$TMP/dsperate.log" \
	|| fail "the RG35XX SP's triggers are not b12/b13: [$(grep map= "$TMP/dsperate.log")]"
grep -q 'rightx' "$TMP/dsperate.log" && fail "a pad with no sticks was given a right stick"
# by position: the bottom button (B, b4) is SDL's a, the right one (A, b3) its b
grep -q 'ANBERNIC-keys,a:b4,b:b3,x:b5,y:b6,' "$TMP/dsperate.log" \
	|| fail "the face buttons are not mapped by position: [$(grep map= "$TMP/dsperate.log")]"
grep -q '^map=19000000010000000100000000010000,' "$TMP/dsperate.log" \
	|| fail "SDL_GAMECONTROLLERCONFIG was not set"
grep -qx "lib=$TMP/card/System/dsperate/lib:/usr/lib:/usr/lib/aarch64-linux-gnu" "$TMP/dsperate.log" \
	|| fail "DSperate does not find the mali SDL2 first: [$(grep lib= "$TMP/dsperate.log")]"
grep -qx 'gov=performance' "$TMP/dsperate.log" || fail "DSperate did not run under performance"
grep -qx 'rotate=' "$TMP/dsperate.log" || fail "the RG35XX SP's panel was turned"
[ "$(cat "$TMP/cpu/scaling_governor")" = schedutil ] || fail "the governor was not given back"
grep -qx "saves = $TMP/card/Saves/NDS" "$TMP/card/Config/dsperate/nds-nostick.ini" \
	|| fail "the settings were not seeded with the card's paths"
grep -q '@SD@' "$TMP/card/Config/dsperate/nds-nostick.ini" && fail "a path was left unwritten"
[ -d "$TMP/card/States/NDS" ] || fail "States/NDS was not made"

# held A: DSperate's resume is not taken up
TMP="$TMP" CPUDIR="$TMP/cpu" OMEGA_ROOT="$TMP/card" AGS_LOADER=sh AGS_CPUFREQ="$TMP/cpu" \
	AGS_BASEOS_RELEASE="$TMP/release" sh "$TMP/card/System/nds.sh" "$rom" --clean || true
grep -qx "args=$rom --fullscreen --config $TMP/card/Config/dsperate/nds-nostick.ini --no-autoload" "$TMP/dsperate.log" \
	|| fail "a clean start still resumed: [$(grep args= "$TMP/dsperate.log")]"

# the player's settings are theirs once seeded
echo "# mine" >> "$TMP/card/Config/dsperate/nds-nostick.ini"
TMP="$TMP" CPUDIR="$TMP/cpu" OMEGA_ROOT="$TMP/card" AGS_LOADER=sh AGS_CPUFREQ="$TMP/cpu" \
	AGS_BASEOS_RELEASE="$TMP/release" sh "$TMP/card/System/nds.sh" "$rom" || true
grep -qx '# mine' "$TMP/card/Config/dsperate/nds-nostick.ini" || fail "the seeded settings were written over"

grep -qx 'stylus_dpad = +lefttrigger' "$TMP/card/Config/dsperate/nds-nostick.ini" \
	|| fail "the RG35XX SP's stylus is not on L2 and the d-pad"
grep -q '@PAD@' "$TMP/card/Config/dsperate/nds-nostick.ini" && fail "the pad's stylus was left unwritten"

# the RG CubeXX: two sticks, the right one the pen, its own settings beside the SP's
printf 'BASEOS_TARGET=rgcubexx\n' > "$TMP/release"
TMP="$TMP" CPUDIR="$TMP/cpu" OMEGA_ROOT="$TMP/card" AGS_LOADER=sh AGS_CPUFREQ="$TMP/cpu" \
	AGS_BASEOS_RELEASE="$TMP/release" sh "$TMP/card/System/nds.sh" "$rom" || true
cube="$TMP/card/Config/dsperate/nds-2stick.ini"
grep -qx "args=$rom --fullscreen --config $cube" "$TMP/dsperate.log" \
	|| fail "the CubeXX did not get settings of its own: [$(grep args= "$TMP/dsperate.log")]"
grep -q 'lefttrigger:b13,righttrigger:b14' "$TMP/dsperate.log" \
	|| fail "the CubeXX's triggers are not b13/b14: [$(grep map= "$TMP/dsperate.log")]"
grep -q 'rightx:a2,righty:a3,rightstick:b15' "$TMP/dsperate.log" \
	|| fail "the CubeXX's right stick is not mapped"
grep -qx 'stylus_axis = right' "$cube" || fail "the CubeXX's pen is not on the right stick"
grep -qx 'stick_dpad = left' "$cube" || fail "the CubeXX's left stick is not a d-pad"
grep -qx "saves = $TMP/card/Saves/NDS" "$cube" || fail "the CubeXX keeps its saves elsewhere"
grep -qx '# mine' "$TMP/card/Config/dsperate/nds-nostick.ini" || fail "the SP's settings were touched"
grep -qx 'rotate=' "$TMP/dsperate.log" || fail "the CubeXX's square panel was turned"

# one stick: the RG40XX V
printf 'BASEOS_TARGET=rg40xxv\n' > "$TMP/release"
TMP="$TMP" CPUDIR="$TMP/cpu" OMEGA_ROOT="$TMP/card" AGS_LOADER=sh AGS_CPUFREQ="$TMP/cpu" \
	AGS_BASEOS_RELEASE="$TMP/release" sh "$TMP/card/System/nds.sh" "$rom" || true
grep -qx 'stylus_axis = left' "$TMP/card/Config/dsperate/nds-1stick.ini" \
	|| fail "the RG40XX V's pen is not on its stick"
grep -q 'rightx' "$TMP/dsperate.log" && fail "a pad with one stick was given a right stick"

# the RG28XX's panel stands on its side
printf 'BASEOS_TARGET="rg28xx"\n' > "$TMP/release"
TMP="$TMP" CPUDIR="$TMP/cpu" OMEGA_ROOT="$TMP/card" AGS_LOADER=sh AGS_CPUFREQ="$TMP/cpu" \
	AGS_BASEOS_RELEASE="$TMP/release" sh "$TMP/card/System/nds.sh" "$rom" || true
grep -qx 'rotate=270' "$TMP/dsperate.log" || fail "the RG28XX's panel was not turned"

# a game that is not there runs nothing
rm -f "$TMP/dsperate.log"
status=0
OMEGA_ROOT="$TMP/card" AGS_LOADER=sh sh "$TMP/card/System/nds.sh" "$TMP/none.nds" >/dev/null || status=$?
[ "$status" = 2 ] || fail "a missing game did not fail: $status"
[ ! -f "$TMP/dsperate.log" ] || fail "DSperate ran with no game"

echo "test-handoff: ok"
