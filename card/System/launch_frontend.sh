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

# The handheld going down starts nothing again: POWER held in DSperate leaves it to be switched
# off here (omega-power-off).
going_off() {
	if [ -f "$RUN/omega-power-off" ]; then
		rm -f "$RUN/omega-power-off"
		log "power held, switching off"
		sync
		$POWEROFF
		exit 0
	fi
}

# An update over the air: Emellie Slot unpacks a release's System/ into .update/System and writes
# .update/ready, then leaves. Here the two folders are swapped, the old one kept as System.old,
# and the new launch_frontend.sh started in place of this one. Its first start is a trial: one
# that cannot run or falls over within TRIAL_S goes back to System.old.
UPDATE="$SD/.update"
TRIAL="$SD/.update-trial"
TRIAL_S="${AGS_UPDATE_TRIAL_S:-30}"

# The old System's LED watchdog, for the new one's to start in its place
stop_watchdogs() {
	pid="$(cat "$RUN/omega-led.pid" 2>/dev/null)"
	[ -n "$pid" ] && kill "$pid" 2>/dev/null
	rm -f "$RUN/omega-led.pid"
}

apply_update() {
	[ -f "$UPDATE/ready" ] || return 0
	if [ ! -f "$UPDATE/System/emellie-slot" ] || [ ! -f "$UPDATE/System/launch_frontend.sh" ]; then
		log "update: .update holds no whole System, dropped"
		rm -rf "$UPDATE"
		return 0
	fi
	tag="$(head -n 1 "$UPDATE/ready")"
	log "update: putting $tag in place"
	rm -rf "$SD/System.old"
	stop_watchdogs
	if ! mv "$SYS" "$SD/System.old"; then
		log "update: could not move System aside, staying"
		return 0
	fi
	if ! mv "$UPDATE/System" "$SYS"; then
		mv "$SD/System.old" "$SYS"
		log "update: could not move the new System in, staying"
		return 0
	fi
	rm -rf "$UPDATE"
	echo "$tag" > "$TRIAL"
	sync
	exec sh "$SYS/launch_frontend.sh"
}

# A trial still running TRIAL_S in is kept there and then: the handheld switched off from the
# menu goes down from inside emellie-slot, and this script never sees it leave to judge it
keep_trial_later() {
	[ -f "$TRIAL" ] || return 0
	(
		sleep "$TRIAL_S"
		[ -f "$TRIAL" ] || exit 0
		log "update: $(cat "$TRIAL") ran ${TRIAL_S}s, kept"
		rm -f "$TRIAL"
		sync
	) > /dev/null 2>&1 &
}

# The first start after an update: one that could not run (126, 127) or was killed (128 and up)
# within TRIAL_S puts System.old back and starts it again
judge_trial() {
	[ -f "$TRIAL" ] || return 0
	status="$1" took="$2"
	if [ "$status" -lt 126 ] || [ "$took" -ge "$TRIAL_S" ] || [ ! -d "$SD/System.old" ]; then
		log "update: $(cat "$TRIAL") started, kept"
		rm -f "$TRIAL"
		return 0
	fi
	log "update: $(cat "$TRIAL") fell over (status $status after ${took}s), back to the one before"
	stop_watchdogs
	rm -rf "$SD/System.failed"
	mv "$SYS" "$SD/System.failed" && mv "$SD/System.old" "$SYS" || return 0
	rm -f "$TRIAL"
	sync
	exec sh "$SYS/launch_frontend.sh"
}

apply_update

pid="$(cat "$RUN/omega-led.pid" 2>/dev/null)"
if [ -z "$pid" ] || [ ! -d "/proc/$pid" ]; then
	sh "$SYS/led.sh" once
	sh "$SYS/led.sh" loop > /dev/null 2>&1 &
	echo $! > "$RUN/omega-led.pid"
fi

rm -f "$RUN/omega-power-off"

echo "|/bin/sh $SYS/coresave.sh %p %s %e %t" > /proc/sys/kernel/core_pattern 2>/dev/null

if [ -f "$SD/RUN_PROBE" ]; then
	rm -f "$SD/RUN_PROBE"
	log "probe $(AGS_PROBE_NET_WAIT_S=15 sh "$SYS/probe.sh" 2>&1)"
fi

[ -f "$SD/emellie-slot.log" ] && mv -f "$SD/emellie-slot.log" "$SD/emellie-slot.log.1"
: > "$SD/emellie-slot.log"

# A game whose system has an emulator of its own (the DS, in DSperate; a port, in ports.sh; a
# PICO-8 cart, in the player's own PICO-8, pico8.sh) is handed over: omega-zero writes what to run here and leaves, letting go of the panel, the sound
# card and the buttons; the emulator runs, and omega-zero starts again with OMEGA_RETURNED set, the game on its shelf.
# One left from a boot the handheld went down in is not acted on: omega-zero hands that game
# over again itself, as the one it was playing.
export OMEGA_HANDOFF="$RUN/omega-handoff"
rm -f "$OMEGA_HANDOFF"

keep_trial_later

returned=0
while :; do
	log "start $SYS/emellie-slot returned=$returned"
	began="$(date +%s)"
	OMEGA_RETURNED=$returned $LOADER "$SYS/emellie-slot" >> "$SD/emellie-slot.log" 2>&1
	status=$?
	going_off
	judge_trial "$status" $(($(date +%s) - began))
	apply_update
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
	PICO8)
		log "handoff: pico-8 $rom"
		sh "$SYS/pico8.sh" "$rom" >> "$SD/emellie-slot.log" 2>&1
		log "handoff: pico-8 left, status $?"
		;;
	*) log "handoff: nothing here runs $platform" ;;
	esac
	sync
	going_off
	returned=1
done
