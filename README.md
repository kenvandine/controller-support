# controller-support

Work-in-progress Linux kernel support for Xbox Wireless Controllers over
Bluetooth: a new `hid-xbox` HID driver split out of `hid-microsoft`, built
as an incremental, bisectable patch series that can later go upstream.

The series is developed against Ubuntu's kernel and tested from Ubuntu
kernel packages until it is ready for the upstream HID tree.

## Layout

| Path | What |
| --- | --- |
| `patches/hid-xbox/` | `git format-patch` series on top of Ubuntu tag `Ubuntu-7.3.0-11.12` (26.10). `HID:` patches touch only `drivers/hid/`; `UBUNTU:` patches touch only `debian.master/`. |
| `tools/lxd-build.sh` | Builds Ubuntu kernel .debs with the series in an Ubuntu 26.10 LXD container. Output goes to `out/<version>/`. |
| `tools/build-debs.sh` | The build step itself, run inside a prepared kernel tree. Bumps the version to `7.3.0-90<abi>.<upload>+hidxboxN` so it installs next to the stock kernel. |
| `tools/capture-controller.sh` | Guided capture of a connected controller (descriptor, IDs, evdev caps, raw reports per button) for the next phases. Run with sudo on the test machine. |
| `tools/dkms/` | Fast loop: builds `hid-xbox` and the matching `hid-microsoft` as DKMS modules for the running kernel. |

## Build kernel packages

```sh
tools/lxd-build.sh 1          # build number -> +hidxbox1
sudo apt install ./out/7.3.0-9011.12+hidxbox1/linux-{image-unsigned,modules,headers}-7.3.0-9011*.deb \
                 ./out/7.3.0-9011.12+hidxbox1/linux-headers-7.3.0-9011_*.deb
```

The kernel image is unsigned: disable Secure Boot or sign it with your MOK.
Remove with `sudo apt purge 'linux-*-7.3.0-90*'`.

## Apply the series to a kernel tree

```sh
git clone git://git.launchpad.net/~ubuntu-kernel/ubuntu/+source/linux/+git/stonking
cd stonking && git switch -c hid-xbox/ubuntu Ubuntu-7.3.0-11.12
git am /path/to/controller-support/patches/hid-xbox/*.patch
```

## Status

| # | Patch | State |
| --- | --- | --- |
| 1 | HID: microsoft: flush pending rumble work on remove | done |
| 2 | HID: xbox: split Xbox Wireless Controller support out of hid-microsoft | done |
| U | UBUNTU: [Config] Enable CONFIG_HID_XBOX | done |

Next: tell classic Bluetooth and BLE firmware apart from the report
descriptor, descriptor fixups, Share/paddle mapping, trigger rumble, battery.
