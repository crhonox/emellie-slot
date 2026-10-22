#!/bin/sh
# lid.sh, off-device: the hall sensor, the RTC, /sys/power/state and poweroff are files and a
# stand-in that note how they were used; omega-zero is a stand-in that notes its SIGTERM.
set -eu

HERE="$(cd "$(dirname "$0")" && pwd)"
TMP="$(mktemp -d)"
trap 'pkill -f "$TMP/card/System/" 2>/dev/null; rm -rf "$TMP"' EXIT HUP INT TERM

fail() {
	echo "$*" >&2
	echo "--- session log" >&2
	cat "$TMP/card/emellie-slot-session.log" >&2 2>/dev/null || true
	exit 1
}

# waits up to 5 s for a condition
until_true() {
	i=0
	while [ "$i" -lt 50 ]; do
		eval "$1" && return 0
		sleep 0.1
		i=$((i + 1))
	done
	return 1
}

setup() {
	pkill -f "$TMP/card/System/" 2>/dev/null || true
	rm -rf "$TMP/card" "$TMP/run" "$TMP/sys" "$TMP"/*.log "$TMP/term"
	mkdir -p "$TMP/card/System" "$TMP/run" "$TMP/sys/rtc/rtc0" "$TMP/sys/rtc/rtc1"
	cp "$HERE/System/lid.sh" "$TMP/card/System/"
	echo 1 > "$TMP/sys/hallkey"
	: > "$TMP/sys/state"
	echo rtc-other > "$TMP/sys/rtc/rtc0/name"
	: > "$TMP/sys/rtc/rtc0/wakealarm"
	echo sunxi-rtc > "$TMP/sys/rtc/rtc1/name"
	: > "$TMP/sys/rtc/rtc1/wakealarm"
	printf '#!/bin/sh\necho off >> "%s/poweroff.log"\n' "$TMP" > "$TMP/poweroff"
	# omega-zero: notes a SIGTERM and leaves
	printf 'trap "echo term > %s/term; exit 0" TERM\nwhile :; do sleep 0.1; done\n' "$TMP" \
		> "$TMP/card/System/emellie-slot"
	sh "$TMP/card/System/emellie-slot" &
	OMEGA_ROOT="$TMP/card" AGS_RUN="$TMP/run" AGS_HALL="$TMP/sys/hallkey" AGS_RTC="$TMP/sys/rtc" \
		AGS_POWER_STATE="$TMP/sys/state" AGS_POWEROFF="sh $TMP/poweroff" AGS_LID_SETTLE_S=0 \
		AGS_LID_POLL_S=0.1 LID_OFF_S="$1" sh "$TMP/card/System/lid.sh" &
	sleep 0.3
}

# --- shut, then opened: the kernel slept, the alarm on the SoC's RTC, nothing switched off
setup 900
echo 0 > "$TMP/sys/hallkey"
until_true 'grep -q mem "$TMP/sys/state"' || fail "a shut lid did not sleep the kernel"
grep -qx '+900' "$TMP/sys/rtc/rtc1/wakealarm" || fail "the sunxi-rtc alarm was not set: [$(cat "$TMP/sys/rtc/rtc1/wakealarm")]"
[ -s "$TMP/sys/rtc/rtc0/wakealarm" ] && fail "the other RTC's alarm was set"
echo 1 > "$TMP/sys/hallkey"
until_true 'grep -q "lid: open" "$TMP/card/emellie-slot-session.log"' || fail "the open lid was not seen"
grep -qx 0 "$TMP/sys/rtc/rtc1/wakealarm" || fail "the alarm was left set with the lid open"
[ -f "$TMP/poweroff.log" ] && fail "an opened lid switched the handheld off"
[ -f "$TMP/term" ] && fail "an opened lid stopped the game"

# --- shut until the alarm comes: the game left, then the handheld off
setup 900
echo 0 > "$TMP/sys/hallkey"
until_true 'grep -q "+900" "$TMP/sys/rtc/rtc1/wakealarm"' || fail "the alarm was not set"
# the alarm firing clears itself
: > "$TMP/sys/rtc/rtc1/wakealarm"
until_true '[ -f "$TMP/poweroff.log" ]' || fail "the alarm did not switch the handheld off"
[ -f "$TMP/term" ] || fail "the game was not told to leave before the handheld went off"
[ -f "$TMP/run/omega-lid-off" ] || fail "launch_frontend.sh was not told the handheld is going off"

# --- a wake with the lid still shut and no alarm yet (the power key) sleeps again
setup 900
echo 0 > "$TMP/sys/hallkey"
until_true 'grep -q mem "$TMP/sys/state"' || fail "a shut lid did not sleep the kernel"
: > "$TMP/sys/state"
until_true 'grep -q mem "$TMP/sys/state"' || fail "a wake with the lid shut did not sleep again"
[ -f "$TMP/poweroff.log" ] && fail "a wake before the alarm switched the handheld off"

# --- a lid shut when the watchdog starts is not a close
pkill -f "$TMP/card/System/" 2>/dev/null || true
rm -rf "$TMP/card" "$TMP/sys"
mkdir -p "$TMP/card/System" "$TMP/sys/rtc"
cp "$HERE/System/lid.sh" "$TMP/card/System/"
echo 0 > "$TMP/sys/hallkey"
: > "$TMP/sys/state"
OMEGA_ROOT="$TMP/card" AGS_RUN="$TMP/run" AGS_HALL="$TMP/sys/hallkey" AGS_RTC="$TMP/sys/rtc" \
	AGS_POWER_STATE="$TMP/sys/state" AGS_POWEROFF=true AGS_LID_SETTLE_S=0 AGS_LID_POLL_S=0.1 \
	sh "$TMP/card/System/lid.sh" &
sleep 0.5
[ -s "$TMP/sys/state" ] && fail "a lid shut at boot slept the handheld"

# --- no hall sensor: the watchdog says so and leaves
rm -f "$TMP/sys/hallkey"
OMEGA_ROOT="$TMP/card" AGS_HALL="$TMP/sys/hallkey" sh "$TMP/card/System/lid.sh" && fail "lid.sh ran with no sensor"
grep -q "no hall sensor" "$TMP/card/emellie-slot-session.log" || fail "no sensor went unsaid"

echo "test-lid: ok"
