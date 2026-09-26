# RE guide: keymap calibration + macros (the missing 20%)

## 1. Confirm your VID/PID

```bash
lsusb -v -d 2512: | grep -E "idVendor|idProduct|bInterfaceNumber|iProduct"
```

Add your PID to `CANDIDATE_PIDS` in `crates/mk730-core/src/lib.rs`.

## 2. Capture lighting (verify MK730 == MK750)

- Linux host + Windows VM (Portal v1.01) + Wireshark on `usbmon`.
- Filter: `usb.idVendor == 0x2512`
- Actions: change effect, speed, colors, save profile. Compare vs `proto::hex()` logs in app.
- Save as `captures/mk730-lighting-<pid>.pcapng`.

## 3. Calibrate keymap (record pattern from masterkeys-linux)

1. Set manual mode `41 02`.
2. For each physical key: send `c0 01 <candidate_id> ff 00 00`, photograph which key lights red.
3. Build `id -> label` table, replace `default_mk730_tkl()` IDs.
4. Verify `c0 02` order is ascending 0..N.

## 4. Crack macros (unknown)

Portal does macros/remap, but no public encoding exists.

Capture matrix (one pcapng each, minimal diffs):

1. `macro-create-F5-abc.pcapng`: create macro on F5 typing `a b c`, save.
2. `macro-assign.pcapng`: assign existing macro to another key.
3. `macro-delete.pcapng`: delete it.
4. `remap-capslock.pcapng`: remap Caps -> Esc.
5. `profile-switch.pcapng`: switch P1->P2 (control sample).

Look for 64B OUT to EP 0x04 with new first bytes outside `41/50/51/52/c0`. Likely a new family (e.g. `60/70/...`). File an issue with pcapng + trigger key + steps; then implement `macros::encode_for_firmware()` and flip `fw_state` to `synced`.

## 5. Wireshark cheat-sheet

```
usb.idVendor == 0x2512
usb.transfer_type == 0x01 && usb.endpoint_address.direction == 1
usb.capdata  # leftover bytes = 64B payload
tshark -r in.pcapng -Y usb.capdata -T fields -e usb.capdata
```

See `scripts/capture/filters.txt`.
