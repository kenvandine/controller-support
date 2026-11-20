#!/bin/sh
# Install the hid-xbox driver from a kernel tree as a DKMS module.
#
#   tools/dkms/install-dkms.sh <path-to-kernel-tree> [version]
#
# Needs: dkms, linux-headers-$(uname -r). Re-run after every change; the
# previous version is removed first.
set -eu

tree=${1:?usage: $0 <kernel-tree> [version]}
here=$(cd "$(dirname "$0")" && pwd)
# DKMS versions must be simple; date-based keeps every install unique.
ver=${2:-0.$(date +%Y%m%d%H%M)}
src=/usr/src/hid-xbox-$ver

for old in $(dkms status hid-xbox 2>/dev/null | sed -n 's|^hid-xbox/\([^,:]*\).*|\1|p' | sort -u); do
	sudo dkms remove "hid-xbox/$old" --all || true
	sudo rm -rf "/usr/src/hid-xbox-$old"
done

sudo mkdir -p "$src"
sudo cp "$tree/drivers/hid/hid-xbox.c" "$tree/drivers/hid/hid-microsoft.c" \
	"$tree/drivers/hid/hid-ids.h" "$here/Makefile" "$src/"
sed "s/@VERSION@/$ver/" "$here/dkms.conf.in" | sudo tee "$src/dkms.conf" >/dev/null

sudo dkms install "hid-xbox/$ver"

# Swap the running modules. Disconnect the controller first if this fails.
sudo modprobe -r hid-microsoft hid-xbox 2>/dev/null || true
sudo modprobe hid-xbox
sudo modprobe hid-microsoft
modinfo -n hid-xbox
echo "Installed hid-xbox $ver. Reconnect the controller over Bluetooth."
