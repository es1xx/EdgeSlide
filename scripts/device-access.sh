#!/usr/bin/env bash
# Explicit opt-in only. This script is never called by the user installer.
set -euo pipefail
if [[ $EUID != 0 || $# != 3 || ! $1 =~ ^(grant|revoke)$ ]]; then
 echo 'Usage: sudo scripts/device-access.sh grant|revoke USER "EXACT TOUCHPAD NAME"' >&2; exit 2
fi
action=$1
account=$2
touchpad=$3
uid=$(id -u -- "$account")
if [[ $uid == 0 || ! $touchpad =~ ^[a-zA-Z0-9\ .:_-]+$ ]]; then echo 'Invalid user or touchpad name.' >&2; exit 2; fi
rule=/etc/udev/rules.d/99-edgeslide-touchpad.rules
if [[ $action == grant ]]; then
 if [[ -e $rule ]]; then echo "Refusing to replace existing $rule. Revoke the old rule first." >&2; exit 1; fi
 printf 'ACTION=="add|change", SUBSYSTEM=="input", KERNEL=="event*", ENV{ID_INPUT_TOUCHPAD}=="1", ATTRS{name}=="%s", RUN+="/usr/bin/setfacl -m u:%s:r /dev/input/%%k"\n' "$touchpad" "$uid" > "$rule"
 chmod 644 "$rule"
else
 if [[ -e $rule ]]; then
  expected=$(printf 'ACTION=="add|change", SUBSYSTEM=="input", KERNEL=="event*", ENV{ID_INPUT_TOUCHPAD}=="1", ATTRS{name}=="%s", RUN+="/usr/bin/setfacl -m u:%s:r /dev/input/%%k"' "$touchpad" "$uid")
  if [[ $(cat "$rule") != "$expected" ]]; then echo 'Rule differs from requested user/device; refusing removal.' >&2; exit 1; fi
  rm -- "$rule"
 fi
fi
udevadm control --reload-rules
for name_file in /sys/class/input/event*/device/name; do
 [[ $(cat "$name_file") == "$touchpad" ]] || continue
 device="/dev/input/$(basename "$(dirname "$(dirname "$name_file")")")"
 udevadm info -q property -n "$device" | grep -qx 'ID_INPUT_TOUCHPAD=1' || continue
 if [[ $action == grant ]]; then setfacl -m "u:$uid:r" "$device"; else setfacl -x "u:$uid" "$device"; fi
done
printf '%s\n' "Touchpad access: $action. No keyboard or other input permissions changed."
