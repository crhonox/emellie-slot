#!/bin/sh
set -eu

here="$(cd "$(dirname "$0")" && pwd)"

device_cflags="-mcpu=cortex-a53"
# armGBA's Game Boy side, Gambatte, is C++. libstdc++ and libgcc go into the core itself, so it
# never asks BaseOS for a libstdc++ as new as the one it was built with.
device_ldflags="-static-libstdc++ -static-libgcc"
recipe="cmake -DCMAKE_BUILD_TYPE=Release -DARMGBA_JIT=ON -DARMGBA_SDL=OFF"

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
	echo "source=https://github.com/Jacob-Matthew-Cook/armGBA/archive/$1.tar.gz"
	echo "recipe=$recipe"
	echo "device_cflags=$device_cflags"
	echo "device_ldflags=$device_ldflags"
	for p in "$here"/*.patch; do
		[ -f "$p" ] || continue
		echo "patch=$(basename "$p") sha256:$(sha256 "$p")"
	done
}

# Applies the patches with git kept out of any repository. Left to look, git finds the
# checkout this runs in, and a worktree's checkout names its repository by a path the build
# container does not have.
apply_patches() {
	ceiling="$(cd "$1/.." && pwd)"
	for p in "$here"/*.patch; do
		[ -f "$p" ] || continue
		GIT_CEILING_DIRECTORIES="$ceiling" git -C "$1" apply -p1 "$p"
	done
}

build() {
	commit="$1" tarball="$2" work="$3" out="$4"
	src="$work/armGBA-$commit"

	rm -rf "$work"
	mkdir -p "$work"
	tar -xzf "$tarball" -C "$work"
	if [ ! -f "$src/libretro/libretro.c" ]; then
		echo "$tarball does not hold armGBA-$commit/libretro/libretro.c" >&2
		exit 1
	fi

	apply_patches "$src"

	# shellcheck disable=SC2086
	CFLAGS="$device_cflags" CXXFLAGS="$device_cflags" $recipe \
		-DCMAKE_SHARED_LINKER_FLAGS="$device_ldflags" \
		-S "$src" -B "$work/build"
	# at most 8 at once, as for DSperate: many compiles at once through qemu run out of memory
	jobs="$(getconf _NPROCESSORS_ONLN)"
	[ "$jobs" -gt 8 ] && jobs=8
	cmake --build "$work/build" --target armgba_libretro -j"$jobs"

	mkdir -p "$(dirname "$out")"
	cp "$work/build/armgba_libretro.so" "$out"
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
