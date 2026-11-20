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
| `tools/lxd-source.sh` | Builds the source package for a PPA upload in the same container (diff-only, `-sd`), and signs it on the host with `KEY=<keyid>`. Output goes to `out/source/<version>/`; upload with `dput ppa:<owner>/<ppa> out/source/<version>/linux_<version>_source.changes`. |
| `tools/capture-report.py` | Guided capture of a connected controller: driver, PID, firmware, transport, report descriptor, evdev capabilities and axis ranges, battery, then the raw report bytes and evdev events for every button, stick and trigger. Writes `<PID>-fw<version>-<transport>.{md,bin,rdesc.txt,log}`. Run with `sudo python3 -I tools/capture-report.py --out descriptors` on the test machine. |
| `tools/uhid-replay.py` | Creates a virtual HID device through uhid with a captured descriptor and IDs, so it binds to hid-xbox, then replays the raw reports of chosen steps from a `capture-report.py` log and prints the resulting evdev keys and KEY bitmap. Tests firmware/transport layouts without the hardware, e.g. `sudo python3 -I tools/uhid-replay.py --descriptor descriptors/02FD-fw0903-bt.bin --product 0x02fd --version 0x0903 --log captures/02FD-fw0903-bt.log --steps View,Menu,Xbox`. |
| `descriptors/` | Captured report descriptors and capture reports, one set per controller, firmware and transport, used to justify the descriptor fixups. |
| `captures/` | Guided capture runs (`capture-report.py` output plus the earlier shell-script run) and a per-controller analysis of the report layout and findings for the next phases. |
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
| 3 | HID: xbox: also report View and Xbox as gamepad buttons on classic firmware | v2: keeps KEY_BACK/KEY_HOMEPAGE, needs SDL re-test |
| U | UBUNTU: [Config] Enable CONFIG_HID_XBOX | done |

Next: tell classic Bluetooth and BLE firmware apart from the report
descriptor, descriptor fixups, Share/paddle mapping, trigger rumble, battery.
