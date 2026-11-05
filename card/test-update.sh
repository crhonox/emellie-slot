#!/bin/sh
# launch_frontend.sh putting an update in place, off-device: omega-zero is a stand-in that, the
# first time it runs, leaves an unpacked release in .update/ as the Update screen does, and the
# release's own omega-zero a stand-in that notes it ran and leaves as it is told to.
set -eu

HERE="$(cd "$(dirname "$0")" && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT HUP INT TERM

fail() {
	echo "$*" >&2
	echo "--- session log" >&2
	cat "$TMP/card/emellie-slot-session.log" >&2 2>/dev/null || true
	exit 1
}

# a System/ whose omega-zero notes "NAME", runs THEN, and leaves with STATUS; the first one also
# unpacks the update the first time it runs, when told to
system() {
	dir="$1" name="$2" status="$3" then="${4:-}"
	mkdir -p "$dir"
	cp "$HERE/System/launch_frontend.sh" "$dir/"
	printf '#!/bin/sh\nexit 0\n' > "$dir/led.sh"
	cat > "$dir/emellie-slot" <<-SH
		#!/bin/sh
		echo "$name" >> "\$TMP/ran.log"
		if [ -f "\$TMP/unpack" ]; then
			rm -f "\$TMP/unpack"
			cp -R "\$TMP/release" "\$OMEGA_ROOT/.update"
		fi
		$then
		exit $status
	SH
}

setup() {
	rm -rf "$TMP/card" "$TMP/run" "$TMP/release" "$TMP/ran.log" "$TMP/unpack"
	mkdir -p "$TMP/card" "$TMP/run" "$TMP/release"
	system "$TMP/card/System" old 0
	echo saved > "$TMP/card/save.srm"
}

# the release Emellie Slot unpacks: its System/ and the ready file naming it
release() {
	system "$TMP/release/System" new "$1" "${2:-}"
	echo es-9.9.9 > "$TMP/release/ready"
}

launch() {
	TMP="$TMP" OMEGA_ROOT="$TMP/card" AGS_RUN="$TMP/run" AGS_LOADER=sh AGS_POWEROFF=true \
		AGS_UPDATE_TRIAL_S="${TRIAL_S:-30}" sh "$TMP/card/System/launch_frontend.sh" || true
}

ran() {
	tr '\n' ' ' < "$TMP/ran.log" | sed 's/ $//'
}

# --- an update unpacked: the old leaves, the new takes its place and starts, the old kept aside
setup
release 0
touch "$TMP/unpack"
launch
[ "$(ran)" = "old new" ] || fail "the new omega-zero did not start after the old: [$(ran)]"
grep -q '^echo "new"' "$TMP/card/System/emellie-slot" || fail "System/ is not the update's"
grep -q '^echo "old"' "$TMP/card/System.old/emellie-slot" || fail "the old System/ was not kept as System.old"
[ -e "$TMP/card/.update" ] && fail ".update/ was left behind"
[ -e "$TMP/card/.update-trial" ] && fail "a start that went well left the trial standing"
[ "$(cat "$TMP/card/save.srm")" = saved ] || fail "something outside System/ was touched"

# --- the new one falls over at once: back to the old, the new kept aside as System.failed
setup
release 139
touch "$TMP/unpack"
launch
[ "$(ran)" = "old new old" ] || fail "a new omega-zero that fell over did not give way to the old: [$(ran)]"
grep -q '^echo "old"' "$TMP/card/System/emellie-slot" || fail "System/ was not put back"
grep -q '^echo "new"' "$TMP/card/System.failed/emellie-slot" || fail "the failed update was not kept aside"
[ -e "$TMP/card/.update-trial" ] && fail "the trial was left standing after going back"

# --- the new one leaves with a status of its own: that is no fall, it stays
setup
release 1
touch "$TMP/unpack"
launch
[ "$(ran)" = "old new" ] || fail "an ordinary leave was taken for a fall: [$(ran)]"
grep -q '^echo "new"' "$TMP/card/System/emellie-slot" || fail "an ordinary leave put the old System/ back"

# --- the new one runs past the trial and the handheld is switched off from inside it, the
# script never seeing it leave: the trial is kept all the same
setup
release 0 'sleep 2; kill -9 $PPID'
touch "$TMP/unpack"
TRIAL_S=1 launch
[ "$(ran)" = "old new" ] || fail "the new omega-zero did not start after the old: [$(ran)]"
[ -e "$TMP/card/.update-trial" ] && fail "a trial switched off from inside was left standing"
grep -q "ran 1s, kept" "$TMP/card/emellie-slot-session.log" || fail "keeping the trial went unsaid"
grep -q '^echo "new"' "$TMP/card/System/emellie-slot" || fail "the update was not kept"

# --- unpacked, then the handheld went off before it was put in: the next boot puts it in first
setup
release 0
cp -R "$TMP/release" "$TMP/card/.update"
launch
[ "$(ran)" = "new" ] || fail "an update waiting at boot was not put in before anything ran: [$(ran)]"

# --- .update/ without a whole System/: dropped, the old runs
setup
release 0
rm "$TMP/release/System/emellie-slot"
cp -R "$TMP/release" "$TMP/card/.update"
launch
[ "$(ran)" = "old" ] || fail "a half update was run: [$(ran)]"
[ -e "$TMP/card/.update" ] && fail "a half update was left in .update/"
grep -q "no whole System" "$TMP/card/emellie-slot-session.log" || fail "the half update went unsaid"

echo "test-update: ok"
