#!/bin/sh
# Steps the panel's light up or down while an emulator of its own has the handheld (DSperate,
# from MENU+VOL), in the frontend's steps: brightness= in its state file, 1 to 9 here (0, which
# the frontend allows, is a dark panel nobody could step back from), 9 the light's whole range.
# The step is written back, so the frontend comes back at it.
SD="${OMEGA_ROOT:-/mnt/sdcard}"
STATE="${AGS_STATE:-$SD/Config/emellie-slot.state}"
SYSFS="${AGS_SYSFS:-/sys}"
TOP=9

step="$(sed -n 's/^brightness=//p' "$STATE" 2>/dev/null)"
case "$step" in '' | *[!0-9]*) step=5 ;; esac
case "$1" in
up) step=$((step + 1)) ;;
down) step=$((step - 1)) ;;
*)
	echo "usage: $0 up|down" >&2
	exit 2
	;;
esac
[ "$step" -gt "$TOP" ] && step=$TOP
[ "$step" -lt 1 ] && step=1

dbg="$SYSFS/kernel/debug/dispdbg"
if [ -e "$dbg/param" ]; then
	echo lcd0 > "$dbg/name"
	echo setbl > "$dbg/command"
	echo $((255 * step / TOP)) > "$dbg/param"
	echo 1 > "$dbg/start"
else
	for dir in "$SYSFS"/class/backlight/*; do
		[ -e "$dir/brightness" ] || continue
		max="$(cat "$dir/max_brightness" 2>/dev/null)"
		echo $((${max:-255} * step / TOP)) > "$dir/brightness"
		break
	done
fi

if [ -f "$STATE" ]; then
	sed "s/^brightness=.*/brightness=$step/" "$STATE" > "$STATE.tmp" && mv -f "$STATE.tmp" "$STATE"
fi
