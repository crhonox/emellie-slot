#!/bin/sh
# Builds DSperate, the DS emulator omega-zero hands the handheld over to, from its pinned source
# archive. It is not a libretro core: it is its own program, drawing to fb0 through the mali-fbdev
# SDL2 shipped beside it, and launch_frontend.sh runs it once omega-zero has left.
#
# Built where the device build is (arm64 Debian bullseye, glibc 2.31), or with CROSS=1 from an
# x86 Debian with g++-aarch64-linux-gnu and libsdl2-dev:arm64. libstdc++ and libgcc are linked
# in, as DSperate's own release is, so the card needs only glibc and the SDL2 beside it.
set -eu

here="$(cd "$(dirname "$0")" && pwd)"

# The SDL frontend alone: no tests, no headless harness, no Wayland, and no Vulkan, which the
# H700's Mali-G31 has none of under BaseOS. RetroAchievements and the LAN's local wireless stay.
recipe="cmake -G Ninja -DCMAKE_BUILD_TYPE=Release -DDSPERATE_TESTS=OFF -DDSPERATE_HEADLESS=OFF -DDSPERATE_WAYLAND=OFF -DDSPERATE_VULKAN=OFF -DDSPERATE_PGO=OFF -DDSPERATE_TUNE=-mcpu=cortex-a53"
device_ldflags="-static-libstdc++ -static-libgcc"

usage() {
	echo "usage: $0 stamp COMMIT | build COMMIT TARBALL WORKDIR OUT" >&2
	exit 2
}

sha256() {
	if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1"; else shasum -a 256 "$1"; fi |
		cut -d' ' -f1
}

stamp() {
	echo "commit=$1"
	echo "source=https://github.com/beebono/DSperate/archive/$1.tar.gz"
	echo "recipe=$recipe"
	echo "device_ldflags=$device_ldflags"
	for p in "$here"/*.patch; do
		[ -f "$p" ] || continue
		echo "patch=$(basename "$p") sha256:$(sha256 "$p")"
	done
}

apply_patches() {
	ceiling="$(cd "$1/.." && pwd)"
	for p in "$here"/*.patch; do
		[ -f "$p" ] || continue
		GIT_CEILING_DIRECTORIES="$ceiling" git -C "$1" apply -p1 "$p"
	done
}

build() {
	commit="$1" tarball="$2" work="$3" out="$4"
	src="$work/DSperate-$commit"

	rm -rf "$work"
	mkdir -p "$work"
	tar -xzf "$tarball" -C "$work"
	if [ ! -f "$src/CMakeLists.txt" ]; then
		echo "$tarball does not hold DSperate-$commit/CMakeLists.txt" >&2
		exit 1
	fi

	apply_patches "$src"

	cross=
	if [ "${CROSS:-0}" = 1 ]; then
		cross="-DCMAKE_TOOLCHAIN_FILE=$src/cmake/aarch64-linux-gnu.cmake"
		export PKG_CONFIG_LIBDIR=/usr/lib/aarch64-linux-gnu/pkgconfig:/usr/share/pkgconfig
	fi
	# shellcheck disable=SC2086
	$recipe $cross -DCMAKE_EXE_LINKER_FLAGS="$device_ldflags" \
		-DDSPERATE_VERSION="$(printf %s "$commit" | cut -c1-7)" \
		-S "$src" -B "$work/build"
	cmake --build "$work/build" --target dsperate -j"$(getconf _NPROCESSORS_ONLN)"

	mkdir -p "$(dirname "$out")"
	cp "$work/build/src/frontend/sdl/dsperate" "$out"
	"${STRIP:-strip}" "$out" 2>/dev/null || true
	stamp "$commit" >"$out.meta"
}

case "${1:-}" in
stamp)
	[ $# -eq 2 ] && [ -n "$2" ] || usage
	stamp "$2"
	;;
build)
	[ $# -eq 5 ] && [ -n "$2" ] && [ -n "$3" ] && [ -n "$4" ] && [ -n "$5" ] || usage
	build "$2" "$3" "$4" "$5"
	;;
*)
	usage
	;;
esac
