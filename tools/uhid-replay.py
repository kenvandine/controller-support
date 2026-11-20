#!/usr/bin/env python3
"""Replay a captured controller through uhid to test hid-xbox without the hardware.

    sudo python3 -I tools/uhid-replay.py --descriptor descriptors/02FD-fw0903-bt.bin \\
        --product 0x02fd --version 0x0903 \\
        --log captures/02FD-fw0903-bt.log --steps View,Menu,Xbox

It creates a virtual HID device (default bus Bluetooth, vendor 045E) with the
captured report descriptor, so the kernel binds it to whatever driver claims
those IDs (hid-xbox). It then writes the raw input reports recorded by
tools/capture-report.py for the chosen steps, and prints the evdev events
each step produced plus the device's KEY capability bitmap. The device gets
uniq "uhid-replay", so it never touches a real controller's hidraw or event
node, and it is destroyed on exit.

Python 3 standard library only. Needs root (/dev/uhid).
"""
import argparse
import glob
import os
import select
import struct
import sys
import time

UHID_DESTROY, UHID_CREATE2, UHID_INPUT2 = 1, 11, 12
BUS = {"usb": 3, "bluetooth": 5}
EV_KEY, EV_ABS = 1, 3
KEY_NAMES = {158: "KEY_BACK", 172: "KEY_HOMEPAGE", 167: "KEY_RECORD", 304: "BTN_SOUTH",
             305: "BTN_EAST", 306: "BTN_C", 307: "BTN_NORTH", 308: "BTN_WEST", 309: "BTN_Z",
             310: "BTN_TL", 311: "BTN_TR", 312: "BTN_TL2", 313: "BTN_TR2", 314: "BTN_SELECT",
             315: "BTN_START", 316: "BTN_MODE", 317: "BTN_THUMBL", 318: "BTN_THUMBR"}
ABS_NAMES = {0: "ABS_X", 1: "ABS_Y", 2: "ABS_Z", 3: "ABS_RX", 4: "ABS_RY", 5: "ABS_RZ",
             9: "ABS_GAS", 10: "ABS_BRAKE", 16: "ABS_HAT0X", 17: "ABS_HAT0Y"}
UNIQ = "uhid-replay"


def create(fd, rdesc, bus, vendor, product, version, name):
    ev = struct.pack("<I128s64s64sHHIIII4096s", UHID_CREATE2, name.encode(), b"uhid-replay",
                     UNIQ.encode(), len(rdesc), bus, vendor, product, version, 0, rdesc)
    os.write(fd, ev)


def send(fd, data):
    os.write(fd, struct.pack("<IH4096s", UHID_INPUT2, len(data), data))


def find_hid_dir(vendor, product, timeout=5):
    end = time.monotonic() + timeout
    pat = f"/sys/devices/virtual/misc/uhid/*:{vendor:04X}:{product:04X}.*"
    while time.monotonic() < end:
        for d in glob.glob(pat):
            try:
                if open(d + "/uevent").read().find(f"HID_UNIQ={UNIQ}") >= 0:
                    evs = glob.glob(d + "/input/input*/event*")
                    if evs and os.path.exists(d + "/driver"):
                        return d, sorted(evs)
            except OSError:
                pass
        time.sleep(0.05)
    sys.exit("virtual device did not bind or create an event node")


def bitmap_codes(bitmap):
    codes = []
    for i, w in enumerate(reversed(bitmap.split())):
        v = int(w, 16)
        codes += [i * 64 + b for b in range(64) if v >> b & 1]
    return codes


def load_steps(path, steps):
    out = {s: [] for s in steps}
    with open(path) as f:
        for line in f:
            p = line.split()
            if len(p) > 3 and p[0] in out and p[2].startswith("hidraw"):
                out[p[0]].append(bytes.fromhex("".join(p[3:])))
    return out


def drain(fds, wait):
    evs = []
    end = time.monotonic() + wait
    while True:
        left = end - time.monotonic()
        if left <= 0:
            break
        r, _, _ = select.select(fds, [], [], left)
        for fd in r:
            data = os.read(fd, 4096)
            for off in range(0, len(data) - 23, 24):
                _, _, typ, code, val = struct.unpack("qqHHi", data[off:off + 24])
                if typ in (EV_KEY, EV_ABS):
                    evs.append((typ, code, val))
    return evs


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--descriptor", required=True)
    ap.add_argument("--bus", choices=BUS, default="bluetooth")
    ap.add_argument("--vendor", type=lambda s: int(s, 0), default=0x045E)
    ap.add_argument("--product", type=lambda s: int(s, 0), required=True)
    ap.add_argument("--version", type=lambda s: int(s, 0), default=0)
    ap.add_argument("--name", default="Xbox Wireless Controller")
    ap.add_argument("--log", help="capture-report.py .log to take reports from")
    ap.add_argument("--steps", default="", help="comma-separated step labels to replay")
    ap.add_argument("--gap", type=float, default=0.02, help="seconds between reports")
    args = ap.parse_args()
    if os.geteuid() != 0:
        sys.exit("Run with sudo: /dev/uhid is root-only.")

    rdesc = open(args.descriptor, "rb").read()
    steps = [s for s in args.steps.split(",") if s]
    reports = load_steps(args.log, steps) if args.log and steps else {}

    ufd = os.open("/dev/uhid", os.O_RDWR)
    try:
        create(ufd, rdesc, BUS[args.bus], args.vendor, args.product, args.version, args.name)
        hid_dir, evnodes = find_hid_dir(args.vendor, args.product)
        driver = os.path.basename(os.path.realpath(hid_dir + "/driver"))
        print(f"device {os.path.basename(hid_dir)} bound to {driver}")
        efds = []
        for ev in evnodes:
            inp = os.path.dirname(ev)
            key = open(inp + "/capabilities/key").read().strip()
            names = [KEY_NAMES.get(c, str(c)) for c in bitmap_codes(key)]
            print(f"{os.path.basename(ev)} KEY {key}")
            print(f"  keys: {', '.join(names)}")
            for n in ("BTN_SELECT", "BTN_MODE", "BTN_START"):
                print(f"  {n} advertised {names.count(n)}x")
            efds.append(os.open("/dev/input/" + os.path.basename(ev), os.O_RDONLY | os.O_NONBLOCK))
        drain(efds, 0.3)
        for step in steps:
            evs = []
            for data in reports.get(step, []):
                send(ufd, data)
                evs += drain(efds, args.gap)
            evs += drain(efds, 0.2)
            keys = {}
            for typ, code, val in evs:
                if typ == EV_KEY:
                    keys.setdefault(KEY_NAMES.get(code, str(code)), []).append(val)
            print(f"[{step}] {len(reports.get(step, []))} reports -> "
                  + (", ".join(f"{k} {v}" for k, v in keys.items()) or "no key events"))
    finally:
        os.write(ufd, struct.pack("<I", UHID_DESTROY))
        os.close(ufd)


if __name__ == "__main__":
    main()
