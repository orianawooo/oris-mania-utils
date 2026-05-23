# Release Checklist

## Baseline

- `npm run check`
- `npm run build:desktop`
- `npm run smoke`

## Functional Smoke

- Launch `oris-mania-utils.exe`
- Confirm bootstrap detects or preserves:
  - `TOSU` root
  - `osu! Songs` folder
- Confirm `Export` still opens a native save dialog
- Confirm `Keystrokes` preview updates and the installed overlay matches source behavior
- Confirm `HitCounter` preview still renders and persists edits
- Confirm `msdconverter` still reads fresh `msd.json`

## Release Artifacts

- Run `npm run release:prep`
- Verify `dist/oris-mania-utils-v<version>.zip`
- Verify the ZIP contains only `oris-mania-utils.exe` and no manual overlay folders
- Verify the latest MSI was copied into `dist/`
- Verify smoke check did not report overlay hash mismatches

## Notes

- The stable bundle path is MSI-first. We intentionally avoid the broken NSIS branch for now.
- The portable ZIP is EXE-only. Overlay sources are bundled into the app and should install into `tosu/static/` from the manager UI.
- If installed overlay sync fails, re-run the manager bootstrap or inspect the configured `tosu_root_path`.
