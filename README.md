# controller-support

Work-in-progress Linux kernel support for Xbox Wireless Controllers over
Bluetooth: a new `hid-xbox` HID driver split out of `hid-microsoft`, built
as an incremental, bisectable patch series that can later go upstream.

The series is developed against Ubuntu's kernel and tested from Ubuntu
kernel packages until it is ready for the upstream HID tree.

## Layout

| Path | What |
| --- | --- |
| `patches/hid-xbox/` | `git format-patch` series on top of Ubuntu tag `Ubuntu-7.3.0-11.12` (26.10). `HID:` patches touch only `drivers/hid/`, `selftests/hid:` only `tools/testing/selftests/hid/`; `UBUNTU:` patches touch only `debian.master/`. |
| `tools/lxd-build.sh` | Builds Ubuntu kernel .debs with the series in an Ubuntu 26.10 LXD container. Output goes to `out/<version>/`. |
| `tools/build-debs.sh` | The build step itself, run inside a prepared kernel tree. Bumps the version to `7.3.0-90<abi>.<upload>+hidxboxN` so it installs next to the stock kernel. |
| `tools/lxd-source.sh` | Builds the source package for a PPA upload in the same container (diff-only, `-sd`), and signs it on the host with `KEY=<keyid>`. Output goes to `out/source/<version>/`; upload with `dput ppa:<owner>/<ppa> out/source/<version>/linux_<version>_source.changes`. |
| `tools/capture-report.py` | Guided capture of a connected controller: driver, PID, firmware, transport, report descriptor, evdev capabilities and axis ranges, battery, then the raw report bytes and evdev events for every button, stick and trigger. Writes `<PID>-fw<version>-<transport>.{md,bin,rdesc.txt,log}`. Run with `sudo python3 -I tools/capture-report.py --out descriptors` on the test machine. |
| `tools/uhid-replay.py` | Creates a virtual HID device through uhid with a captured descriptor and IDs, so it binds to hid-xbox, then replays the raw reports of chosen steps from a `capture-report.py` log and prints the resulting evdev keys and KEY bitmap. Tests firmware/transport layouts without the hardware, e.g. `sudo python3 -I tools/uhid-replay.py --descriptor descriptors/02FD-fw0903-bt.bin --product 0x02fd --version 0x0903 --log captures/02FD-fw0903-bt.log --steps View,Menu,Xbox`. |
| `tools/pad-viewer/` | Rust + SDL3 app that draws a gamepad and lights each control live, with SDL's raw button numbers and mapping string. See its README. |
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
| 1 | HID: microsoft: flush pending rumble work on remove | done, tested |
| 2 | HID: microsoft: report View and Xbox as gamepad buttons on classic Xbox firmware | done; keeps KEY_BACK/KEY_HOMEPAGE and adds BTN_SELECT/BTN_MODE, detected from the descriptor |
| 3 | selftests/hid: add tests for the Xbox Wireless Controller over Bluetooth | new; replays both captured 1708 descriptors, needs a run on umoja |
| 4 | HID: microsoft: rate-limit Xbox controller rumble reports | new; at most one rumble report per 10 ms, latest values win. Needs a hardware run |
| 5 | HID: xbox: split Xbox Wireless Controller support out of hid-microsoft | optional; can be dropped before upstream without touching 1-4 |
| U | UBUNTU: [Config] Enable CONFIG_HID_XBOX | done |

Scope for the first upstream submission is patches 1-3 (Ken, 2026-11-20).
Trigger rumble, battery and Elite paddles are deferred. Stick axes
(Z/RZ), triggers (GAS/BRAKE) and the A/B/X/Y codes stay as they are, so
existing SDL and Steam mappings keep working.
