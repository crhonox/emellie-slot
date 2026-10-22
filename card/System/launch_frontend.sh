#!/bin/sh
SD="${OMEGA_ROOT:-/mnt/sdcard}"
SYS="$SD/System"
RUN="${AGS_RUN:-/run}"
LOADER="${AGS_LOADER:-/lib/ld-linux-aarch64.so.1}"
POWEROFF="${AGS_POWEROFF:-poweroff}"
export OMEGA_ROOT="$SD"
# every thread allocates from the one heap: glibc's arenas, 64 MB of address space reserved
# for each thread that contends, crowded the cores loaded after them, and gpsp's JIT cache has
# to be mapped within 128 MB of its code
export MALLOC_ARENA_MAX=1

log() {
	echo "$(date '+%H:%M:%S') $*" >> "$SD/emellie-slot-session.log" 2>/dev/null
}

# The handheld going down starts nothing again: lid.sh, shut too long, switches it off itself
# (omega-lid-off); POWER held in DSperate leaves it to be switched off here (omega-power-off).
going_off() {
	if [ -f "$RUN/omega-power-off" ]; then
		rm -f "$RUN/omega-power-off"
		log "power held, switching off"
		sync
		$POWEROFF
		exit 0
	fi
	if [ -f "$RUN/omega-lid-off" ]; then
		log "lid shut, switched off by lid.sh"
		sync
		exit 0
	fi
}

pid="$(cat "$RUN/omega-led.pid" 2>/dev/null)"
if [ -z "$pid" ] || [ ! -d "/proc/$pid" ]; then
	sh "$SYS/led.sh" once
	sh "$SYS/led.sh" loop > /dev/null 2>&1 &
	echo $! > "$RUN/omega-led.pid"
fi

# The lid sleeps the handheld, and switches it off shut LID_OFF_S, whatever is running (lid.sh)
rm -f "$RUN/omega-lid-off" "$RUN/omega-power-off"
pid="$(cat "$RUN/omega-lid.pid" 2>/dev/null)"
if [ -z "$pid" ] || [ ! -d "/proc/$pid" ]; then
	sh "$SYS/lid.sh" > /dev/null 2>&1 &
	echo $! > "$RUN/omega-lid.pid"
fi

echo "|/bin/sh $SYS/coresave.sh %p %s %e %t" > /proc/sys/kernel/core_pattern 2>/dev/null

if [ -f "$SD/RUN_PROBE" ]; then
	rm -f "$SD/RUN_PROBE"
	log "probe $(AGS_PROBE_NET_WAIT_S=15 sh "$SYS/probe.sh" 2>&1)"
fi

[ -f "$SD/emellie-slot.log" ] && mv -f "$SD/emellie-slot.log" "$SD/emellie-slot.log.1"
: > "$SD/emellie-slot.log"

# A game whose system has an emulator of its own (the DS, in DSperate; a port, in ports.sh) is
# handed over: omega-zero writes what to run here and leaves, letting go of the panel, the sound
# card and the buttons; the emulator runs, and omega-zero starts again with OMEGA_RETURNED set, the game on its shelf.
# One left from a boot the handheld went down in is not acted on: omega-zero hands that game
# over again itself, as the one it was playing.
export OMEGA_HANDOFF="$RUN/omega-handoff"
rm -f "$OMEGA_HANDOFF"

returned=0
while :; do
	log "start $SYS/emellie-slot returned=$returned"
	OMEGA_RETURNED=$returned $LOADER "$SYS/emellie-slot" >> "$SD/emellie-slot.log" 2>&1
	status=$?
	going_off
	if [ ! -f "$OMEGA_HANDOFF" ]; then
		log "emellie-slot left, status $status"
		exit "$status"
	fi
	platform="$(sed -n 's/^platform=//p' "$OMEGA_HANDOFF")"
	rom="$(sed -n 's/^rom=//p' "$OMEGA_HANDOFF")"
	clean=
	[ "$(sed -n 's/^clean=//p' "$OMEGA_HANDOFF")" = 1 ] && clean=--clean
	rm -f "$OMEGA_HANDOFF"
	case "$platform" in
	NDS)
		log "handoff: dsperate $rom $clean"
		sh "$SYS/nds.sh" "$rom" $clean >> "$SD/emellie-slot.log" 2>&1
		log "handoff: dsperate left, status $?"
		;;
	PORTS)
		log "handoff: port $rom"
		sh "$SYS/ports.sh" "$rom" >> "$SD/emellie-slot.log" 2>&1
		log "handoff: port left, status $?"
		;;
	*) log "handoff: nothing here runs $platform" ;;
	esac
	sync
	going_off
	returned=1
done
