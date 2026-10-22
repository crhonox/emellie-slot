#!/bin/sh
# The lid, for every system: launch_frontend.sh runs this beside omega-zero and whatever it hands
# the handheld over to (DSperate, a port). As spruceOS's lid_watchdog.sh and sleep_helper.sh on
# the Anbernic XX line: the hall sensor is read from the battery's hallkey (1 open, 0 shut), and
# a lid shut suspends the kernel itself, CPU and panel asleep, with the RTC's wakealarm set for
# LID_OFF_S. The lid opened, all goes on where it was; the alarm come first, the game in play is
# left with its save and the handheld switched off. omega-zero hears the same lid on its own
# input node and dozes, its resume written, before the kernel sleeps; DSperate tells the DS's
# own lid, the game sleeping as on the console, and wakes it to the host's clock.
SD="${OMEGA_ROOT:-/mnt/sdcard}"
SYS="$SD/System"
RUN="${AGS_RUN:-/run}"
HALL="${AGS_HALL:-/sys/class/power_supply/axp2202-battery/hallkey}"
RTC="${AGS_RTC:-/sys/class/rtc}"
POWER_STATE="${AGS_POWER_STATE:-/sys/power/state}"
POWEROFF="${AGS_POWEROFF:-poweroff}"
# spruce's own default for "shutdown from sleep"
LID_OFF_S="${LID_OFF_S:-900}"
# how long the frontend has to doze, its resume on the card, before the kernel sleeps
SETTLE_S="${AGS_LID_SETTLE_S:-1}"
POLL_S="${AGS_LID_POLL_S:-0.5}"

log() {
	echo "$(date '+%H:%M:%S') lid: $*" >> "$SD/emellie-slot-session.log" 2>/dev/null
}

# What has the handheld, read from /proc rather than pgrep, which nothing says BaseOS has:
# omega-zero, or DSperate it handed over to.
game_pids() {
	for dir in /proc/[0-9]*; do
		case "$(tr '\0' ' ' 2>/dev/null < "$dir/cmdline")" in
		*"$SYS/emellie-slot "* | *"$SYS/dsperate/dsperate "*) echo "${dir#/proc/}" ;;
		esac
	done
}

lid_open() {
	head -c 1 "$HALL" 2>/dev/null || echo 1
}

# The SoC's own RTC (sunxi-rtc) wakes the handheld; else one whose wakeup is enabled.
wake_alarm() {
	fallback=
	for rtc in "$RTC"/rtc*; do
		[ -e "$rtc/wakealarm" ] || continue
		if [ "$(cat "$rtc/name" 2>/dev/null)" = sunxi-rtc ]; then
			echo "$rtc/wakealarm"
			return
		fi
		[ -z "$fallback" ] && [ "$(cat "$rtc/device/power/wakeup" 2>/dev/null)" = enabled ] &&
			fallback="$rtc/wakealarm"
	done
	echo "${fallback:-$RTC/rtc0/wakealarm}"
}

# The game in play left with its save, then the handheld off. DSperate saves its state on
# SIGTERM as on its own quit; omega-zero, dozing, has written its resume already. lid-off stops
# launch_frontend.sh starting omega-zero again in between.
switch_off() {
	log "shut ${LID_OFF_S}s, switching off"
	touch "$RUN/omega-lid-off"
	pids="$(game_pids)"
	[ -n "$pids" ] && kill -TERM $pids 2>/dev/null
	i=0
	while [ "$i" -lt 20 ] && [ -n "$(game_pids)" ]; do
		sleep 0.5
		i=$((i + 1))
	done
	sync
	$POWEROFF
	exit 0
}

# Suspended until the lid opens or the alarm switches the handheld off; a wake with the lid still
# shut and the alarm not yet come (the power key) goes back to sleep.
sleep_shut() {
	alarm="$(wake_alarm)"
	armed=
	if [ -e "$alarm" ] && echo 0 > "$alarm" 2>/dev/null && echo "+$LID_OFF_S" > "$alarm" 2>/dev/null; then
		armed=1
	else
		log "no wakealarm at $alarm: the handheld sleeps until the lid opens"
	fi
	start="$(date +%s)"
	sleep "$SETTLE_S"
	log "shut, sleeping"
	while :; do
		[ "$(lid_open)" = 1 ] && break
		echo -n mem > "$POWER_STATE" 2>/dev/null || log "the kernel would not sleep"
		[ "$(lid_open)" = 1 ] && break
		elapsed=$(($(date +%s) - start))
		# the alarm cleared itself firing, or the clock says it is time (the two RTCs drift)
		if { [ -n "$armed" ] && [ -z "$(cat "$alarm" 2>/dev/null)" ]; } || [ "$elapsed" -ge $((LID_OFF_S - 10)) ]; then
			switch_off
		fi
		sleep "$POLL_S"
	done
	[ -n "$armed" ] && echo 0 > "$alarm" 2>/dev/null
	log "open"
}

for _ in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25; do
	[ -e "$HALL" ] && break
	sleep 0.2
done
if [ ! -e "$HALL" ]; then
	log "no hall sensor at $HALL, nothing watches the lid"
	exit 1
fi

# A lid already shut when this starts is not a close; the next one is.
handled=0
[ "$(lid_open)" = 0 ] && handled=1
while :; do
	if [ "$(lid_open)" = 0 ]; then
		if [ "$handled" = 0 ]; then
			handled=1
			sleep_shut
		fi
	else
		handled=0
	fi
	sleep "$POLL_S"
done
