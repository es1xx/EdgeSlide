# Touchpad access

EdgeSlide opens only udev-classified touchpad event devices, read-only, and never
calls EVIOCGRAB or injects input. Normal compositor processing continues. This
means the pointer can move and a tap can still be delivered during an edge slide.
Applications cannot suppress those effects without compositor support or input
interception. Hyprland settings are never rewritten.

On the inspected machine, the ELAN050A:00 04F3:3158 Touchpad is /dev/input/event16.
The desktop user is not in the input group and has no device ACL. A separate
libinput context has the same device-permission requirement; logind TakeDevice
is reserved to the session controller already used by the compositor.

The explicit opt-in helper grants one numeric UID **read-only access to the
exact named touchpad**, using a udev rule and a device ACL. It gives no keyboard,
mouse, backlight, sudo, or general input-group access. The rule persists across
replug/reboot. It does not change compositor input configuration.

Review `scripts/device-access.sh`, then, only if you approve this system change:

```sh
sudo ./scripts/device-access.sh grant "$USER" 'ELAN050A:00 04F3:3158 Touchpad'
systemctl --user restart edgeslide.service
```

Security boundary: any process running as that UID can read this touchpad, even
outside an active session. ACL revocation does not revoke an already-open file
descriptor. EdgeSlide checks logind Active and LockedHint before actions, but
this is application behavior, not a kernel-enforced access boundary. Lock
integration depends on the desktop correctly reporting its lock state.
Do not install this mechanism on a shared machine without accepting that tradeoff.

To revoke, stop EdgeSlide first and run:

```sh
systemctl --user stop edgeslide.service
sudo ./scripts/device-access.sh revoke "$USER" 'ELAN050A:00 04F3:3158 Touchpad'
```

The helper owns `/etc/udev/rules.d/99-edgeslide-touchpad.rules` only. It refuses
to overwrite an existing rule. It does not trigger or reload touchpad drivers.
Brightness uses logind SetBrightness, not writable sysfs permissions.
