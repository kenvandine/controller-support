#!/bin/sh
# Build Ubuntu kernel .debs with the hid-xbox series inside an LXD container.
#
#   tools/lxd-build.sh [build-number]
#
# Environment overrides:
#   CONTAINER  container name             (default: hidxbox-build)
#   RELEASE    Ubuntu release image       (default: 26.10)
#   KVER       Ubuntu linux source version (default: 7.3.0-11.12)
#   OUT        where .debs land on the host (default: <repo>/out/<version>)
#
# The container is created on first run (build-deps take a while) and reused
# after that. It needs roughly 40 GB of free space in the LXD storage pool.
set -eu

repo=$(cd "$(dirname "$0")/.." && pwd)
n=${1:-1}
name=${CONTAINER:-hidxbox-build}
release=${RELEASE:-26.10}
kver=${KVER:-7.3.0-11.12}
gname=$(git -C "$repo" config user.name || echo "hid-xbox builder")
gmail=$(git -C "$repo" config user.email || echo "builder@localhost")

log() { printf '\n=== %s: %s\n' "$(date '+%F %T')" "$*"; }
in_ct() { lxc exec "$name" --env DEBIAN_FRONTEND=noninteractive -- "$@"; }

if ! lxc info "$name" >/dev/null 2>&1; then
	log "creating container $name (ubuntu:$release)"
	lxc launch "ubuntu:$release" "$name"
	in_ct cloud-init status --wait >/dev/null 2>&1 || true
	log "installing build dependencies"
	in_ct sh -c 'sed -i "s/^Types: deb$/Types: deb deb-src/" /etc/apt/sources.list.d/ubuntu.sources &&
		apt-get update -q &&
		apt-get install -y -q fakeroot git dpkg-dev devscripts &&
		apt-get build-dep -y -q linux'
fi

log "fetching linux $kver source"
in_ct sh -c "rm -rf /build && mkdir -p /build/src && cd /build/src &&
	apt-get source -q linux=$kver >/dev/null"
in_ct mkdir -p /build/repo
lxc file push -r "$repo/patches" "$repo/tools" "$name/build/repo/"

log "applying patch series"
in_ct sh -c "cd /build/src/linux-* &&
	git init -q -b ubuntu/import &&
	git config user.name '$gname' && git config user.email '$gmail' &&
	git add -f -A && git commit -q -m 'Import Ubuntu linux $kver' &&
	git tag Ubuntu-$kver &&
	git switch -q -c hid-xbox/ubuntu &&
	git am -q /build/repo/patches/hid-xbox/*.patch &&
	git log --oneline Ubuntu-$kver..HEAD"

log "building (this takes a while)"
in_ct sh -c "cd /build/src/linux-* && sh /build/repo/tools/build-debs.sh . $n"

ver=$(in_ct sh -c 'cd /build/src/linux-* && dpkg-parsechangelog -l debian.master/changelog -S Version')
out=${OUT:-$repo/out/$ver}
mkdir -p "$out"
log "copying packages to $out"
for f in $(in_ct sh -c 'cd /build/src && ls *.deb'); do
	lxc file pull "$name/build/src/$f" "$out/"
done
ls -lh "$out"
log "done: $ver"
