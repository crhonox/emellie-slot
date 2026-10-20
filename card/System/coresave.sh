#!/bin/sh
SD="${AGS_CARD:-/mnt/sdcard}"
MOUNTPOINT="${AGS_MOUNTPOINT:-mountpoint}"
DIR="$SD/crash"
# room a core must leave on the card, in KB, for the saves and the config written after it
MIN_FREE_KB="${AGS_CORE_MIN_FREE_KB:-524288}"
# a core kept less than this many minutes ago is kept rather than written over: a crash that
# comes back at every start would otherwise write one at every start
KEEP_MIN=10

if ! $MOUNTPOINT -q "$SD" 2>/dev/null; then
	cat > /dev/null
	exit 0
fi

exe="${AGS_CORE_EXE:-$(readlink "/proc/$1/exe" 2>/dev/null)}"
# launch_frontend.sh runs omega-zero through the dynamic loader, which is then the process's exe,
# omega-zero the first thing on its command line after it
prog="${AGS_CORE_PROG-$(tr '\0' '\n' < "/proc/$1/cmdline" 2>/dev/null | sed -n 2p)}"
mkdir -p "$DIR" 2>/dev/null

case "$exe:$prog" in
*/emellie-slot:* | */ld-linux*:*/emellie-slot) ours=1 ;;
*) ours= ;;
esac

core=
if [ -n "$ours" ]; then
	free="$(df -kP "$SD" 2>/dev/null | awk 'NR == 2 { print $4 }')"
	if [ -n "$(find "$DIR" -maxdepth 1 -name 'emellie-slot.core*' -mmin -$KEEP_MIN 2>/dev/null)" ]; then
		core=" core=kept-earlier"
	elif [ -z "$free" ] || [ "$free" -lt "$MIN_FREE_KB" ]; then
		core=" core=no-room"
	else
		core=" core=saved"
	fi
fi
echo "$(date '+%F %T') pid=$1 signal=$2 thread=$3 exe=$exe${prog:+ prog=$prog}$core" >> "$DIR/cores.log"

if [ -n "$ours" ]; then
	# what it said before it went down, which the next start of it writes over
	cp "$SD/emellie-slot.log" "$DIR/emellie-slot.log" 2>/dev/null
	cp "/proc/$1/maps" "$DIR/emellie-slot.maps" 2>/dev/null
fi

# omega-zero's whole memory as it went down. Most of it is room it took and never wrote, which
# gzip packs to next to nothing, as fast as the card takes the rest: omega-zero starts again
# once all of it has been read.
if [ "$core" = " core=saved" ]; then
	if command -v gzip > /dev/null 2>&1; then
		name=emellie-slot.core.gz
		gzip -1 > "$DIR/$name.tmp"
	else
		name=emellie-slot.core
		cat > "$DIR/$name.tmp"
	fi
	if [ $? -eq 0 ]; then
		rm -f "$DIR/emellie-slot.core" "$DIR/emellie-slot.core.gz"
		mv -f "$DIR/$name.tmp" "$DIR/$name"
	else
		rm -f "$DIR/$name.tmp"
		cat > /dev/null
	fi
else
	cat > /dev/null
fi
sync
