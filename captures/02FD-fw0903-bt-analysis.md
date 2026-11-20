# Model 1708, PID 02FD, firmware 0x0903, classic Bluetooth: analysis

Source: `02FD-fw0903-bt.md` / `.log` (full guided run) and
`02FD-fw0903-bt-LStick.*` (left-stick redo), captured 2026-11-20 on umoja,
kernel 7.3.0-9011-generic with hid-xbox bound. Descriptor sha256
`9844a458be45238f8cae2ca9123a6af7be6f39c633f9df184cf8391388d3431a` (334 bytes).

## Input report 0x01 (17 bytes)

| Bytes | Field | Descriptor | Kernel today |
| --- | --- | --- | --- |
| 1-2 | Left stick X | GD X, 0..65535, u16 LE | ABS_X 0..65535 |
| 3-4 | Left stick Y | GD Y, 0..65535 | ABS_Y 0..65535 |
| 5-6 | Right stick X | GD Z, 0..65535 | ABS_Z 0..65535 |
| 7-8 | Right stick Y | GD Rz, 0..65535 | ABS_RZ 0..65535 |
| 9-10 | LT | Sim. Brake, 10 bits + 6 pad | ABS_BRAKE 0..1023 |
| 11-12 | RT | Sim. Accelerator, 10 bits + 6 pad | ABS_GAS 0..1023 |
| 13 low nibble | D-pad | GD Hat 1..8 (0 = null) | ABS_HAT0X/Y |
| 14 | Buttons 1-8 | Button page | see below |
| 15 bits 0-6 | Buttons 9-15 | Button page | see below |
| 16 bit 0 | View | Consumer AC Back (0x0224) | KEY_BACK |

Input report 0x02 byte 1 bit 0: Xbox button, Consumer AC Home (0x0223), giving
KEY_HOMEPAGE. Input report 0x04 byte 1: battery strength 0..255 (the generic
HID battery, about 53% during the capture).

Hat values seen: 1 up, 3 right, 5 down, 7 left.

## Buttons: what each physical control sends

| Control | Usage | Byte/bit | Kernel code |
| --- | --- | --- | --- |
| A | Button 1 | 14/0x01 | BTN_SOUTH |
| B | Button 2 | 14/0x02 | BTN_EAST |
| X | Button 4 | 14/0x08 | BTN_NORTH (= BTN_X) |
| Y | Button 5 | 14/0x10 | BTN_WEST (= BTN_Y) |
| LB | Button 7 | 14/0x40 | BTN_TL |
| RB | Button 8 | 14/0x80 | BTN_TR |
| Menu | Button 12 | 15/0x08 | BTN_START |
| LS click | Button 14 | 15/0x20 | BTN_THUMBL |
| RS click | Button 15 | 15/0x40 | BTN_THUMBR |
| View | Consumer AC Back | 16/0x01 | KEY_BACK |
| Xbox | Consumer AC Home | rpt 2, 1/0x01 | KEY_HOMEPAGE |

Buttons 3, 6, 9, 10, 11 and 13 are declared but never sent, so BTN_C, BTN_Z,
BTN_TL2, BTN_TR2, BTN_SELECT and BTN_MODE are advertised but never emitted.

## Findings for the next phases

1. **Stick ranges are correct, but unsigned.** Both sticks swept 0..65535 in
   full, and the left stick rested at X 35477 / Y 33786. No sign problem on
   this firmware. Getting to the target -32768..32767 is a recentre (a
   descriptor fixup to signed with Logical Min -32768 / Max 32767, plus
   XOR 0x8000 on the data, or a raw_event recentre), not a range repair.
2. **Right stick is on Z/RZ and triggers are on BRAKE/GAS.** The Phase 2.1
   target (sticks on X/Y/RX/RY, triggers on Z/RZ 0..1023) means remapping
   usages, which changes evdev codes that SDL's and Steam's existing mappings
   for this VID/PID rely on. That's a Phase 5 coordination item.
3. **Phantom buttons.** Six advertised button codes never fire, and View/Xbox
   come in as consumer keys instead of BTN_SELECT/BTN_MODE. A fixup that maps
   AC Back to BTN_SELECT and AC Home to BTN_MODE, and drops the unused button
   bits, would give a gamepad.rst-conformant layout. Like 2, it changes codes
   userspace already maps.
4. **X/Y naming.** Physical X (left) emits BTN_NORTH and physical Y (top)
   emits BTN_WEST. That's backwards against Documentation/input/gamepad.rst,
   but it matches xpad's long-standing BTN_X/BTN_Y behaviour, so changing it is
   also a userspace-visible decision.
5. **Force feedback.** Output report 0x03 has an enable nibble, 4 magnitude
   bytes (0..100: left trigger, right trigger, left/strong, right/weak),
   duration, start delay and loop count. The trigger motors are already in
   the descriptor (Phase 3.1).
6. **Classic-BT descriptor fingerprint for 1.2:** 334 bytes, Consumer
   AC Back inside report 1, AC Home alone in report 2, triggers on Simulation
   Controls Brake/Accelerator.
