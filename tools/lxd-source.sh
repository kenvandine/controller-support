#!/bin/sh
# Build a signed Ubuntu kernel source package with the hid-xbox series,
# ready to dput to a PPA, inside an LXD container.
#
#   tools/lxd-source.sh [build-number]
#
# Environment overrides:
#   CONTAINER  container name             (default: hidxbox-build)
#   RELEASE    Ubuntu release image       (default: 26.10)
#   KVER       Ubuntu linux source version (default: 7.3.0-11.12)
#   KEY        GPG key id to sign with    (default: unset = leave unsigned)
#   OUT        where the source package lands on the host
#              (default: <repo>/out/source/<version>)
#
# The upload is diff-only (-sd): Launchpad takes linux_<upstream>.orig.tar.gz
# from the Ubuntu archive, so only the .dsc, .diff.gz, .buildinfo and
# .changes go up. Bump the build number for every new upload to the same PPA.
#
# Signing happens on the host with debsign, so the key never enters the
# container. Without KEY the files are left unsigned and the debsign command
# is printed. Upload with:
#   dput ppa:<owner>/<ppa> <OUT>/linux_<version>_source.changes
#
# Shares the container with tools/lxd-build.sh but works in /srcpkg, which
# lxd-build.sh never touches.
set -eu

repo=$(cd "$(dirname "$0")/.." && pwd)
n=${1:-1}
name=${CONTAINER:-hidxbox-build}
release=${RELEASE:-26.10}
kver=${KVER:-7.3.0-11.12}
key=${KEY:-}
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
in_ct sh -c "rm -rf /srcpkg && mkdir -p /srcpkg/src /srcpkg/repo && cd /srcpkg/src &&
	apt-get source -q linux=$kver >/dev/null"
lxc file push -r "$repo/patches" "$name/srcpkg/repo/"

log "applying patch series"
in_ct sh -c "cd /srcpkg/src/linux-* &&
	git init -q -b ubuntu/import &&
	git config user.name '$gname' && git config user.email '$gmail' &&
	git add -f -A && git commit -q -m 'Import Ubuntu linux $kver' &&
	git tag Ubuntu-$kver &&
	git switch -q -c hid-xbox/ubuntu &&
	git am -q /srcpkg/repo/patches/hid-xbox/*.patch &&
	git log --oneline Ubuntu-$kver..HEAD"

# Same version scheme as tools/build-debs.sh: ABI 90<abi> so the packages
# install next to the stock kernel, e.g. 7.3.0-9011.12+hidxbox1. Also as
# there, drop linux-modules' Depends on linux-main-modules-{zfs,v4l2loopback}:
# the archive builds those from separate source packages, so neither a PPA
# nor a local install has them for this ABI.
log "building source package"
in_ct sh -c "cd /srcpkg/src/linux-* &&
	ver=\$(dpkg-parsechangelog -l debian.master/changelog -S Version) &&
	upstream=\${ver%-*} && rev=\${ver##*-} &&
	abi=\${rev%%.*} && upload=\${rev#*.} &&
	newver=\$upstream-90\$abi.\$upload+hidxbox$n &&
	series=\$(dpkg-parsechangelog -l debian.master/changelog -S Distribution) &&
	{
		printf 'linux (%s) %s; urgency=medium\n\n' \"\$newver\" \"\$series\"
		printf '  * Test build with the hid-xbox patch series.\n\n'
		printf ' -- %s <%s>  %s\n\n' '$gname' '$gmail' \"\$(date -R)\"
		cat debian.master/changelog
	} >debian.master/changelog.new &&
	mv debian.master/changelog.new debian.master/changelog &&
	printf 'do_linux_main_modules_depends = false\n' >>debian.master/rules.d/hooks.mk &&
	git add debian.master/rules.d/hooks.mk &&
	git commit -q -a -m \"UBUNTU: PPA build \$newver (do not upstream)\" &&
	LANG=C fakeroot debian/rules clean &&
	LANG=C dpkg-buildpackage -S -sd -us -uc"

ver=$(in_ct sh -c 'cd /srcpkg/src/linux-* && dpkg-parsechangelog -l debian.master/changelog -S Version')
out=${OUT:-$repo/out/source/$ver}
mkdir -p "$out"
log "copying source package to $out"
for f in $(in_ct sh -c "cd /srcpkg/src && ls linux_$ver.* linux_${ver}_source.*"); do
	lxc file pull "$name/srcpkg/src/$f" "$out/"
done

changes=$out/linux_${ver}_source.changes
if [ -n "$key" ]; then
	log "signing with $key"
	debsign -k"$key" "$changes"
else
	log "not signing (KEY unset); sign with:"
	echo "  debsign -k<keyid> $changes"
fi
ls -lh "$out"
log "done: $ver"
echo "Upload with: dput ppa:<owner>/<ppa> $changes"
