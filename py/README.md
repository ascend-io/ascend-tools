# py

Python package for ascend-tools. Contains the pure-Python surface (`__init__.py`, type stubs) that wraps the compiled PyO3 extension module.

The native extension (`core.abi3.so`) is built by maturin from `crates/ascend-tools-py/` and installed into this package at `ascend_tools/core`.

| File | Purpose |
|------|---------|
| `ascend_tools/__init__.py` | Re-exports `Client` from the native module, defines `main()` CLI entry point |
| `ascend_tools/core.pyi` | Type stubs for IDE autocomplete |
| `ascend_tools/py.typed` | PEP 561 marker (package has inline types) |
