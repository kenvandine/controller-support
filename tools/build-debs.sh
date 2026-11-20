#!/bin/sh
# Build Ubuntu kernel .debs with the hid-xbox series applied.
#
#   tools/build-debs.sh <path-to-ubuntu-kernel-tree> [local-build-number]
#
# Run it inside an Ubuntu 26.10 (stonking) system or container so the
# toolchain matches what the archive kernel was built with, e.g.:
#
#   lxc launch ubuntu:26.10 kbuild   # or: podman run -it ubuntu:26.10
#   sudo sed -i 's/^Types: deb$/Types: deb deb-src/' \
#       /etc/apt/sources.list.d/ubuntu.sources
#   sudo apt update && sudo apt build-dep -y linux && \
#       sudo apt install -y fakeroot git
#
# The packages get ABI 90<abi> (e.g. 7.3.0-9011-generic), so they install
# next to the stock 7.3.0-11 kernel instead of replacing it, and GRUB
# boots them by default. Remove them with:
#   sudo apt purge 'linux-*-7.3.0-90*'
#
# The image is unsigned. With Secure Boot on, either turn it off for
# testing or sign the kernel with your MOK key.
set -eu

tree=${1:?usage: $0 <kernel-tree> [local-build-number]}
n=${2:-1}
cd "$tree"

# Work from a throwaway branch so the version bump never lands on the
# series branch.
base=$(git rev-parse --abbrev-ref HEAD)
git switch -C "build/$base-hidxbox$n"

ver=$(dpkg-parsechangelog -l debian.master/changelog -S Version)  # 7.3.0-11.12
upstream=${ver%-*}                                                  # 7.3.0
rev=${ver##*-}                                                      # 11.12
abi=${rev%%.*}                                                      # 11
upload=${rev#*.}                                                    # 12
newver="$upstream-90$abi.$upload+hidxbox$n"
series=$(dpkg-parsechangelog -l debian.master/changelog -S Distribution)

{
	printf 'linux (%s) %s; urgency=medium\n\n' "$newver" "$series"
	printf '  * Local test build with the hid-xbox patch series.\n\n'
	printf ' -- %s <%s>  %s\n\n' "$(git config user.name)" \
		"$(git config user.email)" "$(date -R)"
	cat debian.master/changelog
} >debian.master/changelog.new
mv debian.master/changelog.new debian.master/changelog
git commit -q -a -m "UBUNTU: local build $newver (do not upstream)"

LANG=C fakeroot debian/rules clean
# skipdbg avoids the multi-GB dbgsym packages; drop it if you need them.
LANG=C fakeroot debian/rules binary-headers binary-generic binary-perarch \
	skipdbg=true

echo
echo "Built $newver. Packages are in $(dirname "$PWD"):"
ls -1 ../linux-*"90$abi"*"+hidxbox$n"*.deb 2>/dev/null || ls -1 ../*.deb
echo
echo "Install with:"
echo "  sudo apt install ../linux-image-unsigned-$upstream-90$abi-generic_*.deb \\"
echo "    ../linux-modules-$upstream-90$abi-generic_*.deb \\"
echo "    ../linux-headers-$upstream-90$abi*_*.deb"
