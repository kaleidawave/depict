## Depict

Depict: deterministic evaluation of performance via instruction counting.

> [!IMPORTANT]
> Currently QBDI only works on ARM MacOs. SDE works on Linux and Windows. Using these requires additional tools or libraries present in paths. [See issues](https://github.com/kaleidawave/depict/issues).

A tool for benchmarking!

Modes:

- SDE (x86 only)
- QBDI

## Required dependencies

You can quickly install required dependencies for instruction counting with

```shell
depict install
```

### QBDI

The tool is compiled. Not sure whether QBDI is required is needed but you can get it on macos here.

```yaml
- name: Get and build QBDI (macos)
  if: ${{ matrix.os == 'macos-latest' }}
  run: |
    curl https://github.com/QBDI/QBDI/releases/download/v0.12.0/QBDI-0.12.0-osx-AARCH64.pkg -L > QBDI.pkg
    sudo installer -pkg QBDI.pkg -target ~
```

Runtime libraries are required **adjacent** to the binary.

On Windows: `libqbdi_tracer.dll` and `QBDIWinPreloader.exe` are required. On MacOS: `libqbdi_tracer.dylib` is required. On Linux `libqbdi_tracer.so` is required. They *should* be present in the releases assets section.

### SDE

This uses an external binary.

You can [download it here](https://www.intel.com/content/www/us/en/download/684897/intel-software-development-emulator.html). `sde` must be under `PATH` or `SDE_PATH`

You can get it on GitHub actions with the following.

```yaml
- name: Setup SDE binaries (linux or windows)
  if: ${{ matrix.os == 'ubuntu-latest' || matrix.os == 'windows-latest' }}
  uses: petarpetrovt/setup-sde@v3.0
  with:
    environmentVariableName: SDE_PATH # default value is `SDE_PATH`
    sdeVersion: 9.58.0 # possible values: 9.58.0 (default), 9.33.0
```

## Notes

- MacOs and Linux Rust builds get debug information. **Windows** release builds needs
  ```toml
  [profile.release-with-debug]
  inherits = "release"
  debug = true
  ```

## TODO

- Find some way to bundle the SDE and QBDI components
- Explain difference between SDE and QBDI
- Add wall-clock and unix-events tools
- Get QBDI working on more platforms / backends
