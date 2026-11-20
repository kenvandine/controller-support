#!/usr/bin/env python3
"""Guided capture of an Xbox controller for the hid-xbox work.

    sudo python3 -I tools/capture-report.py [--out DIR] [--seconds N]

Run it on the machine the controller is connected to, with the controller
paired and nothing else holding it. It finds the controller (any HID device
from Microsoft, 045E), records the static facts (driver, PID, firmware,
transport, report descriptor, evdev capabilities and axis ranges, battery),
then walks you through every button, stick and trigger. For each step it
records the raw HID input reports from hidraw and the evdev events from the
event node at the same time, so the report shows:

  * which report ID and which bytes/bits each control changes in the raw
    report (needed for the descriptor fixups in Phase 2.1 and for spotting
    vendor bytes such as the Elite paddles in 2.3), and
  * which evdev code the kernel turns it into and the range actually seen
    (needed to show the parity and the before/after of each fixup).

It writes <PID>-fw<version>-<transport>.md (the report), .bin (the raw
descriptor) and .log (every raw report and event, timestamped) to --out,
default ./descriptors. Nothing is changed on the system. Python 3 standard
library only.
"""
import argparse
import datetime
import glob
import hashlib
import os
import select
import struct
import subprocess
import sys
import time

EV_NAMES = {0: "EV_SYN", 1: "EV_KEY", 2: "EV_REL", 3: "EV_ABS", 4: "EV_MSC", 0x15: "EV_FF"}
KEY_NAMES = {
    158: "KEY_BACK", 172: "KEY_HOMEPAGE", 139: "KEY_MENU", 167: "KEY_RECORD",
    304: "BTN_SOUTH", 305: "BTN_EAST", 306: "BTN_C", 307: "BTN_NORTH", 308: "BTN_WEST",
    309: "BTN_Z", 310: "BTN_TL", 311: "BTN_TR", 312: "BTN_TL2", 313: "BTN_TR2",
    314: "BTN_SELECT", 315: "BTN_START", 316: "BTN_MODE", 317: "BTN_THUMBL", 318: "BTN_THUMBR",
    0x224: "BTN_GRIPL", 0x225: "BTN_GRIPR", 0x226: "BTN_GRIPL2", 0x227: "BTN_GRIPR2",
}
KEY_NAMES.update({0x2C0 + i: f"BTN_TRIGGER_HAPPY{i + 1}" for i in range(40)})
ABS_NAMES = {0: "ABS_X", 1: "ABS_Y", 2: "ABS_Z", 3: "ABS_RX", 4: "ABS_RY", 5: "ABS_RZ",
             6: "ABS_THROTTLE", 7: "ABS_RUDDER", 9: "ABS_GAS", 10: "ABS_BRAKE",
             16: "ABS_HAT0X", 17: "ABS_HAT0Y"}

STEPS = [
    ("idle", "Leave the controller untouched (baseline)", 3),
    ("A", "Press and release A", None),
    ("B", "Press and release B", None),
    ("X", "Press and release X", None),
    ("Y", "Press and release Y", None),
    ("LB", "Press and release LB (left bumper)", None),
    ("RB", "Press and release RB (right bumper)", None),
    ("View", "Press and release View (two squares, left of the Xbox button)", None),
    ("Menu", "Press and release Menu (three lines, right of the Xbox button)", None),
    ("Xbox", "Press and release the Xbox button (briefly, don't hold)", None),
    ("Share", "Press and release Share (Series X|S only)", None),
    ("LS", "Click the left stick", None),
    ("RS", "Click the right stick", None),
    ("Up", "Press and release D-pad up", None),
    ("Down", "Press and release D-pad down", None),
    ("Left", "Press and release D-pad left", None),
    ("Right", "Press and release D-pad right", None),
    ("LStick", "Roll the LEFT stick around its edge 3 times, then let go", 6),
    ("RStick", "Roll the RIGHT stick around its edge 3 times, then let go", 6),
    ("LT", "Pull LT all the way in slowly, then release slowly", 5),
    ("RT", "Pull RT all the way in slowly, then release slowly", 5),
    ("P1", "Elite only: press upper-left paddle P1", None),
    ("P2", "Elite only: press upper-right paddle P2", None),
    ("P3", "Elite only: press lower-left paddle P3", None),
    ("P4", "Elite only: press lower-right paddle P4", None),
    ("Profile", "Elite only: press the profile button once", None),
]
BUTTON_WINDOW = 3  # seconds to capture for a press-and-release step


def read(path, default=""):
    try:
        with open(path) as f:
            return f.read().strip()
    except OSError:
        return default


def run(cmd):
    try:
        return subprocess.run(cmd, capture_output=True, text=True, timeout=10).stdout.strip()
    except (OSError, subprocess.SubprocessError):
        return ""


def find_controller():
    for dev in sorted(glob.glob("/sys/bus/hid/devices/*")):
        ue = dict(l.split("=", 1) for l in read(dev + "/uevent").splitlines() if "=" in l)
        hid_id = ue.get("HID_ID", "")
        parts = hid_id.split(":")
        if len(parts) == 3 and parts[1].upper().endswith("045E"):
            return dev, ue
    sys.exit("No Microsoft (045E) HID device found. Is the controller connected?")


def find_nodes(dev):
    hidraw = [os.path.basename(p) for p in glob.glob(dev + "/hidraw/hidraw*")]
    events = []
    for inp in sorted(glob.glob(dev + "/input/input*")):
        for ev in glob.glob(inp + "/event*"):
            events.append((os.path.basename(ev), read(inp + "/name"), inp))
    return hidraw, events


def absinfo(fd, code):
    buf = bytearray(24)
    import fcntl
    try:
        fcntl.ioctl(fd, 0x80184540 + code, buf)
    except OSError:
        return None
    return struct.unpack("6i", buf)


def caps(inp):
    out = {}
    for k in ("ev", "key", "abs", "msc", "ff"):
        out[k] = read(f"{inp}/capabilities/{k}")
    return out


def bitmap_codes(bitmap):
    words = bitmap.split()
    codes = []
    for i, w in enumerate(reversed(words)):
        v = int(w, 16)
        for b in range(64):
            if v >> b & 1:
                codes.append(i * 64 + b)
    return codes


def transport(ue, mac):
    bus = ue.get("HID_ID", "").split(":")[0]
    if bus == "0003":
        return "usb"
    if bus != "0005":
        return f"bus{bus}"
    info = run(["bluetoothctl", "info", mac]) if mac else ""
    if "00001812" in info:
        return "ble"
    if "00001124" in info:
        return "bt"
    return "bt-unknown"


def capture(fds, seconds, log, label, wait_for_input):
    """Read hidraw reports and evdev events for `seconds` (or, for a button
    step, until BUTTON_WINDOW seconds after the first change)."""
    reports, events = [], []
    t0 = time.monotonic()
    deadline = t0 + seconds
    while True:
        left = deadline - time.monotonic()
        if left <= 0:
            break
        r, _, _ = select.select(list(fds), [], [], left)
        for fd in r:
            kind, node = fds[fd]
            try:
                data = os.read(fd, 4096)
            except OSError:
                continue
            ts = time.monotonic() - t0
            if kind == "hidraw":
                reports.append((ts, data))
                log.write(f"{label} {ts:8.3f} {node} {data.hex(' ')}\n")
            else:
                for off in range(0, len(data) - 23, 24):
                    _, _, typ, code, val = struct.unpack("qqHHi", data[off:off + 24])
                    if typ == 0:
                        continue
                    events.append((ts, node, typ, code, val))
                    log.write(f"{label} {ts:8.3f} {node} {EV_NAMES.get(typ, typ)} {code} {val}\n")
    return reports, events


def describe_report_changes(baseline, reports):
    """Which (report id, byte) positions change, with the bits/values seen."""
    changes = {}
    for _, data in reports:
        rid = data[0]
        base = baseline.get(rid)
        if base is None or len(base) != len(data):
            changes.setdefault((rid, "len"), set()).add(len(data))
            continue
        for i, (a, b) in enumerate(zip(base, data)):
            if a != b:
                changes.setdefault((rid, i), set()).add(b)
    out = []
    for (rid, i), vals in sorted(changes.items(), key=lambda x: (x[0][0], str(x[0][1]))):
        if i == "len":
            out.append(f"report 0x{rid:02x} length {sorted(vals)}")
            continue
        base = baseline[rid][i]
        bits = 0
        for v in vals:
            bits |= v ^ base
        vs = sorted(vals)
        span = f"{vs[0]}..{vs[-1]}" if len(vs) > 4 else ",".join(map(str, vs))
        out.append(f"report 0x{rid:02x} byte {i}: base {base} -> {span} (bits 0x{bits:02x})")
    return out


def describe_events(events):
    seen = {}
    for _, node, typ, code, val in events:
        if typ == 4:  # EV_MSC scan codes: keep them, they show the HID usage
            name = f"MSC_SCAN 0x{val & 0xffffffff:08x}"
            seen.setdefault((node, name), [val, val])
            continue
        if typ == 1:
            name = KEY_NAMES.get(code, f"KEY_{code}")
        elif typ == 3:
            name = ABS_NAMES.get(code, f"ABS_{code}")
        else:
            name = f"{EV_NAMES.get(typ, typ)}:{code}"
        lo_hi = seen.setdefault((node, name), [val, val])
        lo_hi[0] = min(lo_hi[0], val)
        lo_hi[1] = max(lo_hi[1], val)
    out = []
    for (node, name), (lo, hi) in sorted(seen.items()):
        out.append(f"{node} {name}" if name.startswith("MSC") else f"{node} {name} {lo}..{hi}")
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--out", default="descriptors")
    ap.add_argument("--seconds", type=float, default=None,
                    help="override every step's capture time")
    ap.add_argument("--static-only", action="store_true",
                    help="record the static facts and skip the guided steps")
    ap.add_argument("--steps", metavar="NAME[,NAME...]",
                    help="run only these guided steps, e.g. --steps LStick,RStick "
                         f"(names: {', '.join(s[0] for s in STEPS)}); the output "
                         "files get the step names as a suffix")
    args = ap.parse_args()
    steps = STEPS
    if args.steps:
        want = [s.strip().lower() for s in args.steps.split(",") if s.strip()]
        known = {s[0].lower(): s for s in STEPS}
        bad = [s for s in want if s not in known]
        if bad:
            sys.exit(f"Unknown step(s): {', '.join(bad)}. Known: {', '.join(s[0] for s in STEPS)}")
        steps = [known[s] for s in want]
    if os.geteuid() != 0:
        sys.exit("Run with sudo: hidraw and event nodes are root-only.")

    dev, ue = find_controller()
    hid_id = os.path.basename(dev)
    pid = ue["HID_ID"].split(":")[2][-4:].upper()
    mac = ue.get("HID_UNIQ", "")
    hidraw, events = find_nodes(dev)
    rdesc = open(dev + "/report_descriptor", "rb").read()
    driver = os.path.basename(os.path.realpath(dev + "/driver")) if os.path.exists(dev + "/driver") else "(none)"

    # Firmware: the BT/USB "version" (bcdDevice) is in the input id.
    ver = ""
    for _, _, inp in events:
        ver = read(inp + "/id/version")
        if ver:
            break
    tr = transport(ue, mac)
    stem = f"{pid}-fw{ver or 'unknown'}-{tr}"
    if args.steps:
        stem += "-" + "-".join(s[0] for s in steps)
    os.makedirs(args.out, exist_ok=True)
    base = os.path.join(args.out, stem)
    with open(base + ".bin", "wb") as f:
        f.write(rdesc)

    md = []
    w = md.append
    w(f"# {ue.get('HID_NAME', 'Controller')}: {stem}\n")
    w(f"Captured {datetime.datetime.now().astimezone():%Y-%m-%d %H:%M %Z} on {os.uname().nodename}, "
      f"kernel {os.uname().release}.\n")
    w("## Device\n")
    w(f"- HID device: `{hid_id}`, driver **{driver}**")
    w(f"- HID_ID `{ue.get('HID_ID')}`, PID {pid}, firmware/version 0x{ver}, transport {tr}")
    w(f"- MAC/uniq `{mac}`, hidraw {', '.join(hidraw) or '-'}")
    mods = [l.split()[0] for l in read("/proc/modules").splitlines()
            if l.split()[0] in ("hid_xbox", "hid_microsoft", "xpad", "hid_xpadneo", "ff_memless")]
    w(f"- Modules loaded: {', '.join(mods) or '-'}")
    w(f"- Report descriptor: {len(rdesc)} bytes, sha256 `{hashlib.sha256(rdesc).hexdigest()}` "
      f"(saved as `{os.path.basename(base)}.bin`)\n")
    w("```")
    for i in range(0, len(rdesc), 16):
        w(f"{i:04x}: " + " ".join(f"{b:02x}" for b in rdesc[i:i + 16]))
    w("```\n")
    dbg = read(f"/sys/kernel/debug/hid/{hid_id}/rdesc")
    if dbg:
        with open(base + ".rdesc.txt", "w") as f:
            f.write(dbg + "\n")
        w(f"Kernel parse saved as `{os.path.basename(base)}.rdesc.txt`.\n")

    w("## Input nodes\n")
    fds = {}
    for ev, name, inp in events:
        c = caps(inp)
        w(f"### {ev}: {name}\n")
        w(f"- EV `{c['ev']}`  KEY `{c['key']}`  ABS `{c['abs']}`  MSC `{c['msc']}`  FF `{c['ff']}`")
        keys = [KEY_NAMES.get(k, str(k)) for k in bitmap_codes(c["key"] or "0")]
        w(f"- Keys: {', '.join(keys) or '-'}")
        fd = os.open(f"/dev/input/{ev}", os.O_RDONLY | os.O_NONBLOCK)
        fds[fd] = ("event", ev)
        for code in bitmap_codes(c["abs"] or "0"):
            ai = absinfo(fd, code)
            if ai:
                w(f"- {ABS_NAMES.get(code, code)}: min {ai[1]} max {ai[2]} fuzz {ai[3]} flat {ai[4]}")
        ffs = bitmap_codes(c["ff"] or "0")
        if ffs:
            w(f"- FF codes: {', '.join(map(str, ffs))}")
        w("")
    for ps in glob.glob(dev + "/power_supply/*"):
        w(f"## Battery `{os.path.basename(ps)}`\n")
        for l in read(ps + "/uevent").splitlines():
            w(f"- {l}")
        w("")

    if not args.static_only:
        for h in hidraw:
            fds[os.open(f"/dev/{h}", os.O_RDONLY | os.O_NONBLOCK)] = ("hidraw", h)
        w("## Guided capture\n")
        w("Raw changes are against the idle baseline; bits are the XOR of every value seen.\n")
        baseline = {}
        with open(base + ".log", "w") as log:
            for label, prompt, secs in steps:
                skippable = "only" in prompt
                ans = input(f"\n[{label}] {prompt}. Press Enter, then do it"
                            f"{' (or type s + Enter to skip)' if skippable else ''}: ").strip().lower()
                if ans == "s":
                    w(f"### {label}: skipped\n")
                    continue
                dur = args.seconds or secs or BUTTON_WINDOW
                print(f"  capturing {dur:g}s...", flush=True)
                reps, evs = capture(fds, dur, log, label, secs is None)
                if label == "idle":
                    for _, data in reps:
                        baseline[data[0]] = data
                    w(f"### idle\n")
                    for rid, data in sorted(baseline.items()):
                        w(f"- report 0x{rid:02x} ({len(data)} bytes): `{data.hex(' ')}`")
                    if not baseline:
                        w("- no reports while idle (the controller only sends on change); "
                          "the first report of each step is used as its baseline")
                    w("")
                    continue
                for _, data in reps:
                    baseline.setdefault(data[0], data)
                rc = describe_report_changes(baseline, reps)
                ec = describe_events(evs)
                w(f"### {label}\n")
                w("- raw: " + ("; ".join(rc) if rc else "no change"))
                w("- evdev: " + (", ".join(ec) if ec else "no events"))
                w("")
                print("  raw:", "; ".join(rc) or "no change")
                print("  evdev:", ", ".join(ec) or "no events")
        w(f"Full timestamped log: `{os.path.basename(base)}.log`.\n")

    with open(base + ".md", "w") as f:
        f.write("\n".join(md) + "\n")
    # Hand the files back to the user who ran sudo.
    uid, gid = int(os.environ.get("SUDO_UID", 0)), int(os.environ.get("SUDO_GID", 0))
    for p in glob.glob(base + ".*"):
        if uid:
            os.chown(p, uid, gid)
    print(f"\nWrote {base}.md (and .bin, .log, .rdesc.txt).")


if __name__ == "__main__":
    main()
