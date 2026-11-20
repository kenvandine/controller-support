#!/bin/bash
# Guided capture of an Xbox controller's HID data for hid-xbox development.
#
#   sudo tools/capture-controller.sh [output-dir]
#
# Run with the controller connected over Bluetooth. It walks you through
# pressing buttons and records the raw HID input reports for each step,
# plus the report descriptor, device IDs and evdev capabilities. Every step
# goes to its own file so each report can be matched to the button that
# caused it.
#
# Nothing is installed or changed. hid-recorder and evtest are used if
# they're present; otherwise it falls back to reading /dev/hidrawN directly
# (raw bytes, so report boundaries aren't marked; hid-recorder is better).
#
# The result is a single tarball. Attach it in the project thread, or copy it
# into controller-support/captures/.
set -u

[ "$(id -u)" = 0 ] || { echo "Run with sudo (hidraw and debugfs need root)." >&2; exit 1; }

STEP_SECS=${STEP_SECS:-6}

# --- find the controller --------------------------------------------------
devs=()
for d in /sys/bus/hid/devices/*; do
	id=$(sed -n 's/^HID_ID=//p' "$d/uevent")
	bus=${id%%:*}
	vid=$(echo "$id" | cut -d: -f2)
	# Bluetooth (0005) Microsoft (045E), or anything bound to hid-xbox
	drv=$(basename "$(readlink -f "$d/driver" 2>/dev/null)" 2>/dev/null)
	if [ "$drv" = xbox ] || { [ "$bus" = 00000005 ] && [ "$vid" = 0000045E ]; }; then
		devs+=("$d")
	fi
done
if [ ${#devs[@]} -eq 0 ]; then
	echo "No Xbox controller found. Pair and connect it over Bluetooth first." >&2
	exit 1
fi
if [ ${#devs[@]} -gt 1 ]; then
	echo "Several candidates found:"
	for i in "${!devs[@]}"; do
		echo "  $i) $(basename "${devs[$i]}") $(sed -n 's/^HID_NAME=//p' "${devs[$i]}/uevent")"
	done
	read -r -p "Which one? " pick
	dev=${devs[$pick]}
else
	dev=${devs[0]}
fi

hid=$(basename "$dev")
name=$(sed -n 's/^HID_NAME=//p' "$dev/uevent")
id=$(sed -n 's/^HID_ID=//p' "$dev/uevent")
pid=$(echo "$id" | cut -d: -f3 | sed 's/^0000//')
drv=$(basename "$(readlink -f "$dev/driver" 2>/dev/null)" 2>/dev/null || echo none)
hidraw=$(ls "$dev/hidraw" 2>/dev/null | head -n1)
evnode=$(ls -d "$dev"/input/input*/event* 2>/dev/null | head -n1 | xargs -r basename)

case $pid in
02FD) model="Xbox One S 1708 (classic BT firmware)" ;;
0B20) model="Xbox One S 1708 (BLE firmware)" ;;
0B13) model="Xbox Series X|S 1914" ;;
0B05) model="Elite Series 2 1797 (classic BT firmware)" ;;
0B22) model="Elite Series 2 1797 (BLE firmware)" ;;
02E0) model="8BitDo SN30 Pro+ (Xbox mode)" ;;
*) model="unknown PID $pid" ;;
esac

echo "Controller: $name"
echo "  HID $hid, PID $pid ($model), driver: $drv, ${hidraw:-no hidraw}, ${evnode:-no event node}"
[ "$drv" = xbox ] || echo "  WARNING: not bound to hid-xbox. Is the test kernel running? (uname -r: $(uname -r))"
echo

out=${1:-$PWD/capture-$pid-$(date +%Y%m%d-%H%M%S)}
mkdir -p "$out"

# --- static information ---------------------------------------------------
{
	echo "date: $(date -R)"
	echo "kernel: $(uname -r)"
	echo "hid: $hid"
	echo "name: $name"
	echo "hid_id: $id"
	echo "pid: $pid"
	echo "model_guess: $model"
	echo "driver: $drv"
	echo "hidraw: ${hidraw:-}"
	echo "event: ${evnode:-}"
	cat "$dev/uevent"
	echo "modalias: $(cat "$dev/modalias" 2>/dev/null)"
} >"$out/device.txt"

cp "$dev/report_descriptor" "$out/report_descriptor.bin"
od -An -tx1 -v -w16 "$out/report_descriptor.bin" >"$out/report_descriptor.hex"
[ -r "/sys/kernel/debug/hid/$hid/rdesc" ] && cat "/sys/kernel/debug/hid/$hid/rdesc" >"$out/rdesc-parsed.txt"

# Bluetooth side: firmware revision and transport (classic vs LE)
mac=$(sed -n 's/^HID_UNIQ=//p' "$dev/uevent")
if [ -n "$mac" ]; then
	bluetoothctl info "$mac" >"$out/bluetooth-info.txt" 2>&1
fi
grep -A9 -i "Name=\"$name\"" /proc/bus/input/devices >"$out/proc-input.txt" 2>/dev/null
if command -v evtest >/dev/null && [ -n "$evnode" ]; then
	timeout 2 evtest "/dev/input/$evnode" </dev/null >"$out/evtest-info.txt" 2>&1
fi
dmesg | grep -iE "xbox|$hid|hid-generic|microsoft" | tail -50 >"$out/dmesg.txt"

# --- guided input capture -------------------------------------------------
recorder=""
command -v hid-recorder >/dev/null && recorder=hid-recorder

capture() { # file prompt
	local file=$1
	shift
	echo
	echo ">>> $*"
	read -r -p "    Press Enter, then do it within $STEP_SECS seconds (s = skip): " ans
	[ "$ans" = s ] && { echo "skipped" >"$out/$file.skipped"; return; }
	if [ -n "$recorder" ]; then
		timeout "$STEP_SECS" hid-recorder "/dev/$hidraw" >"$out/$file.hid" 2>/dev/null
	else
		timeout "$STEP_SECS" cat "/dev/$hidraw" | od -An -tx1 -v -w64 >"$out/$file.hex"
	fi
	echo "    saved $file"
}

if [ -z "$hidraw" ]; then
	echo "No hidraw node; only static info was captured."
else
	echo "Now the guided part. Each step records ${STEP_SECS}s of raw reports."
	echo "Keep the controller still unless the step says otherwise."
	capture 00-idle        "Don't touch anything (baseline)"
	capture 01-face        "Press A, B, X, Y once each, in that order"
	capture 02-shoulders   "Press LB, RB, then pull LT fully and release, then RT"
	capture 03-sticks      "Move the LEFT stick to full up, down, left, right, then click it"
	capture 04-rstick      "Move the RIGHT stick to full up, down, left, right, then click it"
	capture 05-dpad        "Press D-pad up, down, left, right"
	capture 06-menu        "Press View, then Menu"
	capture 07-share       "Press the Share button (middle button; skip if your controller has none)"
	capture 08-xbox        "Press the Xbox button briefly (don't hold it)"
	if [ "$pid" = 0B05 ] || [ "$pid" = 0B22 ]; then
		echo
		echo "Elite Series 2: make sure NO profile LED is lit (profile 0) for the next step."
		capture 09-paddles-p0 "Press paddles P1, P2, P3, P4 in order (upper-right, lower-right, upper-left, lower-left)"
		echo
		echo "Now press the profile button until profile 1 is lit."
		capture 10-profile-switch "Press the profile button once to go to profile 1 (captures the switch report)"
		capture 11-paddles-p1 "In profile 1, press P1, P2, P3, P4 in order"
		capture 12-profile-cycle "Cycle through profiles 2, 3 and back to 0 with the profile button"
	fi
	capture 20-battery-idle "Don't touch anything. Plug in a USB cable during this step if you can (charging state)"
fi

tar -C "$(dirname "$out")" -czf "$out.tar.gz" "$(basename "$out")"
echo
echo "Done. Captures are in $out/ and $out.tar.gz"
echo "Attach $out.tar.gz in the project thread, or copy it into controller-support/captures/."
