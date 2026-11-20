# Model 1708, PID 0B20, firmware 5.23 (5.23.6.0), BLE: analysis and comparison with 02FD

Source: `0B20-fw0523-ble.md` / `.log` (full guided run), captured 2026-11-20
on umoja, kernel 7.3.0-9011-generic, hid-xbox bound. It's the same physical
controller as the `02FD-fw0903-bt` captures, after updating it with the Xbox
Accessories app. Descriptor: 283 bytes, sha256
`a681e2cb8b5d9466f258e2f07f0fcc518ec438231af1b61b125c213d2c62e165`.

## Transport

BLE HID over GATT (service 0x1812), plus Battery Service 0x180F, Device
Information 0x180A and a vendor service 00000001-5f60-4c4f-9c83-a7953298d40d.
The address stays public, C8:3F:26:D7:DD:4B. BlueZ hands the device to the
kernel through uhid as BUS_BLUETOOTH, version 0x0523.

## Descriptor diff against 02FD (334 bytes)

The first 169 bytes are identical: sticks, triggers, hat and the Button 1-15
block. Then:

| | 02FD classic | 0B20 BLE |
| --- | --- | --- |
| Report 1, last bit | Consumer AC Back (0x0224) | Consumer Record (0x00B2) |
| Report 2 | Consumer AC Home, alone | absent |
| Report 3 (FF out) | same | same |
| Report 4 (battery in) | same | declared but never sent |

## Buttons: what changed

| Control | 02FD sends | 0B20 sends |
| --- | --- | --- |
| View | AC Back, KEY_BACK | Button 11 (byte 15/0x04), BTN_SELECT |
| Menu | Button 12, BTN_START | Button 12 (15/0x08), BTN_START |
| Xbox | AC Home (report 2), KEY_HOMEPAGE | Button 13 (15/0x10), BTN_MODE |

A, B, X, Y, LB, RB, LS, RS, the hat, both sticks (0..65535) and both
triggers (Brake/Gas 0..1023) are byte-for-byte the same as on 02FD.

## Findings

1. **The two firmwares differ only in View/Xbox/Share and battery.** On BLE,
   View and Xbox are already gamepad.rst-conformant (BTN_SELECT/BTN_MODE).
   On classic they're consumer keys. A 02FD-only fixup that rewrites AC Back
   to Button 11 and AC Home to Button 13 would make both transports emit
   the same codes. That's the concrete case for 1.2.
2. **A Record usage doesn't mean there's a Share button.** The 1708 has no
   Share button, yet its BLE descriptor declares Consumer Record, so
   KEY_RECORD is advertised and can never fire. For 2.2, gate the Share
   mapping on the PID (Series X|S 0B12/0B13), not on the usage being present,
   or suppress it on 0B20.
3. **No kernel battery on BLE.** Report 0x04 is declared but never arrives
   over HOGP, and no power_supply is created. Battery comes through the GATT
   Battery Service instead: BlueZ Battery1 reports 100% and UPower shows it
   as a gaming-input device. On classic, the kernel's generic HID battery
   (report 4) was the only source. Phase 4's power_supply should therefore
   be transport-specific, classic only, so it doesn't duplicate or conflict
   with BlueZ's BAS battery on BLE.
4. **Descriptor fingerprint for 1.2:** 283 vs 334 bytes; report 1's
   trailing consumer usage is Record vs AC Back; report 2 is absent vs
   present. Any one of these tells the two layouts apart without assuming a
   firmware version.
5. **Binding:** on BLE hid-xbox bound directly (no hid-generic first),
   because the module was already loaded from the classic session.
