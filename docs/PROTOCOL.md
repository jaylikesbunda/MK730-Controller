# MK730 protocol (family summary, from libcmmk PROTOCOL.md)

Transport: USB interrupt, Interface 1, OUT endpoint 0x04, IN 0x83. All payloads 64B, zero/ff padded.

- `01 02` + `40 20`: startup handshake (FW ignores if skipped). `05 00` returns ASCII FW version.
- `41 00/01/02/03`: firmware / effect / manual / profile mode. Portal lives in `41 01`, SDK manual uses `41 02`.
- `50 55`: save current profile to flash.
- `51/52` mirror set/get:
  - `00`: active profile `00 00 <id>`
  - `28`: active effect `00 00 <eid>` (00 fully-lit, 01 breathe, 02 cycle, 03 single, 04 wave, 05 ripple, 06 cross, 07 raindrops, 08 stars, 09 snake, 0a customized, e0 multilayer, fe off)
  - `29`: enabled effects list (18x eid, ff-padded)
  - `2c`: `<ml> 00 <eid> <p1 speed 10..50> <p2 dir/mode> <p3 intensity> <c1 rgb> <c2 rgb>` ff-padded. Wave dirs: 00 L→R, 04 R→L, 02 B→F, 06 F→B. Ripple p2 00=color, 80=random.
  - `a8`: custom map, 8× packets `o1=2*i`, 16 RGB triplets each.
  - `a0`: multilayer map, 3 packets `00 07 / 07 07 / 0e 01` + eid list.
- `c0` manual: `00` full `00 00 rgb`, `01` key `01 00 <id> rgb`, `02` colormap (same shape as `51 a8`).

Key IDs are linear 0..N ascending for `a8/c0 02` but physical topology is arbitrary — calibrate per layout (record tool). MK730 adds side/front lightbar zones beyond the 87 TKL keys.

Source: https://github.com/chmod222/libcmmk (archived Feb 2026, LGPL-3.0).
