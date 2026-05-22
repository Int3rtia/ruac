UAC bypass I wrote for Interium. Uses the CMSTP INF hijack, launches `cmstp.exe /au` with a INF file that run under `RunPreSetupCommands`. cmstp has `autoElevate=true` in its manifest so Windows silently elevates it, no UAC prompt. Payload runs at High IL.

Tested on Windows 10 and Windows 11 25H2 (build 26200.8457). Written in Rust.

Note: CMSTP technique is well-known (been around since 2017) but most public POCs are in C# or PowerShell and leave artifacts everywhere. This one is cleaner.

---

https://github.com/Int3rtia/ruac/raw/main/.media/11%20-%20Trim.mp4
<video src="https://raw.githubusercontent.com/Int3rtia/ruac/main/.media/11%20-%20Trim.mp4" controls autoplay loop muted width="100%">
</video>

Right now it just spawns an elevated `cmd.exe`. Swap out `elevated_payload()` with whatever you actually want to run.

---

## usage

Just run it.

```
uac_bypass.exe
```

If you want to use it as a library pattern, check `elevated_payload()` in `src/main.rs`, that's where the elevated code runs. The re-entry guard is the `/setup` arg check in `main()`.

---

## how the re-entry works

The INF file runs: `"<your exe path>" /setup`

When the elevated instance starts, `main()` checks for `/setup` in args. If found, it skips the bypass and goes straight to `elevated_payload()`. Without this guard it would loop forever trying to bypass itself.

---

## ai disclosure

String obfuscation setup and some of the windows crate API wiring were figured out with AI help. The technique, logic, dialog dismissal, window cleanup, and elevated kill flow are mine. The actual UAC bypass method (CMSTP INF RunPreSetupCommands) is public research.

---

## limitations

- Requires the user account to be in the Administrators group (standard users won't work)
- UAC must not be set to "Always notify" (default settings only)
- CMSTP must be present on the system (it's on all Windows installs by default but can be removed)
- No AV evasion built in. **this is the bypass only**, obfuscation is a separate concern

---

## license

Do whatever. If you use this for something illegal that's on you.
