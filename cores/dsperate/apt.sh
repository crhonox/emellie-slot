#!/bin/sh
# Installs the Debian packages named, those not there already, in the bullseye box the device
# build runs in. Bullseye has left the Debian mirrors, so apt is pointed at the snapshot of it
# the image names (commented out in its sources.list) whenever the mirrors refuse it.
set -eu

missing=
for pkg in "$@"; do
	dpkg -s "$pkg" >/dev/null 2>&1 || missing="$missing $pkg"
done
[ -z "$missing" ] && exit 0

if [ "$(id -u)" != 0 ]; then
	echo "apt.sh: needs$missing installed, and is not root to install them" >&2
	exit 1
fi

list=/etc/apt/sources.list
if ! apt-get update -qq 2>/dev/null || ! apt-get install -y -qq --no-install-recommends $missing >/dev/null 2>&1; then
	if grep -q '^# deb http://snapshot.debian.org' "$list" 2>/dev/null; then
		sed -i -e 's|^deb |# &|' -e 's|^# deb http://snapshot|deb [check-valid-until=no] http://snapshot|' "$list"
	else
		sed -i -e 's|deb.debian.org/debian-security|archive.debian.org/debian-security|' \
			-e 's|deb.debian.org/debian|archive.debian.org/debian|' "$list"
	fi
	apt-get -o Acquire::Check-Valid-Until=false update -qq
	DEBIAN_FRONTEND=noninteractive apt-get install -y -qq --no-install-recommends $missing >/dev/null
fi
