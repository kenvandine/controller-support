# Xbox Wireless Controller: 02FD-fw0903-bt-LStick

Captured 2026-11-20 10:27 EST on umoja, kernel 7.3.0-9011-generic.

## Device

- HID device: `0005:045E:02FD.0006`, driver **xbox**
- HID_ID `0005:0000045E:000002FD`, PID 02FD, firmware/version 0x0903, transport bt
- MAC/uniq `c8:3f:26:d7:dd:4b`, hidraw hidraw2
- Modules loaded: xpad, hid_xbox, ff_memless
- Report descriptor: 334 bytes, sha256 `9844a458be45238f8cae2ca9123a6af7be6f39c633f9df184cf8391388d3431a` (saved as `02FD-fw0903-bt-LStick.bin`)

```
0000: 05 01 09 05 a1 01 85 01 09 01 a1 00 09 30 09 31
0010: 15 00 27 ff ff 00 00 95 02 75 10 81 02 c0 09 01
0020: a1 00 09 32 09 35 15 00 27 ff ff 00 00 95 02 75
0030: 10 81 02 c0 05 02 09 c5 15 00 26 ff 03 95 01 75
0040: 0a 81 02 15 00 25 00 75 06 95 01 81 03 05 02 09
0050: c4 15 00 26 ff 03 95 01 75 0a 81 02 15 00 25 00
0060: 75 06 95 01 81 03 05 01 09 39 15 01 25 08 35 00
0070: 46 3b 01 66 14 00 75 04 95 01 81 42 75 04 95 01
0080: 15 00 25 00 35 00 45 00 65 00 81 03 05 09 19 01
0090: 29 0f 15 00 25 01 75 01 95 0f 81 02 15 00 25 00
00a0: 75 01 95 01 81 03 05 0c 0a 24 02 15 00 25 01 95
00b0: 01 75 01 81 02 15 00 25 00 75 07 95 01 81 03 05
00c0: 0c 09 01 85 02 a1 01 05 0c 0a 23 02 15 00 25 01
00d0: 95 01 75 01 81 02 15 00 25 00 75 07 95 01 81 03
00e0: c0 05 0f 09 21 85 03 a1 02 09 97 15 00 25 01 75
00f0: 04 95 01 91 02 15 00 25 00 75 04 95 01 91 03 09
0100: 70 15 00 25 64 75 08 95 04 91 02 09 50 66 01 10
0110: 55 0e 15 00 26 ff 00 75 08 95 01 91 02 09 a7 15
0120: 00 26 ff 00 75 08 95 01 91 02 65 00 55 00 09 7c
0130: 15 00 26 ff 00 75 08 95 01 91 02 c0 85 04 05 06
0140: 09 20 15 00 26 ff 00 75 08 95 01 81 02 c0
```

Kernel parse saved as `02FD-fw0903-bt-LStick.rdesc.txt`.

## Input nodes

### event17: Xbox Wireless Controller

- EV `20001b`  KEY `7fff000000000000 0 100040000000 0 0`  ABS `30627`  MSC `10`  FF `107030000 0`
- Keys: KEY_BACK, KEY_HOMEPAGE, BTN_SOUTH, BTN_EAST, BTN_C, BTN_NORTH, BTN_WEST, BTN_Z, BTN_TL, BTN_TR, BTN_TL2, BTN_TR2, BTN_SELECT, BTN_START, BTN_MODE, BTN_THUMBL, BTN_THUMBR
- ABS_X: min 0 max 65535 fuzz 255 flat 4095
- ABS_Y: min 0 max 65535 fuzz 255 flat 4095
- ABS_Z: min 0 max 65535 fuzz 255 flat 4095
- ABS_RZ: min 0 max 65535 fuzz 255 flat 4095
- ABS_GAS: min 0 max 1023 fuzz 3 flat 63
- ABS_BRAKE: min 0 max 1023 fuzz 3 flat 63
- ABS_HAT0X: min -1 max 1 fuzz 0 flat 0
- ABS_HAT0Y: min -1 max 1 fuzz 0 flat 0
- FF codes: 80, 81, 88, 89, 90, 96

## Battery `hid-c8:3f:26:d7:dd:4b-battery-4`

- DEVTYPE=power_supply
- POWER_SUPPLY_NAME=hid-c8:3f:26:d7:dd:4b-battery-4
- POWER_SUPPLY_TYPE=Battery
- POWER_SUPPLY_STATUS=Discharging
- POWER_SUPPLY_PRESENT=1
- POWER_SUPPLY_ONLINE=1
- POWER_SUPPLY_CAPACITY=52
- POWER_SUPPLY_TYPE=Battery
- POWER_SUPPLY_SCOPE=Device
- POWER_SUPPLY_MODEL_NAME=Xbox Wireless Controller

## Guided capture

Raw changes are against the idle baseline; bits are the XOR of every value seen.

### LStick

- raw: report 0x01 byte 1: base 169 -> 0..255 (bits 0xff); report 0x01 byte 15: base 0 -> 32 (bits 0x20); report 0x01 byte 2: base 140 -> 0..255 (bits 0xff); report 0x01 byte 3: base 95 -> 0..255 (bits 0xff); report 0x01 byte 4: base 125 -> 0..255 (bits 0xff)
- evdev: event17 ABS_X 0..65535, event17 ABS_Y 0..65535, event17 BTN_THUMBL 0..1, event17 MSC_SCAN 0x0009000e

Full timestamped log: `02FD-fw0903-bt-LStick.log`.

