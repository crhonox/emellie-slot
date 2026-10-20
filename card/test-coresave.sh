#!/bin/sh
set -eu

HERE="$(cd "$(dirname "$0")" && pwd)"
SCRIPT="$HERE/System/coresave.sh"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT HUP INT TERM
LOADER=/usr/lib/aarch64-linux-gnu/ld-linux-aarch64.so.1

setup() {
	rm -rf "$TMP/card" "$TMP/mountpoint"
	mkdir -p "$TMP/card"
	cat > "$TMP/mountpoint" <<-SH
		#!/bin/sh
		exit ${1:-0}
	SH
	chmod +x "$TMP/mountpoint"
}

run() {
	printf '%s' "$2" | AGS_CARD="$TMP/card" AGS_MOUNTPOINT="$TMP/mountpoint" \
		AGS_CORE_EXE="$1" AGS_CORE_PROG="${6:-}" AGS_CORE_MIN_FREE_KB="${MIN_FREE_KB:-0}" \
		sh "$SCRIPT" "${3:-4321}" "${4:-11}" "${5:-omega-emu}" 0
}

kept() {
	gzip -dc "$TMP/card/crash/emellie-slot.core.gz"
}

# the core kept as one saved long before the last ten minutes
age() {
	touch -t 202601010000 "$TMP/card/crash/emellie-slot.core.gz"
}

setup
run /usr/bin/busybox "not a frontend core"
grep -q 'signal=11 thread=omega-emu exe=/usr/bin/busybox$' "$TMP/card/crash/cores.log" \
	|| { echo "no log line for a non-frontend crash: [$(cat "$TMP/card/crash/cores.log")]" >&2; exit 1; }
[ -z "$(ls "$TMP/card/crash" | grep core\\. || true)" ] \
	|| { echo "kept a core for a process that is not the frontend" >&2; exit 1; }

setup
run /mnt/sdcard/System/emellie-slot "first core"
[ "$(kept)" = "first core" ] \
	|| { echo "the frontend's core was not saved" >&2; exit 1; }
grep -q 'exe=/mnt/sdcard/System/emellie-slot core=saved$' "$TMP/card/crash/cores.log" \
	|| { echo "cores.log does not say the core was saved: [$(cat "$TMP/card/crash/cores.log")]" >&2; exit 1; }

run /mnt/sdcard/System/emellie-slot "second core"
[ "$(kept)" = "first core" ] \
	|| { echo "a crash minutes after the last wrote over its core" >&2; exit 1; }
grep -q 'core=kept-earlier$' "$TMP/card/crash/cores.log" \
	|| { echo "cores.log does not say the earlier core was kept" >&2; exit 1; }

age
run /mnt/sdcard/System/emellie-slot "third core"
[ "$(kept)" = "third core" ] \
	|| { echo "a crash after the ten minutes did not replace the core" >&2; exit 1; }
[ ! -f "$TMP/card/crash/emellie-slot.core.gz.tmp" ] \
	|| { echo "left a half-written core behind" >&2; exit 1; }
[ "$(grep -c . "$TMP/card/crash/cores.log")" = "3" ] \
	|| { echo "cores.log did not record all three crashes" >&2; exit 1; }

setup
echo "fatal runtime error: Rust cannot catch foreign exceptions, aborting" > "$TMP/card/emellie-slot.log"
run "$LOADER" "core through the loader" 4321 6 omega-emu /mnt/sdcard/System/emellie-slot
[ "$(kept)" = "core through the loader" ] \
	|| { echo "the core of omega-zero run through the loader was not saved" >&2; exit 1; }
grep -q 'cannot catch foreign exceptions' "$TMP/card/crash/emellie-slot.log" \
	|| { echo "the log of the run that went down was not kept" >&2; exit 1; }
grep -q "signal=6 thread=omega-emu exe=$LOADER prog=/mnt/sdcard/System/emellie-slot core=saved" \
	"$TMP/card/crash/cores.log" \
	|| { echo "cores.log does not name the program: [$(cat "$TMP/card/crash/cores.log")]" >&2; exit 1; }

setup
echo "omega-zero is fine" > "$TMP/card/emellie-slot.log"
run "$LOADER" "another program" 4321 11 worker /usr/bin/other
[ ! -f "$TMP/card/crash/emellie-slot.log" ] \
	|| { echo "kept omega-zero's log for another program run through the loader" >&2; exit 1; }
[ ! -f "$TMP/card/crash/emellie-slot.core.gz" ] \
	|| { echo "kept a core for another program run through the loader" >&2; exit 1; }

setup
MIN_FREE_KB=999999999999 run /mnt/sdcard/System/emellie-slot "core with no room for it"
[ ! -f "$TMP/card/crash/emellie-slot.core.gz" ] \
	|| { echo "wrote a core to a card with no room for it" >&2; exit 1; }
grep -q 'core=no-room$' "$TMP/card/crash/cores.log" \
	|| { echo "cores.log does not say there was no room: [$(cat "$TMP/card/crash/cores.log")]" >&2; exit 1; }

setup
cat > "$TMP/mountpoint" <<-SH
	#!/bin/sh
	exit 1
SH
chmod +x "$TMP/mountpoint"
run /mnt/sdcard/System/emellie-slot "core with nowhere to go" \
	|| { echo "a missing card made the helper fail" >&2; exit 1; }
[ ! -d "$TMP/card/crash" ] \
	|| { echo "wrote to a card that is not mounted" >&2; exit 1; }

echo "coresave: all passed"
