#!/bin/sh
# Installs the Debian packages named, those not there already, in the bullseye box the device
# build runs in. Bullseye has left the Debian mirrors, so apt is pointed at archive.debian.org,
# security included: the image ships packages from bullseye-security, and without it their -dev
# packages (libudev-dev under libsdl2-dev) cannot be installed.
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

cat >/etc/apt/sources.list <<SOURCES
deb http://archive.debian.org/debian bullseye main
deb http://archive.debian.org/debian bullseye-updates main
deb http://archive.debian.org/debian-security bullseye-security main
SOURCES
apt-get -o Acquire::Check-Valid-Until=false update -qq
# shellcheck disable=SC2086
DEBIAN_FRONTEND=noninteractive apt-get install -y -qq --no-install-recommends $missing >/dev/null
